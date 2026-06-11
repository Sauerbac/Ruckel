//! In-process probe — the single entry point for reading a file's duration
//! and confirming a video stream is present. Shared by `preflight` and
//! `encoder` (ADR-0017).
//!
//! Reads the file directly through libavformat (`avformat_open_input` +
//! `avformat_find_stream_info` via rsmpeg, ADR-0027). The [`ProbeResult`]
//! surface is unchanged from v1 — pre-flight, the ADR-0026 skip semantics, and
//! the ADR-0011 extensionless fallback never noticed the swap.

use std::ffi::CString;
use std::path::Path;

use rsmpeg::avformat::AVFormatContextInput;

/// What a probe tells us about a candidate file: its duration (for accurate
/// progress, ADR-0013), its coded dimensions from the first video stream, and
/// whether it carries a video stream at all (a pre-flight error otherwise,
/// ADR-0015).
#[derive(Debug, Clone, PartialEq)]
pub struct ProbeResult {
    pub duration_secs: f64,
    pub has_video: bool,
    pub width: u32,
    pub height: u32,
}

/// Probe a file in-process via libavformat (ADR-0027). Opening also runs
/// `avformat_find_stream_info`, so stream parameters are populated by the time
/// we inspect them. Any failure to open or parse the file maps to the same
/// `Err(String)` that pre-flight treats as a skip (ADR-0026).
pub fn probe(input: &Path) -> Result<ProbeResult, String> {
    let url = CString::new(input.to_str().ok_or_else(|| {
        format!("path is not valid UTF-8: {}", input.display())
    })?)
    .map_err(|e| format!("path contains an interior NUL: {e}"))?;

    let ctx = AVFormatContextInput::open(&url)
        .map_err(|e| format!("failed to open {}: {e}", input.display()))?;

    let video_stream = ctx
        .streams()
        .iter()
        .find(|s| s.codecpar().codec_type().is_video());

    let has_video = video_stream.is_some();
    let (width, height) = video_stream
        .map(|s| {
            let par = s.codecpar();
            (par.width.max(0) as u32, par.height.max(0) as u32)
        })
        .unwrap_or((0, 0));

    Ok(ProbeResult {
        duration_secs: duration_secs(ctx.duration),
        has_video,
        width,
        height,
    })
}

/// Convert the format-level duration (`AVFormatContext.duration`, in
/// `AV_TIME_BASE` units) to seconds, matching the format duration the v1 probe
/// reported. Unknown durations (`AV_NOPTS_VALUE`, negative) fall back to `0.0`
/// exactly as v1's parser did.
fn duration_secs(raw: i64) -> f64 {
    if raw == rsmpeg::ffi::AV_NOPTS_VALUE || raw < 0 {
        return 0.0;
    }
    raw as f64 / rsmpeg::ffi::AV_TIME_BASE as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duration_known_value_converts_to_seconds() {
        // 12.48 s in AV_TIME_BASE (1e6) units.
        assert!((duration_secs(12_480_000) - 12.48).abs() < 1e-6);
    }

    #[test]
    fn duration_nopts_falls_back_to_zero() {
        assert_eq!(duration_secs(rsmpeg::ffi::AV_NOPTS_VALUE), 0.0);
    }

    #[test]
    fn duration_negative_falls_back_to_zero() {
        assert_eq!(duration_secs(-1), 0.0);
    }
}
