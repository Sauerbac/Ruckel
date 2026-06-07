//! Encoder: takes a Conversion Plan and runs FFmpeg per job, emitting
//! progress events. Does no decision-making (ADR-0010, ADR-0013).

pub mod cancel;
pub mod progress;
pub mod runner;
