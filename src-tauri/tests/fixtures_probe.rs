//! Every committed fixture in the ADR-0028 corpus must probe successfully —
//! i.e. the pre-flight gate (`has_video`) lets it through to conversion. This
//! is the probe-compatibility half of issue 01; the manifest↔corpus drift gate
//! lives in `matrix.rs` (issue 05).
//!
//! The probe runs in-process (ADR-0027). While the v1 ffprobe binary was still
//! around, a parity test here pinned the in-process probe's duration and
//! dimensions against its JSON output for the whole corpus; it left with the
//! sidecar retirement (issue 06) after the swap was proven.

use std::path::PathBuf;

use ruckel_lib::probe;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

/// Yield each fixture path, skipping the `allowlisted-gaps.json` corpus metadata.
fn fixtures() -> impl Iterator<Item = PathBuf> {
    std::fs::read_dir(fixtures_dir())
        .expect("fixtures dir must exist")
        .map(|e| e.expect("read fixtures dir").path())
        .filter(|p| p.extension().map_or(true, |e| e != "json"))
}

#[test]
fn every_committed_fixture_probes_with_a_video_stream() {
    let mut probed = 0;
    for path in fixtures() {
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
