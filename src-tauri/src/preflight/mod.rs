//! Pre-flight: all fast checks (scan, filter, probe, collision detection)
//! that run *before* any encoding, producing a fully-resolved Conversion
//! Plan (ADR-0010).

pub mod collision;
pub mod plan;
pub mod scanner;
