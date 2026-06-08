//! `ConversionJob` / `ConversionPlan` types — the fully-resolved batch the
//! encoder executes blindly (ADR-0010), plus the four user options and the
//! three presets (ADR-0007). Part of the frozen IPC contract (ADR-0016, S1):
//! every type here derives `TS` so the TypeScript counterpart is generated,
//! never hand-written.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Resolution knob (ADR-0007). `Original` passes the source size through; the
/// rest cap the frame, aspect-preserving. The encoder maps these to a `scale`
/// filter (B2) — never a hardcoded `-level` (ADR-0006).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "ipc/")]
pub enum Resolution {
    Original,
    P1080,
    P720,
    P480,
}

/// Framerate cap (ADR-0007). `Original` leaves the source rate untouched.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "ipc/")]
pub enum Framerate {
    Original,
    Fps60,
    Fps30,
    Fps24,
}

/// Audio knob (ADR-0007). `None` maps to `-an` (no audio); the bitrates map to
/// `-b:a`. A bitrate on a source without audio is a no-op.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "ipc/")]
pub enum Audio {
    Kbps192,
    Kbps128,
    Kbps96,
    None,
}

/// The four user-facing encode knobs (ADR-0007). CRF is the raw 18–28 value
/// (the UI surfaces 18/23/28; an Advanced stepper is deferred). The
/// PowerPoint-safe flags (ADR-0006) are fixed and not represented here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "ipc/")]
pub struct ConversionOptions {
    pub resolution: Resolution,
    pub crf: u8,
    pub framerate: Framerate,
    pub audio: Audio,
}

impl ConversionOptions {
    /// **Presentation** preset — the tracer's hardcoded values and the default.
    pub const PRESENTATION: Self = Self {
        resolution: Resolution::Original,
        crf: 23,
        framerate: Framerate::Original,
        audio: Audio::Kbps128,
    };

    /// **High Quality** preset (ADR-0007).
    pub const HIGH_QUALITY: Self = Self {
        resolution: Resolution::Original,
        crf: 18,
        framerate: Framerate::Original,
        audio: Audio::Kbps192,
    };

    /// **Compact** preset (ADR-0007).
    pub const COMPACT: Self = Self {
        resolution: Resolution::P720,
        crf: 28,
        framerate: Framerate::Fps30,
        audio: Audio::Kbps96,
    };
}

/// One resolved unit of work (ADR-0010): a source path, its planned `_ppt`
/// output path (ADR-0008), and the four options applied.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "ipc/")]
pub struct ConversionJob {
    pub source_path: String,
    pub output_path: String,
    pub options: ConversionOptions,
}

/// A Conversion Plan is an ordered list of Conversion Jobs (CONTEXT.md). The
/// IPC surface passes it as `ConversionJob[]`; this alias names the concept on
/// the Rust side (`start_conversion` takes `Vec<ConversionJob>`).
pub type ConversionPlan = Vec<ConversionJob>;
