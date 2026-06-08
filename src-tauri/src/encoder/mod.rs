//! Encoder: takes a Conversion Plan and runs FFmpeg per job, emitting
//! progress events. Does no decision-making (ADR-0010, ADR-0013).
//!
//! The event payloads below and the `events` name constants are the encoder's
//! half of the frozen IPC contract (ADR-0016, S1). They are the single source
//! of truth: the runner emits using the `events::*` constants, and the S1
//! codegen mirrors both the payload shapes (`#[derive(TS)]`) and the name map
//! (`events.ts`) onto the frontend — no magic strings, no hand-written types.

pub mod cancel;
pub mod progress;
pub mod runner;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Event-name constants (ADR-0016). The runner emits with these; the S1
/// codegen emits the matching `events.ts` constant map from the same list, so
/// the stringly-typed names are never duplicated by hand.
pub mod events {
    /// `conversion:progress` → [`super::ProgressEvent`]
    pub const PROGRESS: &str = "conversion:progress";
    /// `conversion:file_done` → [`super::FileDoneEvent`]
    pub const FILE_DONE: &str = "conversion:file_done";
    /// `conversion:file_error` → [`super::FileErrorEvent`]
    pub const FILE_ERROR: &str = "conversion:file_error";
    /// `conversion:done` → [`super::DoneEvent`]
    pub const DONE: &str = "conversion:done";
    /// `conversion:cancelled` → [`super::CancelledEvent`]
    pub const CANCELLED: &str = "conversion:cancelled";

    /// `(TS constant name, event string)` pairs, in contract order. The S1
    /// codegen turns this into the `events.ts` map.
    pub const ALL: &[(&str, &str)] = &[
        ("PROGRESS", PROGRESS),
        ("FILE_DONE", FILE_DONE),
        ("FILE_ERROR", FILE_ERROR),
        ("DONE", DONE),
        ("CANCELLED", CANCELLED),
    ];
}

/// `conversion:progress` — live per-file progress at ~2 Hz (ADR-0013). Percent
/// is `out_time ÷ probed duration`, so it is accurate from the first frame.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export_to = "ipc/")]
pub struct ProgressEvent {
    pub file_index: u32,
    pub total_files: u32,
    pub file_name: String,
    pub percent: f64,
    pub fps: f64,
    pub speed: f64,
}

/// `conversion:file_done` — one job finished; `<stem>_ppt.mp4` now exists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "ipc/")]
pub struct FileDoneEvent {
    pub file_index: u32,
    pub output_path: String,
}

/// `conversion:file_error` — one job failed; the batch continues (ADR-0015).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "ipc/")]
pub struct FileErrorEvent {
    pub file_index: u32,
    pub file_name: String,
    pub error_message: String,
}

/// One failure in the end-of-batch summary (ADR-0015).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "ipc/")]
pub struct ConversionError {
    pub file_name: String,
    pub error_message: String,
}

/// `conversion:done` — the batch finished; carries the skip-and-continue
/// summary (ADR-0015): how many succeeded, how many failed, and every reason.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "ipc/")]
pub struct DoneEvent {
    pub succeeded: u32,
    pub failed: u32,
    pub errors: Vec<ConversionError>,
}

/// `conversion:cancelled` — the batch was cancelled; the in-flight temp file
/// has been cleaned up (ADR-0012, ADR-0013). Carries no data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "ipc/")]
pub struct CancelledEvent {}
