//! Issue 05: the ADR-0028 matrix made mechanically enforceable.
//!
//! Two layers:
//!   1. **Drift gate** — parse the committed build manifest's enabled decoders +
//!      demuxers and assert the corpus covers each (or an allowlisted gap), AND
//!      that no fixture is stranded outside the matrix. Manifest and corpus can
//!      never silently diverge.
//!   2. **Per-fixture transcode** — every fixture is probed + transcoded
//!      in-process and the output checked structurally (h264 high / yuv420p /
//!      even dims / aac when the source has audio / sane duration / moov-before-
//!      mdat faststart), plus the rotated fixture's swapped dimensions.
//!
//! No child processes, no network — all in-process against the self-built
//! static FFmpeg.

// Native link directives ride ruckel_lib; this test opens outputs with rsmpeg.
extern crate ruckel_lib;

use std::collections::BTreeSet;
use std::ffi::CString;
use std::path::{Path, PathBuf};

use rsmpeg::avformat::AVFormatContextInput;
use rsmpeg::ffi;
use serde_json::Value;

use ruckel_lib::encoder::runner::encode;
use ruckel_lib::preflight::plan::{ConversionJob, ConversionOptions};
use ruckel_lib::probe;

fn manifest_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("ffmpeg-build-manifest.json")
}

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures")
}

// ---- manifest parsing -------------------------------------------------------

fn manifest() -> Value {
    let text = std::fs::read_to_string(manifest_path()).expect("read manifest");
    serde_json::from_str(&text).expect("manifest is valid json")
}

/// The comma-separated values of the `--<prefix>=...` ffmpeg configure flag.
fn enabled(m: &Value, flag_prefix: &str) -> BTreeSet<String> {
    m["configure"]["ffmpeg"]
        .as_array()
        .expect("configure.ffmpeg array")
        .iter()
        .filter_map(|v| v.as_str())
        .find(|s| s.starts_with(flag_prefix))
        .map(|s| s[flag_prefix.len()..].split(',').map(str::to_string).collect())
        .unwrap_or_default()
}

fn allowlisted_decoders() -> BTreeSet<String> {
    let text = std::fs::read_to_string(fixtures_dir().join("allowlisted-gaps.json"))
        .expect("read allowlisted-gaps.json");
    let json: Value = serde_json::from_str(&text).expect("allowlist is valid json");
    json["gaps"]
        .as_array()
        .expect("gaps array")
        .iter()
        .filter_map(|g| g["decoder"].as_str())
        .map(str::to_string)
        .collect()
}

// ---- name maps (decoder name vs fixture codec_name spelling) ----------------

/// The fixture codec token a decoder covers (identity except where FFmpeg's
/// decoder name differs from the `codec_name` spelling the issue 01 fixture
/// filenames use).
fn decoder_to_codec(decoder: &str) -> &str {
    match decoder {
        "libdav1d" => "av1",
        "flv" => "flv1",
        other => other,
    }
}

/// The decoder a fixture codec token needs (inverse of [`decoder_to_codec`]).
fn codec_to_decoder(codec: &str) -> &str {
    match codec {
        "av1" => "libdav1d",
        "flv1" => "flv",
        other => other,
    }
}

/// The manifest demuxer token a container extension is read by.
fn ext_to_demuxer(ext: &str) -> Option<&'static str> {
    Some(match ext {
        "mp4" | "mov" | "3gp" | "m4v" | "m4a" => "mov",
        "mkv" | "webm" => "matroska",
        "avi" => "avi",
        "wmv" | "asf" => "asf",
        "flv" => "flv",
        "ts" => "mpegts",
        "vob" => "mpegps",
        "m1v" | "mpg" | "mpeg" => "mpegvideo",
        "mxf" => "mxf",
        _ => return None,
    })
}

// ---- corpus parsing (coverage from filenames, issue 01 convention) ----------

/// Fixture file names, excluding corpus metadata (`*.json`).
fn fixture_files() -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(fixtures_dir())
        .expect("fixtures dir")
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .filter(|n| !n.ends_with(".json"))
        .collect();
    names.sort();
    names
}

/// `<vcodec>-<acodec>.<container>` (or `<vcodec>.<container>`), ignoring the
/// `rotated`/`noext` derived-fixture suffixes.
fn codec_tokens(files: &[String]) -> BTreeSet<String> {
    let mut tokens = BTreeSet::new();
    for name in files {
        let stem = name.rsplit_once('.').map_or(name.as_str(), |(s, _)| s);
        let parts: Vec<&str> = stem.split('-').collect();
        tokens.insert(parts[0].to_string());
        if let Some(&a) = parts.get(1) {
            if a != "rotated" && a != "noext" {
                tokens.insert(a.to_string());
            }
        }
    }
    tokens
}

/// The container extension of each fixture that has one (the extensionless
/// ADR-0011 fixture yields `None`).
fn container_exts(files: &[String]) -> BTreeSet<String> {
    files
        .iter()
        .filter_map(|n| n.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()))
        .collect()
}

// ---- drift gate (pure core + real-data tests + deliberate red runs) ---------

/// Enabled decoders with neither a corpus fixture nor an allowlist entry.
fn uncovered_decoders(
    decoders: &BTreeSet<String>,
    tokens: &BTreeSet<String>,
    allow: &BTreeSet<String>,
) -> Vec<String> {
    decoders
        .iter()
        .filter(|d| !tokens.contains(decoder_to_codec(d)) && !allow.contains(*d))
        .cloned()
        .collect()
}

/// Fixture codec tokens whose decoder is not enabled (and not allowlisted) —
/// the "stranded fixture" direction. This is exactly the check that caught
/// flv1: the corpus had flv1-mp3.flv while the manifest enabled `h263` but not
/// the distinct `flv` decoder (issue 05 arbiter → `flv` added + lib rebuild).
fn stranded_codecs(
    decoders: &BTreeSet<String>,
    tokens: &BTreeSet<String>,
    allow: &BTreeSet<String>,
) -> Vec<String> {
    tokens
        .iter()
        .filter(|t| {
            let d = codec_to_decoder(t);
            !decoders.contains(d) && !allow.contains(*t) && !allow.contains(d)
        })
        .cloned()
        .collect()
}

fn set(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|s| s.to_string()).collect()
}

#[test]
fn every_enabled_decoder_has_a_fixture_or_allowlist_entry() {
    let m = manifest();
    let uncovered = uncovered_decoders(
        &enabled(&m, "--enable-decoder="),
        &codec_tokens(&fixture_files()),
        &allowlisted_decoders(),
    );
    assert!(
        uncovered.is_empty(),
        "enabled decoders with neither a corpus fixture nor an allowlisted-gaps \
         entry: {uncovered:?} — add fixtures/gaps or remove the decoders"
    );
}

#[test]
fn every_enabled_demuxer_has_a_fixture() {
    let m = manifest();
    let demuxers = enabled(&m, "--enable-demuxer=");
    let covered: BTreeSet<&str> = container_exts(&fixture_files())
        .iter()
        .filter_map(|e| ext_to_demuxer(e))
        .collect();

    for dm in &demuxers {
        assert!(
            covered.contains(dm.as_str()),
            "enabled demuxer `{dm}` has no corpus fixture whose container maps to it"
        );
    }
}

#[test]
fn no_fixture_is_stranded_outside_the_matrix() {
    let m = manifest();
    let stranded = stranded_codecs(
        &enabled(&m, "--enable-decoder="),
        &codec_tokens(&fixture_files()),
        &allowlisted_decoders(),
    );
    assert!(
        stranded.is_empty(),
        "fixture codecs whose decoder is not in the matrix: {stranded:?} — \
         add the decoders (manifest edit + lib rebuild) or drop the fixtures"
    );

    // Every fixture container must map to an enabled demuxer.
    let demuxers = enabled(&m, "--enable-demuxer=");
    for ext in container_exts(&fixture_files()) {
        let dm = ext_to_demuxer(&ext)
            .unwrap_or_else(|| panic!("fixture extension `{ext}` has no known demuxer mapping"));
        assert!(
            demuxers.contains(dm),
            "fixture container `.{ext}` (demuxer `{dm}`) is not an enabled demuxer"
        );
    }
}

/// The *runtime* lib must have been configured with exactly the manifest's
/// decoder/demuxer/filter enumerations. The stamp guard checks the libs *on
/// disk*; this checks the libs *linked into this binary* — catching any path
/// where a stale build sneaks behind a fresh stamp (which happened once: a
/// build-script rename bug kept old libs while restamping).
#[test]
fn runtime_lib_matches_manifest_configuration() {
    // FFmpeg's configure echo wraps list values in single quotes
    // (--enable-decoder='h264,...'); the manifest stores them bare — strip the
    // quotes so the contains() comparison sees the same spelling.
    let cfg = unsafe { std::ffi::CStr::from_ptr(ffi::avcodec_configuration()) }
        .to_string_lossy()
        .replace('\'', "");
    let m = manifest();
    for flag in m["configure"]["ffmpeg"].as_array().expect("configure.ffmpeg array") {
        let flag = flag.as_str().unwrap();
        if flag.starts_with("--enable-decoder=")
            || flag.starts_with("--enable-demuxer=")
            || flag.starts_with("--enable-parser=")
            || flag.starts_with("--enable-filter=")
        {
            assert!(
                cfg.contains(flag),
                "the linked libavcodec was not built from the committed manifest.\n\
                 manifest flag: {flag}\n\
                 runtime configuration: {cfg}"
            );
        }
    }
}

// The deliberate red runs (issue 05 acceptance): prove the gate actually trips,
// without editing the committed manifest (any edit invalidates the lib stamp).

#[test]
fn drift_gate_trips_on_a_decoder_without_fixture_or_gap() {
    // `hevc` enabled, but the corpus only covers h264 and nothing is allowlisted.
    let uncovered = uncovered_decoders(
        &set(&["h264", "hevc"]),
        &set(&["h264", "aac"]),
        &set(&[]),
    );
    assert_eq!(uncovered, vec!["hevc".to_string()]);
}

#[test]
fn drift_gate_trips_on_a_stranded_fixture() {
    // The historical flv1 case, reproduced synthetically: corpus has flv1 but
    // the matrix only enables h263 — the gate must name flv1 as stranded.
    let stranded = stranded_codecs(
        &set(&["h264", "h263", "aac", "mp3"]),
        &set(&["h264", "aac", "flv1", "mp3"]),
        &set(&["vc1", "wmapro"]),
    );
    assert_eq!(stranded, vec!["flv1".to_string()]);
}

// ---- per-fixture transcode --------------------------------------------------

fn open(path: &Path) -> AVFormatContextInput {
    let c = CString::new(path.to_str().unwrap()).unwrap();
    AVFormatContextInput::open(&c).expect("output must open in-process")
}

fn first_video(ctx: &AVFormatContextInput) -> usize {
    ctx.streams()
        .iter()
        .position(|s| s.codecpar().codec_type().is_video())
        .expect("has a video stream")
}

fn has_audio(ctx: &AVFormatContextInput) -> bool {
    ctx.streams().iter().any(|s| s.codecpar().codec_type().is_audio())
}

fn moov_precedes_mdat(path: &Path) -> bool {
    let bytes = std::fs::read(path).unwrap();
    let find = |needle: &[u8]| bytes.windows(needle.len()).position(|w| w == needle);
    matches!((find(b"moov"), find(b"mdat")), (Some(m), Some(d)) if m < d)
}

fn transcode(src: &Path, tag: &str) -> PathBuf {
    let out = std::env::temp_dir().join(format!("ruckel-matrix-{tag}.mp4"));
    let _ = std::fs::remove_file(&out);
    let duration = probe::probe(src).expect("probe source").duration_secs;
    let job = ConversionJob {
        source_path: src.to_string_lossy().into_owned(),
        output_path: out.to_string_lossy().into_owned(),
        options: ConversionOptions::PRESENTATION,
    };
    encode(&job, duration, &|| false, |_| {})
        .unwrap_or_else(|e| panic!("transcode {} failed: {e}", src.display()));
    out
}

#[test]
fn every_fixture_transcodes_to_a_structurally_valid_mp4() {
    for file in fixture_files() {
        let src = fixtures_dir().join(&file);
        let tag = file.replace('.', "_");

        // Source facts we need to assert parity against.
        let src_ctx = open(&src);
        let src_has_audio = has_audio(&src_ctx);
        let src_duration = probe::probe(&src).expect("probe source").duration_secs;
        drop(src_ctx);

        let out = transcode(&src, &tag);

        // Copy the scalars out so the context's borrows end before we drop it.
        let ctx = open(&out);
        let vi = first_video(&ctx);
        let (codec_id, profile, format, w, h) = {
            let par = ctx.streams()[vi].codecpar();
            (par.codec_id, par.profile, par.format, par.width, par.height)
        };
        let out_audio_codec = ctx
            .streams()
            .iter()
            .find(|s| s.codecpar().codec_type().is_audio())
            .map(|a| a.codecpar().codec_id);
        let out_duration = ctx.duration as f64 / ffi::AV_TIME_BASE as f64;
        drop(ctx);

        assert_eq!(codec_id, ffi::AV_CODEC_ID_H264, "{file}: video must be h264");
        assert_eq!(profile, ffi::AV_PROFILE_H264_HIGH as i32, "{file}: h264 high");
        assert_eq!(format, ffi::AV_PIX_FMT_YUV420P, "{file}: yuv420p");
        assert!(w % 2 == 0 && h % 2 == 0, "{file}: even dims ({w}x{h})");

        if src_has_audio {
            assert_eq!(
                out_audio_codec,
                Some(ffi::AV_CODEC_ID_AAC),
                "{file}: source had audio, output must carry aac"
            );
        }

        // Duration within tolerance. Raw elementary streams (mpegvideo) report
        // no real duration — just a near-zero bitrate estimate — so treat
        // anything under 0.2s as unknown and skip the comparison.
        if src_duration > 0.2 {
            assert!(
                (out_duration - src_duration).abs() < 0.5,
                "{file}: duration {out_duration:.3}s vs source {src_duration:.3}s"
            );
        }

        assert!(moov_precedes_mdat(&out), "{file}: +faststart (moov before mdat)");
        let _ = std::fs::remove_file(&out);
    }
}

#[test]
fn rotated_fixture_comes_out_physically_rotated() {
    let src = fixtures_dir().join("h264-aac-rotated.mp4");
    // Coded (pre-rotation) dimensions of the source — the display matrix is
    // metadata, so codecpar reports the unrotated frame (96×64).
    let src_ctx = open(&src);
    let svi = first_video(&src_ctx);
    let (cw, ch) = {
        let par = src_ctx.streams()[svi].codecpar();
        (par.width, par.height)
    };
    drop(src_ctx);
    assert_ne!(cw, ch, "rotated fixture must be non-square to prove the swap");

    let out = transcode(&src, "rotated");
    let ctx = open(&out);
    let vi = first_video(&ctx);
    let (ow, oh) = {
        let par = ctx.streams()[vi].codecpar();
        (par.width, par.height)
    };
    drop(ctx);
    assert_eq!(
        (ow, oh),
        (ch, cw),
        "autorotation must swap dimensions (source {cw}x{ch} -> output should be {ch}x{cw})"
    );
    let _ = std::fs::remove_file(&out);
}
