//! Every committed fixture in the ADR-0028 corpus must probe successfully —
//! i.e. the pre-flight gate (`has_video`) would let v1 convert it. This is the
//! probe-compatibility half of issue 01; the manifest↔corpus drift test
//! arrives with issue 05.
//!
//! Skipped when `src-tauri/binaries/` is empty, same as `smoke.rs` (probe is
//! still sidecar-based at this point).

use std::path::PathBuf;

use ruckel_lib::{probe, sidecar_path};

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

#[test]
fn every_committed_fixture_probes_with_a_video_stream() {
    if !(sidecar_path("ffmpeg").exists() && sidecar_path("ffprobe").exists()) {
        eprintln!("skipping fixture probe test: src-tauri/binaries/ is empty");
        return;
    }

    let mut probed = 0;
    for entry in std::fs::read_dir(fixtures_dir()).expect("fixtures dir must exist") {
        let path = entry.expect("read fixtures dir").path();
        // The allowlisted-gaps file is corpus metadata, not a fixture.
        if path.extension().is_some_and(|e| e == "json") {
            continue;
        }

        let result = probe::probe(&path)
            .unwrap_or_else(|e| panic!("{} failed to probe: {e}", path.display()));
        assert!(
            result.has_video,
            "{} has no video stream — pre-flight would reject it",
            path.display()
        );
        probed += 1;
    }

    // Sanity: the corpus is actually there (matrix cells + rotated + noext).
    assert!(probed >= 30, "expected the full corpus, found {probed} fixtures");
}
