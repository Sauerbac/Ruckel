//! Real-FFmpeg smoke test for the tracer's encode path (S2). It synthesizes
//! its own fixture with `ffmpeg -f lavfi` — no media is committed — then runs
//! the real probe + encode and asserts a valid PowerPoint-safe `_ppt.mp4`
//! lands beside the source.
//!
//! Skipped when `src-tauri/binaries/` is empty, so contract-free CI stays green
//! without the ~270 MB sidecars (PRD verification philosophy).

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use ruckel_lib::encoder::runner;
use ruckel_lib::preflight::collision::output_path_for;
use ruckel_lib::preflight::plan::{ConversionJob, ConversionOptions};
use ruckel_lib::{probe, sidecar_command, sidecar_path};

/// Skip-guard: are the fetched sidecars present?
fn sidecars_present() -> bool {
    sidecar_path("ffmpeg").exists() && sidecar_path("ffprobe").exists()
}

/// A unique scratch dir under the OS temp dir, cleaned up on drop.
struct Scratch(PathBuf);
impl Scratch {
    fn new(tag: &str) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("ruckel-smoke-{tag}-{nanos}"));
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
    fn path(&self) -> &Path {
        &self.0
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Synthesize a tiny 2s test clip (video + audio) at `dest` with lavfi.
fn synthesize_fixture(dest: &Path) {
    let status = sidecar_command("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc=duration=2:size=320x240:rate=30",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=2",
            "-shortest",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
        ])
        .arg(dest)
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .status()
        .expect("spawn ffmpeg to build fixture");
    assert!(status.success(), "fixture generation failed");
    assert!(dest.exists(), "fixture not written");
}

/// Read codec_name / pix_fmt / profile of the first video stream, for asserting
/// the PowerPoint-safe flags actually landed (ADR-0006).
fn video_stream_props(file: &Path) -> (String, String, String) {
    let out = sidecar_command("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=codec_name,pix_fmt,profile",
            "-of",
            "default=noprint_wrappers=1",
        ])
        .arg(file)
        .output()
        .expect("spawn ffprobe");
    let text = String::from_utf8_lossy(&out.stdout);
    let field = |key: &str| {
        text.lines()
            .find_map(|l| l.strip_prefix(&format!("{key}=")))
            .unwrap_or("")
            .trim()
            .to_string()
    };
    (field("codec_name"), field("pix_fmt"), field("profile"))
}

#[test]
fn encode_produces_valid_powerpoint_safe_ppt_mp4() {
    if !sidecars_present() {
        eprintln!("skipping smoke test: src-tauri/binaries/ is empty");
        return;
    }

    let scratch = Scratch::new("encode");
    let source = scratch.path().join("clip.mp4");
    synthesize_fixture(&source);

    // Probe drives both the video-stream gate and the progress denominator.
    let probed = probe::probe(&source).expect("probe fixture");
    assert!(probed.has_video, "fixture should have a video stream");
    assert!(
        (probed.duration_secs - 2.0).abs() < 0.5,
        "duration ~2s, got {}",
        probed.duration_secs
    );

    let output = output_path_for(&source);
    assert_eq!(output.file_name().unwrap(), "clip_ppt.mp4");

    let job = ConversionJob {
        source_path: source.to_string_lossy().into_owned(),
        output_path: output.to_string_lossy().into_owned(),
        options: ConversionOptions::PRESENTATION,
    };

    // Capture progress so we prove the percentage path runs end-to-end.
    let updates = RefCell::new(Vec::<f64>::new());
    let written = runner::encode(&job, probed.duration_secs, &|| false, |u| {
        updates.borrow_mut().push(u.percent);
    })
    .expect("encode should succeed");

    assert_eq!(written, output);
    assert!(output.exists(), "_ppt.mp4 should exist");
    assert!(
        std::fs::metadata(&output).unwrap().len() > 0,
        "_ppt.mp4 should be non-empty"
    );
    // The temp must have been promoted by the atomic rename, not left behind
    // (ADR-0012).
    assert!(
        !scratch.path().join("clip_ppt.tmp.mp4").exists(),
        "temp file must be renamed away on success"
    );

    let percents = updates.into_inner();
    assert!(!percents.is_empty(), "progress callback should have fired");
    assert!(
        percents.iter().cloned().fold(0.0_f64, f64::max) > 0.0,
        "progress should advance past 0%"
    );

    // PowerPoint-safe flags actually landed (ADR-0006).
    let (codec, pix_fmt, profile) = video_stream_props(&output);
    assert_eq!(codec, "h264", "must be H.264");
    assert_eq!(pix_fmt, "yuv420p", "must be 8-bit 4:2:0");
    assert_eq!(profile, "High", "must be High profile");

    // Output is itself a valid, probeable video.
    let reprobe = probe::probe(&output).expect("re-probe output");
    assert!(reprobe.has_video);
}

#[test]
fn cancel_leaves_no_output_file() {
    if !sidecars_present() {
        eprintln!("skipping smoke test: src-tauri/binaries/ is empty");
        return;
    }

    let scratch = Scratch::new("cancel");
    let source = scratch.path().join("clip.mp4");
    synthesize_fixture(&source);

    let output = output_path_for(&source);
    let job = ConversionJob {
        source_path: source.to_string_lossy().into_owned(),
        output_path: output.to_string_lossy().into_owned(),
        options: ConversionOptions::PRESENTATION,
    };

    // Always-true predicate: the encode bails at its first progress read and
    // must clean up its partial output (ADR-0012, ADR-0013).
    let result = runner::encode(&job, 2.0, &|| true, |_| {});

    assert!(result.is_err(), "a cancelled encode should not succeed");
    assert!(!output.exists(), "cancel must leave no _ppt.mp4");
    assert!(
        !scratch.path().join("clip_ppt.tmp.mp4").exists(),
        "cancel must leave no temp file (ADR-0012)"
    );
}
