//! Pure options → [`EncoderConfig`] mapping (B2, ADR-0027).
//!
//! [`encoder_config`] turns a [`ConversionOptions`] into the fully-resolved
//! settings the in-process transcode loop ([`super::runner`]) applies to the
//! libav encoders and filter graph. It is the single source of truth for what
//! an encode runs with, and it is pure (no IO, no libav state) so every preset
//! and custom combo is covered by the table tests below — the in-process
//! successor to v1's argument-vector tests.
//!
//! Two groups, kept distinct:
//! - **Fixed PowerPoint-safe contract (ADR-0006)** — always the same, never
//!   user-facing: h264 *high*, `yuv420p`, faststart, and crucially **no
//!   `-level`** (x264 derives a conforming level from the actual size/rate).
//! - **The four user options (ADR-0007)** — resolution, CRF, framerate, audio.

use rsmpeg::ffi;

use crate::preflight::plan::{Audio, ConversionOptions, Framerate, Resolution};

/// The audio half of the contract: native AAC at a chosen bitrate, or no audio
/// stream at all (the in-process equivalent of `-an`, ADR-0007).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioConfig {
    None,
    Aac { bitrate_bps: i64 },
}

/// Everything the transcode loop needs to configure its encoders and video
/// filter graph for one job. The fixed fields encode the ADR-0006 contract; the
/// rest are the resolved user knobs.
#[derive(Debug, Clone, PartialEq)]
pub struct EncoderConfig {
    // --- Fixed PowerPoint-safe contract (ADR-0006), identical for every job ---
    /// The video encoder, by libav name. Always `libx264`.
    pub video_codec: &'static str,
    /// H.264 profile. Always `high`.
    pub profile: &'static str,
    /// x264 speed/efficiency preset. Always `medium`.
    pub preset: &'static str,
    /// Output pixel format. Always `yuv420p` (normalizes 10-bit/4:2:2 sources).
    pub pix_fmt: ffi::AVPixelFormat,
    /// H.264 level. **Always `None`** — x264 derives a conforming level from the
    /// actual size/rate; pinning one is exactly what ADR-0006 forbids.
    pub level: Option<i32>,
    /// mp4 `+faststart` (moov before mdat). Always `true`.
    pub faststart: bool,

    // --- The four user knobs (ADR-0007) ---
    /// Constant Rate Factor passed straight to x264.
    pub crf: u8,
    /// Target height for an aspect-preserving `scale=-2:H`, or `None` to pass the
    /// source resolution through (`Original`).
    pub scale_height: Option<u32>,
    /// Framerate cap for a CFR `fps=N` filter, or `None` to pass source timing
    /// through (`Original`).
    pub fps_cap: Option<u32>,
    /// AAC at a bitrate, or no audio stream.
    pub audio: AudioConfig,
}

/// Resolve the four user options into the full [`EncoderConfig`]. Pure.
pub fn encoder_config(options: &ConversionOptions) -> EncoderConfig {
    EncoderConfig {
        video_codec: "libx264",
        profile: "high",
        preset: "medium",
        pix_fmt: ffi::AV_PIX_FMT_YUV420P,
        level: None,
        faststart: true,

        crf: options.crf,
        scale_height: scale_height(options.resolution),
        fps_cap: fps_cap(options.framerate),
        audio: audio_config(options.audio),
    }
}

/// Target height for a resolution cap, or `None` for `Original`. The cap is a
/// target height; `-2` derives an even width preserving the source aspect ratio.
/// Sources smaller than the cap are scaled up to it (a deliberate v1
/// simplification — the knob is a target, not a no-upscale ceiling).
fn scale_height(resolution: Resolution) -> Option<u32> {
    match resolution {
        Resolution::Original => None,
        Resolution::P1080 => Some(1080),
        Resolution::P720 => Some(720),
        Resolution::P480 => Some(480),
    }
}

/// Framerate cap, or `None` for `Original`.
fn fps_cap(framerate: Framerate) -> Option<u32> {
    match framerate {
        Framerate::Original => None,
        Framerate::Fps60 => Some(60),
        Framerate::Fps30 => Some(30),
        Framerate::Fps24 => Some(24),
    }
}

/// AAC at the chosen bitrate (in bits/s), or no audio for `None`.
fn audio_config(audio: Audio) -> AudioConfig {
    match audio {
        Audio::None => AudioConfig::None,
        Audio::Kbps192 => AudioConfig::Aac { bitrate_bps: 192_000 },
        Audio::Kbps128 => AudioConfig::Aac { bitrate_bps: 128_000 },
        Audio::Kbps96 => AudioConfig::Aac { bitrate_bps: 96_000 },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- The fixed PowerPoint-safe contract holds for every preset (ADR-0006) ---

    #[test]
    fn powerpoint_safe_contract_holds_and_no_level_for_all_presets() {
        for opt in [
            ConversionOptions::PRESENTATION,
            ConversionOptions::HIGH_QUALITY,
            ConversionOptions::COMPACT,
        ] {
            let c = encoder_config(&opt);
            assert_eq!(c.video_codec, "libx264", "{opt:?}");
            assert_eq!(c.profile, "high", "{opt:?}");
            assert_eq!(c.preset, "medium", "{opt:?}");
            assert_eq!(c.pix_fmt, ffi::AV_PIX_FMT_YUV420P, "{opt:?}");
            assert!(c.faststart, "{opt:?}");
            assert_eq!(c.level, None, "no pinned level: {opt:?}");
        }
    }

    // --- Preset mappings (table) ---

    #[test]
    fn presentation_preset_passthrough_res_rate_crf23_aac128() {
        let c = encoder_config(&ConversionOptions::PRESENTATION);
        assert_eq!(c.scale_height, None);
        assert_eq!(c.fps_cap, None);
        assert_eq!(c.crf, 23);
        assert_eq!(c.audio, AudioConfig::Aac { bitrate_bps: 128_000 });
    }

    #[test]
    fn high_quality_preset_passthrough_res_rate_crf18_aac192() {
        let c = encoder_config(&ConversionOptions::HIGH_QUALITY);
        assert_eq!(c.scale_height, None);
        assert_eq!(c.fps_cap, None);
        assert_eq!(c.crf, 18);
        assert_eq!(c.audio, AudioConfig::Aac { bitrate_bps: 192_000 });
    }

    #[test]
    fn compact_preset_720p_30fps_crf28_aac96() {
        let c = encoder_config(&ConversionOptions::COMPACT);
        assert_eq!(c.scale_height, Some(720));
        assert_eq!(c.fps_cap, Some(30));
        assert_eq!(c.crf, 28);
        assert_eq!(c.audio, AudioConfig::Aac { bitrate_bps: 96_000 });
    }

    // --- Per-knob mappings (custom combos) ---

    #[test]
    fn each_resolution_maps_to_its_capped_height() {
        for (res, h) in [
            (Resolution::Original, None),
            (Resolution::P1080, Some(1080)),
            (Resolution::P720, Some(720)),
            (Resolution::P480, Some(480)),
        ] {
            let c = encoder_config(&ConversionOptions {
                resolution: res,
                ..ConversionOptions::PRESENTATION
            });
            assert_eq!(c.scale_height, h, "{res:?}");
        }
    }

    #[test]
    fn each_framerate_maps_to_its_cap() {
        for (fr, f) in [
            (Framerate::Original, None),
            (Framerate::Fps60, Some(60)),
            (Framerate::Fps30, Some(30)),
            (Framerate::Fps24, Some(24)),
        ] {
            let c = encoder_config(&ConversionOptions {
                framerate: fr,
                ..ConversionOptions::PRESENTATION
            });
            assert_eq!(c.fps_cap, f, "{fr:?}");
        }
    }

    #[test]
    fn each_audio_knob_maps_to_its_bitrate_or_none() {
        for (audio, expected) in [
            (Audio::Kbps192, AudioConfig::Aac { bitrate_bps: 192_000 }),
            (Audio::Kbps128, AudioConfig::Aac { bitrate_bps: 128_000 }),
            (Audio::Kbps96, AudioConfig::Aac { bitrate_bps: 96_000 }),
            (Audio::None, AudioConfig::None),
        ] {
            let c = encoder_config(&ConversionOptions {
                audio,
                ..ConversionOptions::PRESENTATION
            });
            assert_eq!(c.audio, expected, "{audio:?}");
        }
    }

    #[test]
    fn custom_crf_passes_through() {
        let c = encoder_config(&ConversionOptions {
            crf: 21,
            ..ConversionOptions::PRESENTATION
        });
        assert_eq!(c.crf, 21);
    }
}
