use std::sync::{Arc, Mutex};

/// Which hotkey started the dictation. Captured when recording begins, so the
/// stop press uses the mode the speaker chose at the start.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Paste the transcript exactly as the recogniser returned it.
    Raw,
    /// Restructure the transcript into a prompt before pasting.
    Prompt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Idle,
    Recording,
    Transcribing,
}

#[derive(Clone)]
pub struct AppState(Arc<Mutex<Phase>>);

impl AppState {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(Phase::Idle)))
    }

    #[cfg_attr(not(windows), allow(dead_code))]
    pub fn get(&self) -> Phase {
        *self.0.lock().unwrap()
    }

    /// Transition to `next` only if current phase matches `from`.
    /// Returns true on success.
    pub fn transition(&self, from: Phase, to: Phase) -> bool {
        let mut guard = self.0.lock().unwrap();
        if *guard == from {
            *guard = to;
            true
        } else {
            false
        }
    }

    pub fn set(&self, phase: Phase) {
        *self.0.lock().unwrap() = phase;
    }
}
