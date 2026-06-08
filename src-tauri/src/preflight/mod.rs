//! Pre-flight: all fast checks (scan, filter, probe, collision detection)
//! that run *before* any encoding, producing a fully-resolved Conversion
//! Plan (ADR-0010).

pub mod collision;
pub mod plan;
pub mod scanner;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use collision::Collision;
use plan::ConversionJob;

/// The output of pre-flight (ADR-0010, ADR-0016): the fully-resolved plan plus
/// any output-path collisions the frontend must resolve before encoding.
/// Part of the frozen IPC contract (S1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "ipc/")]
pub struct PreflightResult {
    pub plan: Vec<ConversionJob>,
    pub collisions: Vec<Collision>,
}
