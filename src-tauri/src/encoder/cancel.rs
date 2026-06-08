//! Cancellation token / shared state for stopping an in-flight encode
//! (ADR-0013). A cheap, cloneable flag the runner polls between progress
//! reads; on cancel it kills FFmpeg and deletes the temp output. Full
//! cancel UX is wired in I5.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// A shared, cloneable cancellation flag. Clones share one underlying flag, so
/// the Tauri command thread can signal the encode thread.
#[derive(Debug, Clone, Default)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    pub fn new() -> Self {
        Self::default()
    }

    /// Request cancellation of the in-flight batch.
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    /// Clear the flag at the start of a fresh batch.
    pub fn reset(&self) {
        self.0.store(false, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clones_share_one_flag() {
        let a = CancelToken::new();
        let b = a.clone();
        assert!(!b.is_cancelled());
        a.cancel();
        assert!(b.is_cancelled());
        b.reset();
        assert!(!a.is_cancelled());
    }
}
