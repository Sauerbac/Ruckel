//! Cancellation state for stopping an in-flight encode. Two independent
//! mechanisms (ADR-0013, ADR-0025):
//!
//! - [`CancelToken`] — the whole-batch abort flag (footer Cancel). A cheap,
//!   cloneable bool the runner polls between progress reads.
//! - [`CancelledJobs`] — the per-job cancel set (a row's ✕ during a batch). A
//!   job whose index is in the set is skipped if still queued, or killed and
//!   skipped if active, while the rest of the batch continues.
//!
//! On either, the runner kills FFmpeg and deletes the temp output.

use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

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

/// The set of per-job cancellations for the in-flight batch (ADR-0025). Clones
/// share one underlying set, so the `cancel_job` command thread can mark a job
/// the encode thread is about to (or already does) run. Distinct from
/// [`CancelToken`]: cancelling one job never stops the rest of the batch.
#[derive(Debug, Clone, Default)]
pub struct CancelledJobs(Arc<Mutex<HashSet<u32>>>);

impl CancelledJobs {
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark one job (by its `file_index`) for cancellation.
    pub fn cancel(&self, index: u32) {
        self.0.lock().unwrap().insert(index);
    }

    /// Whether this job has been individually cancelled.
    pub fn contains(&self, index: u32) -> bool {
        self.0.lock().unwrap().contains(&index)
    }

    /// Clear every per-job cancellation at the start of a fresh batch.
    pub fn reset(&self) {
        self.0.lock().unwrap().clear();
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

    #[test]
    fn cancelled_jobs_clones_share_one_set_and_reset_clears() {
        let a = CancelledJobs::new();
        let b = a.clone();
        assert!(!b.contains(2));
        a.cancel(2);
        assert!(b.contains(2));
        assert!(!b.contains(3));
        b.reset();
        assert!(!a.contains(2));
    }
}
