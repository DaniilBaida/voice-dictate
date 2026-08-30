use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

fn default_hotkey() -> String {
    "Ctrl+Space".into()
}

fn default_paste_shortcut() -> String {
    "Ctrl+V".into()
}

fn default_model() -> String {
    "default".into()
}

fn default_server_url() -> String {
    "http://127.0.0.1:8080/v1".into()
}

fn default_history_days() -> u32 {
    7
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    #[serde(default = "default_server_url")]
    pub server_url: String,

    #[serde(default = "default_model")]
    pub model: String,

    #[serde(default)]
    pub language: String,

    #[serde(default)]
    pub prompt: String,

    #[serde(default = "default_hotkey")]
    pub hotkey: String,

    #[serde(default = "default_paste_shortcut")]
    pub paste_shortcut: String,

    /// 0 = use device native rate
    #[serde(default)]
    pub samplerate: u32,

    #[serde(default = "default_history_days")]
    pub history_retention_days: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server_url: default_server_url(),
            model: default_model(),
            language: String::new(),
            prompt: String::new(),
            hotkey: default_hotkey(),
            paste_shortcut: default_paste_shortcut(),
            samplerate: 0,
            history_retention_days: default_history_days(),
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
    save_shortcut("hotkey", combo)
}

#[cfg_attr(windows, allow(dead_code))]
pub fn save_paste_shortcut(combo: &str) -> std::io::Result<()> {
    save_shortcut("paste_shortcut", combo)
}

fn save_shortcut(name: &str, combo: &str) -> std::io::Result<()> {
    let path = config_file();
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let existing = fs::read_to_string(&path).unwrap_or_default();
    let mut out = String::new();
    let mut replaced = false;
    for line in existing.lines() {
        if line.trim_start().starts_with(name)
            && line
                .split_once('=')
                .map(|(k, _)| k.trim() == name)
                .unwrap_or(false)
        {
            out.push_str(&format!("{name} = \"{combo}\""));
            replaced = true;
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }
    if !replaced {
        out.push_str(&format!("{name} = \"{combo}\"\n"));
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

#[cfg(test)]
mod tests {
    use super::Config;

    #[test]
    fn defaults_to_local_parakeet_service() {
        let config = Config::default();
        assert_eq!(config.server_url, "http://127.0.0.1:8080/v1");
        assert_eq!(config.model, "default");
    }
}
