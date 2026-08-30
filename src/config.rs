use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

fn default_hotkey() -> String {
    "Ctrl+Space".into()
}

fn default_model() -> String {
    "gpt-4o-transcribe".into()
}

fn default_api_base() -> String {
    "https://api.openai.com/v1".into()
}

fn default_history_days() -> u32 {
    7
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    #[serde(default = "default_api_base")]
    pub api_base: String,

    #[serde(default = "default_model")]
    pub model: String,

    #[serde(default)]
    pub language: String,

    #[serde(default)]
    pub prompt: String,

    #[serde(default = "default_hotkey")]
    pub hotkey: String,

    /// 0 = use device native rate
    #[serde(default)]
    pub samplerate: u32,

    #[serde(default = "default_history_days")]
    pub history_retention_days: u32,

    #[serde(default)]
    pub openai_api_key: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_base: default_api_base(),
            model: default_model(),
            language: String::new(),
            prompt: String::new(),
            hotkey: default_hotkey(),
            samplerate: 0,
            history_retention_days: default_history_days(),
            openai_api_key: String::new(),
        }
    }
}

pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("voice-dictate")
}

pub fn config_file() -> PathBuf {
    config_dir().join("config.toml")
}

/// Persist a new hotkey to the config file, replacing the existing `hotkey`
/// line (preserving comments) or appending one if absent.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn save_hotkey(combo: &str) -> std::io::Result<()> {
    let path = config_file();
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let existing = fs::read_to_string(&path).unwrap_or_default();
    let mut out = String::new();
    let mut replaced = false;
    for line in existing.lines() {
        if line.trim_start().starts_with("hotkey")
            && line.split_once('=').map(|(k, _)| k.trim() == "hotkey").unwrap_or(false)
        {
            out.push_str(&format!("hotkey = \"{combo}\""));
            replaced = true;
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }
    if !replaced {
        out.push_str(&format!("hotkey = \"{combo}\"\n"));
    }
    fs::write(path, out)
}

pub fn load() -> Config {
    let path = config_file();
    if !path.exists() {
        return Config::default();
    }
    let text = match fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!("could not read config file: {e}");
            return Config::default();
        }
    };
    match toml::from_str::<Config>(&text) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("config parse error: {e}");
            Config::default()
        }
    }
}

pub fn resolve_api_key(cfg: &Config) -> Option<String> {
    // Env var takes priority
    if let Ok(k) = std::env::var("OPENAI_API_KEY") {
        let k = k.trim().to_string();
        if !k.is_empty() {
            return Some(k);
        }
    }
    // Then config file field
    if !cfg.openai_api_key.is_empty() {
        return Some(cfg.openai_api_key.clone());
    }
    // Legacy ~/.config/openai.key (whisper-dictate convention; home-relative so it works on both platforms)
    let legacy = dirs::home_dir()
        .unwrap_or_default()
        .join(".config")
        .join("openai.key");
    if let Ok(k) = fs::read_to_string(legacy) {
        let k = k.trim().to_string();
        if !k.is_empty() {
            return Some(k);
        }
    }
    None
}
