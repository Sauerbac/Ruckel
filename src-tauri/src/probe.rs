//! `ffprobe` wrapper — the single entry point for reading a file's duration
//! and confirming a video stream is present. Shared by `preflight` and
//! `encoder` (ADR-0017).

use std::path::Path;

use serde::Deserialize;

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

/// Probe a file with the bundled `ffprobe` sidecar (ADR-0004).
pub fn probe(input: &Path) -> Result<ProbeResult, String> {
    let output = crate::sidecar_command("ffprobe")
        .args([
            "-v",
            "quiet",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
        ])
        .arg(input)
        .output()
        .map_err(|e| format!("failed to spawn ffprobe: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "ffprobe failed for {}: {}",
            input.display(),
            output.status
        ));
    }

    parse_probe_json(&output.stdout)
}

/// Pure parse of `ffprobe -print_format json` output, factored out so it is
/// table-testable without spawning a sidecar.
fn parse_probe_json(stdout: &[u8]) -> Result<ProbeResult, String> {
    #[derive(Deserialize)]
    struct Output {
        #[serde(default)]
        format: Format,
        #[serde(default)]
        streams: Vec<Stream>,
    }
    #[derive(Deserialize, Default)]
    struct Format {
        #[serde(default)]
        duration: Option<String>,
    }
    #[derive(Deserialize)]
    struct Stream {
        #[serde(default)]
        codec_type: Option<String>,
        #[serde(default)]
        width: Option<u32>,
        #[serde(default)]
        height: Option<u32>,
    }

    let parsed: Output =
        serde_json::from_slice(stdout).map_err(|e| format!("malformed ffprobe output: {e}"))?;

    let has_video = parsed
        .streams
        .iter()
        .any(|s| s.codec_type.as_deref() == Some("video"));

    let video_stream = parsed
        .streams
        .iter()
        .find(|s| s.codec_type.as_deref() == Some("video"));

    let width = video_stream.and_then(|s| s.width).unwrap_or(0);
    let height = video_stream.and_then(|s| s.height).unwrap_or(0);

    let duration_secs = parsed
        .format
        .duration
        .and_then(|d| d.parse::<f64>().ok())
        .filter(|d| d.is_finite() && *d >= 0.0)
        .unwrap_or(0.0);

    Ok(ProbeResult {
        duration_secs,
        has_video,
        width,
        height,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_duration_and_detects_video_stream() {
        let json = br#"{
            "streams": [
                { "codec_type": "video", "width": 1920, "height": 1080 },
                { "codec_type": "audio" }
            ],
            "format": { "duration": "12.480000" }
        }"#;
        let got = parse_probe_json(json).unwrap();
        assert_eq!(got.has_video, true);
        assert!((got.duration_secs - 12.48).abs() < 1e-6);
        assert_eq!(got.width, 1920);
        assert_eq!(got.height, 1080);
    }

    #[test]
    fn audio_only_file_has_no_video() {
        let json = br#"{
            "streams": [{ "codec_type": "audio" }],
            "format": { "duration": "60.0" }
        }"#;
        let got = parse_probe_json(json).unwrap();
        assert_eq!(got.has_video, false);
        assert_eq!(got.width, 0);
        assert_eq!(got.height, 0);
        assert!((got.duration_secs - 60.0).abs() < 1e-6);
    }

    #[test]
    fn missing_duration_falls_back_to_zero() {
        let json = br#"{ "streams": [{ "codec_type": "video" }], "format": {} }"#;
        let got = parse_probe_json(json).unwrap();
        assert_eq!(got.has_video, true);
        assert_eq!(got.duration_secs, 0.0);
    }

    #[test]
    fn malformed_json_is_an_error() {
        assert!(parse_probe_json(b"not json").is_err());
    }

    #[test]
    fn video_without_dimensions_falls_back_to_zero() {
        let json = br#"{
            "streams": [{ "codec_type": "video" }],
            "format": { "duration": "5.0" }
        }"#;
        let got = parse_probe_json(json).unwrap();
        assert_eq!(got.has_video, true);
        assert_eq!(got.width, 0);
        assert_eq!(got.height, 0);
    }
}
