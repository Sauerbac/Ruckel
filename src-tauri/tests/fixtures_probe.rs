//! Every committed fixture in the ADR-0028 corpus must probe successfully —
//! i.e. the pre-flight gate (`has_video`) would let v1 convert it. This is the
//! probe-compatibility half of issue 01; the manifest↔corpus drift test
//! arrives with issue 05.
//!
//! As of issue 03 the probe runs in-process (ADR-0027) — no sidecar — so the
//! `has_video` sweep runs unconditionally. A second test pins parity against the
//! retired ffprobe sidecar (skip-guarded on its presence) so we know the swap
//! reads the same duration and dimensions the old code did.

use std::path::{Path, PathBuf};

use ruckel_lib::{probe, sidecar_command, sidecar_path};

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

#[test]
fn in_process_probe_matches_ffprobe_sidecar() {
    if !sidecar_path("ffprobe").exists() {
        eprintln!("skipping sidecar-parity test: src-tauri/binaries/ is empty");
        return;
    }

    // Dimensions our minimal build can't read because the stream's codec needs
    // a decoder that isn't in the ADR-0028 matrix (the codec carries no size in
    // its container header, so `find_stream_info` would have to decode a frame).
    // This is a build-matrix concern (issue 05's drift test), not a probe defect:
    // `has_video` — the only field pre-flight gates on — still matches. We pin the
    // exact set so a regression in any other codec, or a new gap, fails the test.
    let mut decoder_gap_dims: Vec<String> = Vec::new();

    for path in fixtures() {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let ours = probe::probe(&path)
            .unwrap_or_else(|e| panic!("{name} failed to probe in-process: {e}"));
        let theirs = ffprobe_ground_truth(&path);

        assert_eq!(
            ours.has_video, theirs.has_video,
            "{name}: has_video disagrees (in-process {}, ffprobe {})",
            ours.has_video, theirs.has_video
        );
        assert!(
            (ours.duration_secs - theirs.duration_secs).abs() <= 0.5,
            "{name}: duration disagrees (in-process {:.3}s, ffprobe {:.3}s)",
            ours.duration_secs, theirs.duration_secs
        );

        if (ours.width, ours.height) == (0, 0) && (theirs.width, theirs.height) != (0, 0) {
            decoder_gap_dims.push(name);
            continue;
        }
        assert_eq!(
            (ours.width, ours.height),
            (theirs.width, theirs.height),
            "{name}: dimensions disagree (in-process {}x{}, ffprobe {}x{})",
            ours.width, ours.height, theirs.width, theirs.height
        );
    }

    decoder_gap_dims.sort();
    assert_eq!(
        decoder_gap_dims,
        ["flv1-mp3.flv"],
        "in-process dimension gaps changed. The only expected gap is flv1 — the \
         FLV1/Sorenson decoder is absent from the ADR-0028 matrix (the manifest \
         enables h263, a distinct decoder), so its frame-only dimensions can't be \
         read in-process. Resolve in issue 05 (add the decoder or allowlist it). \
         Got: {decoder_gap_dims:?}"
    );
}

struct Truth {
    has_video: bool,
    width: u32,
    height: u32,
    duration_secs: f64,
}

/// Ground truth from the v1 ffprobe sidecar — the values the old `probe()`
/// would have parsed out of `-show_format -show_streams` JSON.
fn ffprobe_ground_truth(path: &Path) -> Truth {
    let output = sidecar_command("ffprobe")
        .args([
            "-v",
            "quiet",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
        ])
        .arg(path)
        .output()
        .expect("spawn ffprobe sidecar");
    assert!(
        output.status.success(),
        "ffprobe failed for {}: {}",
        path.display(),
        output.status
    );

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("ffprobe emits valid json");

    let video = json["streams"]
        .as_array()
        .and_then(|streams| streams.iter().find(|s| s["codec_type"] == "video"));

    Truth {
        has_video: video.is_some(),
        width: video.and_then(|s| s["width"].as_u64()).unwrap_or(0) as u32,
        height: video.and_then(|s| s["height"].as_u64()).unwrap_or(0) as u32,
        duration_secs: json["format"]["duration"]
            .as_str()
            .and_then(|d| d.parse::<f64>().ok())
            .filter(|d| d.is_finite() && *d >= 0.0)
            .unwrap_or(0.0),
    }
}
