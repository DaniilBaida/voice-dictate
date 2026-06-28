use std::sync::{Arc, Mutex};

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
