//! Output path generation (`<stem>_ppt.mp4`) and collision detection against
//! existing files on disk (ADR-0008, ADR-0014). The `Collision` type is part
//! of the frozen IPC contract (ADR-0016, S1); full per-file resolution
//! (Override / Rename / Cancel) is wired in I3.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// A planned `_ppt` output path that already exists on disk (ADR-0014).
/// Detected in pre-flight and surfaced in the `PreflightResult` so the
/// frontend can prompt for a resolution before any encode runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "ipc/")]
pub struct Collision {
    /// Index into the plan's job list this collision belongs to.
    pub job_index: u32,
    pub source_path: String,
    /// The existing `_ppt` output path that would be overwritten.
    pub output_path: String,
}

/// The fixed `_ppt` naming contract (ADR-0008): `<source-stem>_ppt.mp4`,
/// written in the source file's own folder. `presentation.avi` →
/// `presentation_ppt.mp4`.
pub fn output_path_for(source: &Path) -> PathBuf {
    let stem = source
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let file_name = format!("{stem}_ppt.mp4");
    match source.parent() {
        Some(dir) => dir.join(file_name),
        None => PathBuf::from(file_name),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appends_ppt_suffix_and_mp4_extension_beside_source() {
        let got = output_path_for(Path::new("/videos/presentation.avi"));
        assert_eq!(got, PathBuf::from("/videos/presentation_ppt.mp4"));
    }

    #[test]
    fn rewrites_any_source_extension_to_mp4() {
        let got = output_path_for(Path::new("/clips/lecture_03.mov"));
        assert_eq!(got, PathBuf::from("/clips/lecture_03_ppt.mp4"));
    }

    #[test]
    fn already_converted_output_gains_a_second_suffix_visibly() {
        // ADR-0008: re-running on x_ppt.mp4 yields x_ppt_ppt.mp4 — visible, not
        // silently destructive.
        let got = output_path_for(Path::new("/v/clip_ppt.mp4"));
        assert_eq!(got, PathBuf::from("/v/clip_ppt_ppt.mp4"));
    }
}
