//! Pre-flight: all fast checks (scan, filter, probe, collision detection)
//! that run *before* any encoding, producing a fully-resolved Conversion
//! Plan (ADR-0010).

pub mod collision;
pub mod plan;
pub mod scanner;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use plan::ConversionJob;

/// The output of pre-flight (ADR-0010, ADR-0016): the fully-resolved plan and
/// per-file probe metadata (duration + resolution for the frontend's phased
/// reveal, ADR-0020). Part of the frozen IPC contract (S1).
///
/// Collision detection no longer lives here (ADR-0025): whether a planned output
/// already exists is checked fresh at convert time via `check_collisions`, not
/// cached at drop time where it goes stale the instant the batch writes outputs.
///
/// `skipped` is the count of scanned candidates that pre-flight discarded —
/// no video stream or unreadable (ADR-0026). One stray file no longer aborts the
/// whole drop; the good files still become jobs. The frontend can't derive this
/// (it drops *paths*, but a folder expands into candidates it never sees), so the
/// backend reports it: `skipped == scanned candidates − plan.len()`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "ipc/")]
pub struct PreflightResult {
    pub plan: Vec<ConversionJob>,
    pub files: Vec<FileProbe>,
    pub skipped: u32,
}

/// Per-file probe metadata surfaced in the pre-flight result (ADR-0020, S1
/// amendment). 1:1 with `plan` by position — the frontend iterates both
/// together. Kept separate from `ConversionJob` so the encoder input stays
/// clean (the encoder re-probes duration and only needs the job fields).
///
/// `size_bytes` is the source file's on-disk size — filesystem metadata, not a
/// probe field — carried here so the frontend's phased reveal (ADR-0020) can
/// show size from the first frame without a separate IPC round-trip. It is an
/// `f64` (not `u64`) so the generated TS stays a plain `number`; exact for any
/// real video size.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "ipc/")]
pub struct FileProbe {
    pub duration_secs: f64,
    pub width: u32,
    pub height: u32,
    pub size_bytes: f64,
}
