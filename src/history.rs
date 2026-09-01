//! Append-only record of what was dictated, one JSON object per line.
//!
//! Prompt mode keeps both texts: the transcript is what the recogniser heard,
//! the prompt is what actually reached the clipboard, and only the pair shows
//! whether a rewrite went wrong.

use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    /// Seconds since the Unix epoch.
    pub at: u64,
    pub mode: String,
    pub transcript: String,
    /// Absent in raw mode, where the transcript is what was pasted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
}

impl Entry {
    /// What the dictation put on the clipboard.
    pub fn pasted(&self) -> &str {
        self.prompt.as_deref().unwrap_or(&self.transcript)
    }
}

pub fn data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("voice-dictate")
}

pub fn file() -> PathBuf {
    data_dir().join("history.jsonl")
}

pub fn record(mode: &str, transcript: &str, prompt: Option<&str>) {
    let entry = Entry {
        at: now(),
        mode: mode.to_string(),
        transcript: transcript.to_string(),
        prompt: prompt.map(str::to_string),
    };
    if let Err(e) = append(&entry) {
        tracing::warn!("history: {e}");
    }
}

fn append(entry: &Entry) -> anyhow::Result<()> {
    let path = file();
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let mut line = serde_json::to_string(entry)?;
    line.push('\n');
    let mut handle = fs::OpenOptions::new().create(true).append(true).open(&path)?;
    handle.write_all(line.as_bytes())?;
    Ok(())
}

/// Newest first. A malformed line is skipped rather than losing the whole file.
pub fn load() -> Vec<Entry> {
    let Ok(text) = fs::read_to_string(file()) else {
        return Vec::new();
    };
    let mut entries: Vec<Entry> = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();
    entries.reverse();
    entries
}

pub fn clear() -> std::io::Result<()> {
    match fs::remove_file(file()) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

/// Drops entries older than the retention window. Runs at startup, so the file
/// is trimmed once per session rather than on every dictation.
pub fn prune(retention_days: u32) {
    if retention_days == 0 {
        return;
    }
    let cutoff = now().saturating_sub(u64::from(retention_days) * 86_400);
    let kept: Vec<Entry> = load().into_iter().filter(|e| e.at >= cutoff).collect();

    let mut text = String::new();
    for entry in kept.iter().rev() {
        match serde_json::to_string(entry) {
            Ok(line) => {
                text.push_str(&line);
                text.push('\n');
            }
            Err(e) => tracing::warn!("history: {e}"),
        }
    }
    if let Err(e) = fs::write(file(), text) {
        if e.kind() != std::io::ErrorKind::NotFound {
            tracing::warn!("history: {e}");
        }
    }
}

/// Single-line preview for a menu entry.
pub fn preview(text: &str, max_chars: usize) -> String {
    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= max_chars {
        return flat;
    }
    let cut: String = flat.chars().take(max_chars.saturating_sub(1)).collect();
    format!("{}…", cut.trim_end())
}

/// Coarse age label, so the menu needs no date formatting dependency.
pub fn age(at: u64) -> String {
    let elapsed = now().saturating_sub(at);
    match elapsed {
        0..=59 => "just now".to_string(),
        60..=3599 => format!("{}m ago", elapsed / 60),
        3600..=86_399 => format!("{}h ago", elapsed / 3600),
        _ => format!("{}d ago", elapsed / 86_400),
    }
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pasted_prefers_the_prompt() {
        let raw = Entry { at: 0, mode: "raw".into(), transcript: "hello".into(), prompt: None };
        assert_eq!(raw.pasted(), "hello");

        let prompt = Entry {
            at: 0,
            mode: "prompt".into(),
            transcript: "hello".into(),
            prompt: Some("Hello.".into()),
        };
        assert_eq!(prompt.pasted(), "Hello.");
    }

    #[test]
    fn preview_collapses_whitespace_and_truncates() {
        assert_eq!(preview("one   two\nthree", 40), "one two three");
        assert_eq!(preview("abcdefghij", 5), "abcd…");
    }

    #[test]
    fn age_reads_in_the_largest_whole_unit() {
        let t = now();
        assert_eq!(age(t), "just now");
        assert_eq!(age(t - 120), "2m ago");
        assert_eq!(age(t - 7200), "2h ago");
        assert_eq!(age(t - 172_800), "2d ago");
    }
}
