//! Issue 04 acceptance: the in-process transcode loop (ADR-0027) produces
//! PowerPoint-safe mp4s and honours the four user options, cancellation, and the
//! ADR-0015 failure surface — verified by encoding the committed h264 fixture and
//! inspecting the output in-process. The full per-fixture matrix sweep + manifest
//! drift test belong to issue 05; this file pins the loop's behaviour on one
//! known-good fixture and the option/cancel/error paths.

// The native x264/dav1d/bcrypt link directives ride the ruckel_lib target; this
// test opens its outputs with rsmpeg directly, so it must pull in the lib (the
// tracer gate uses the same pattern).
extern crate ruckel_lib;

use std::ffi::CString;
use std::path::{Path, PathBuf};

use rsmpeg::avformat::AVFormatContextInput;
use rsmpeg::ffi;

use ruckel_lib::encoder::runner::{encode, EncodeError};
use ruckel_lib::preflight::plan::{Audio, ConversionJob, ConversionOptions, Framerate, Resolution};
use ruckel_lib::probe;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn out_path(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(name);
    let _ = std::fs::remove_file(&p);
    // Also clear any stale temp sibling from a previous aborted run.
    let _ = std::fs::remove_file(temp_sibling(&p));
    p
}

/// The `<stem>.tmp.mp4` the encoder writes before its atomic rename (ADR-0012).
fn temp_sibling(final_path: &Path) -> PathBuf {
    let stem = final_path.file_stem().unwrap().to_string_lossy().into_owned();
    final_path
        .parent()
        .unwrap()
        .join(format!("{stem}.tmp.mp4"))
}

/// Encode a fixture with the given options to a temp output, asserting success.
fn convert(fixture_name: &str, out_name: &str, options: ConversionOptions) -> PathBuf {
    let src = fixture(fixture_name);
    let out = out_path(out_name);
    let job = ConversionJob {
        source_path: src.to_string_lossy().into_owned(),
        output_path: out.to_string_lossy().into_owned(),
        options,
    };
    let duration = probe::probe(&src).unwrap().duration_secs;
    let result = encode(&job, duration, &|| false, |_| {});
    assert!(result.is_ok(), "encode failed: {result:?}");
    out
}

fn open(path: &Path) -> AVFormatContextInput {
    let c = CString::new(path.to_str().unwrap()).unwrap();
    AVFormatContextInput::open(&c).expect("output must open in-process")
}

fn video_stream<'a>(ctx: &'a AVFormatContextInput) -> &'a rsmpeg::avformat::AVStreamRef<'a> {
    ctx.streams()
        .iter()
        .find(|s| s.codecpar().codec_type().is_video())
        .expect("output has a video stream")
}

fn has_audio(ctx: &AVFormatContextInput) -> bool {
    ctx.streams()
        .iter()
        .any(|s| s.codecpar().codec_type().is_audio())
}

/// `moov` before `mdat` in the byte stream — the structural proof of
/// `+faststart` (the v1 suite never actually checked this).
fn moov_precedes_mdat(path: &Path) -> bool {
    let bytes = std::fs::read(path).unwrap();
    let find = |needle: &[u8]| bytes.windows(needle.len()).position(|w| w == needle);
    match (find(b"moov"), find(b"mdat")) {
        (Some(moov), Some(mdat)) => moov < mdat,
        _ => false,
    }
}

#[test]
fn h264_fixture_transcodes_to_the_powerpoint_safe_contract() {
    let out = convert("h264-aac.mp4", "ruckel-it-contract_ppt.mp4", ConversionOptions::PRESENTATION);
    let ctx = open(&out);

    let v = video_stream(&ctx);
    let par = v.codecpar();
    assert_eq!(par.codec_id, ffi::AV_CODEC_ID_H264, "video must be h264");
    assert_eq!(par.profile, ffi::AV_PROFILE_H264_HIGH as i32, "must be h264 high");
    assert_eq!(par.format, ffi::AV_PIX_FMT_YUV420P, "must be yuv420p");
    assert!(par.width % 2 == 0 && par.height % 2 == 0, "even dimensions");

    assert!(has_audio(&ctx), "PRESENTATION keeps audio");
    let a = ctx
        .streams()
        .iter()
        .find(|s| s.codecpar().codec_type().is_audio())
        .unwrap();
    assert_eq!(a.codecpar().codec_id, ffi::AV_CODEC_ID_AAC, "audio must be aac");

    assert!(ctx.duration > 0, "output has a duration");
    assert!(moov_precedes_mdat(&out), "+faststart: moov must precede mdat");
}

#[test]
fn resolution_cap_yields_even_aspect_preserved_height() {
    let out = convert(
        "h264-aac.mp4",
        "ruckel-it-res_ppt.mp4",
        ConversionOptions { resolution: Resolution::P480, ..ConversionOptions::PRESENTATION },
    );
    let ctx = open(&out);
    let par = video_stream(&ctx).codecpar();
    assert_eq!(par.height, 480, "height capped to the target");
    assert_eq!(par.width % 2, 0, "width stays even for yuv420p");
}

#[test]
fn fps_cap_yields_constant_frame_rate_at_the_cap() {
    let out = convert(
        "h264-aac.mp4",
        "ruckel-it-fps_ppt.mp4",
        ConversionOptions { framerate: Framerate::Fps24, ..ConversionOptions::PRESENTATION },
    );
    let ctx = open(&out);
    // r_frame_rate is the base CFR rate (robust on a sub-second clip, unlike
    // avg_frame_rate which is frames÷duration); the fps filter pins it to the cap.
    let rate = video_stream(&ctx).r_frame_rate;
    let fps = rate.num as f64 / rate.den as f64;
    assert!((fps - 24.0).abs() < 0.5, "expected 24 fps CFR, got {fps}");
}

#[test]
fn rotated_fixture_transcodes_through_the_autorotate_path() {
    // The display-matrix fixture must convert cleanly with autorotation applied
    // (the transpose filter runs). The committed fixture is 64×64 (square), so a
    // dimension-swap assertion isn't meaningful here — issue 05 owns physical-
    // rotation verification with a non-square sample. This pins that the rotate
    // path produces a valid, contract-compliant output rather than erroring.
    let out = convert("h264-aac-rotated.mp4", "ruckel-it-rot_ppt.mp4", ConversionOptions::PRESENTATION);
    let ctx = open(&out);
    let par = video_stream(&ctx).codecpar();
    assert_eq!(par.codec_id, ffi::AV_CODEC_ID_H264);
    assert_eq!(par.format, ffi::AV_PIX_FMT_YUV420P);
    assert!(ctx.duration > 0);
}

#[test]
fn audio_none_drops_the_audio_stream() {
    let out = convert(
        "h264-aac.mp4",
        "ruckel-it-noaudio_ppt.mp4",
        ConversionOptions { audio: Audio::None, ..ConversionOptions::PRESENTATION },
    );
    let ctx = open(&out);
    assert!(!has_audio(&ctx), "Audio::None must leave no audio stream");
}

#[test]
fn cancel_leaves_no_output_and_no_temp() {
    let src = fixture("h264-aac.mp4");
    let out = out_path("ruckel-it-cancel_ppt.mp4");
    let job = ConversionJob {
        source_path: src.to_string_lossy().into_owned(),
        output_path: out.to_string_lossy().into_owned(),
        options: ConversionOptions::PRESENTATION,
    };
    let duration = probe::probe(&src).unwrap().duration_secs;

    let result = encode(&job, duration, &|| true, |_| {});
    assert_eq!(result, Err(EncodeError::Cancelled));
    assert!(!out.exists(), "no _ppt output on cancel");
    assert!(!temp_sibling(&out).exists(), "temp file cleaned up on cancel");
}

#[test]
fn corrupt_input_fails_the_job_without_output() {
    let bad = std::env::temp_dir().join("ruckel-it-corrupt-input.mp4");
    std::fs::write(&bad, b"this is not a video file").unwrap();
    let out = out_path("ruckel-it-corrupt_ppt.mp4");
    let job = ConversionJob {
        source_path: bad.to_string_lossy().into_owned(),
        output_path: out.to_string_lossy().into_owned(),
        options: ConversionOptions::PRESENTATION,
    };

    let result = encode(&job, 0.0, &|| false, |_| {});
    assert!(matches!(result, Err(EncodeError::Failed(_))), "got {result:?}");
    assert!(!out.exists(), "no output on failure");
    assert!(!temp_sibling(&out).exists(), "temp file cleaned up on failure");
}
