//! Pure options → FFmpeg argument mapping (B2).
//!
//! [`ffmpeg_args`] turns a [`ConversionJob`] into the exact argument vector for
//! the sidecar. It is the single source of truth for what flags an encode runs
//! with, and it is pure (no IO, no process) so every preset and custom combo is
//! covered by table tests below. The runner ([`super::runner::encode`]) calls
//! it and does nothing else to the command line.
//!
//! Two flag groups, kept distinct:
//! - **Fixed PowerPoint-safe flags (ADR-0006)** — always present, never
//!   user-facing, and crucially *no hardcoded `-level`* (x264 derives a
//!   conforming level from the actual size/rate).
//! - **The four user options (ADR-0007)** — resolution, CRF, framerate, audio —
//!   mapped from `job.options`.

use std::path::Path;

use crate::preflight::plan::{Audio, ConversionJob, Framerate, Resolution};

/// Build the full FFmpeg argument vector for one job, writing to `output`.
///
/// `output` is passed separately from `job.output_path` because the runner
/// encodes to a temp path first and renames on success (B1); the args must
/// point at wherever bytes are actually written.
pub fn ffmpeg_args(job: &ConversionJob, output: &Path) -> Vec<String> {
    let opt = &job.options;
    let mut args: Vec<String> = vec![
        "-hide_banner".into(),
        "-loglevel".into(),
        "error".into(),
        // Overwrite the (temp/)output path; real collisions are resolved in
        // pre-flight (ADR-0014), so reaching here means writing is intended.
        "-y".into(),
        "-i".into(),
        job.source_path.clone(),
    ];

    // PowerPoint-safe video, fixed and non-user-facing (ADR-0006). No `-level`.
    args.extend(
        [
            "-c:v",
            "libx264",
            "-profile:v",
            "high",
            "-pix_fmt",
            "yuv420p",
            "-preset",
            "medium",
        ]
        .map(String::from),
    );

    // Resolution cap → aspect-preserving scale (ADR-0007); `Original` omits it.
    if let Some(vf) = scale_filter(opt.resolution) {
        args.push("-vf".into());
        args.push(vf);
    }

    // Quality — raw CRF (ADR-0007).
    args.push("-crf".into());
    args.push(opt.crf.to_string());

    // Framerate cap (ADR-0007); `Original` omits it so the source rate passes
    // through untouched.
    if let Some(fps) = framerate_value(opt.framerate) {
        args.push("-r".into());
        args.push(fps.to_string());
    }

    // Audio: the PowerPoint-safe AAC codec (ADR-0006) at the chosen bitrate, or
    // `-an` to drop audio entirely (ADR-0007).
    args.extend(audio_args(opt.audio));

    // Faststart muxer flag — instant load in PowerPoint (ADR-0006).
    args.extend(["-movflags", "+faststart"].map(String::from));
    // Machine-readable progress on stdout at ~2 Hz (ADR-0013).
    args.extend(["-progress", "pipe:1", "-stats_period", "0.5", "-nostats"].map(String::from));

    args.push(output.to_string_lossy().into_owned());
    args
}

/// The `scale` filter value for a resolution cap, or `None` for `Original`.
///
/// `-2` lets x264 derive the width from the capped height while preserving the
/// source aspect ratio and keeping the result divisible by 2 (required by
/// `yuv420p`, ADR-0006). The cap is expressed by target height; sources smaller
/// than the cap are scaled to it (a deliberate v1 simplification — the knob is a
/// target, not a no-upscale ceiling).
fn scale_filter(resolution: Resolution) -> Option<String> {
    let height = match resolution {
        Resolution::Original => return None,
        Resolution::P1080 => 1080,
        Resolution::P720 => 720,
        Resolution::P480 => 480,
    };
    Some(format!("scale=-2:{height}"))
}

/// The numeric `-r` value for a framerate cap, or `None` for `Original`.
fn framerate_value(framerate: Framerate) -> Option<u32> {
    match framerate {
        Framerate::Original => None,
        Framerate::Fps60 => Some(60),
        Framerate::Fps30 => Some(30),
        Framerate::Fps24 => Some(24),
    }
}

/// The audio argument group: AAC at the chosen bitrate, or `-an` for `None`.
fn audio_args(audio: Audio) -> Vec<String> {
    match audio {
        Audio::None => vec!["-an".into()],
        Audio::Kbps192 => aac("192k"),
        Audio::Kbps128 => aac("128k"),
        Audio::Kbps96 => aac("96k"),
    }
}

fn aac(bitrate: &str) -> Vec<String> {
    vec!["-c:a".into(), "aac".into(), "-b:a".into(), bitrate.into()]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preflight::plan::ConversionOptions;

    const OUT: &str = "out_ppt.mp4";

    fn args_for(options: ConversionOptions) -> Vec<String> {
        let job = ConversionJob {
            source_path: "in.mov".into(),
            output_path: OUT.into(),
            options,
        };
        ffmpeg_args(&job, Path::new(OUT))
    }

    /// True if `args` contains `flag` immediately followed by `value`.
    fn has_pair(args: &[String], flag: &str, value: &str) -> bool {
        args.windows(2).any(|w| w[0] == flag && w[1] == value)
    }

    // --- Fixed PowerPoint-safe flags hold for every preset and custom combo ---

    #[test]
    fn powerpoint_safe_video_flags_present_and_no_level_for_all_presets() {
        for opt in [
            ConversionOptions::PRESENTATION,
            ConversionOptions::HIGH_QUALITY,
            ConversionOptions::COMPACT,
        ] {
            let args = args_for(opt);
            assert!(has_pair(&args, "-c:v", "libx264"), "{opt:?}");
            assert!(has_pair(&args, "-profile:v", "high"), "{opt:?}");
            assert!(has_pair(&args, "-pix_fmt", "yuv420p"), "{opt:?}");
            assert!(has_pair(&args, "-preset", "medium"), "{opt:?}");
            assert!(has_pair(&args, "-movflags", "+faststart"), "{opt:?}");
            assert!(!args.iter().any(|a| a == "-level"), "no -level: {opt:?}");
        }
    }

    #[test]
    fn progress_requested_on_stdout_and_output_is_last() {
        let args = args_for(ConversionOptions::PRESENTATION);
        assert!(has_pair(&args, "-progress", "pipe:1"));
        assert_eq!(args.last().unwrap(), OUT);
    }

    // --- Preset mappings (table) ---

    #[test]
    fn presentation_preset_maps_to_passthrough_res_rate_crf23_aac128() {
        let args = args_for(ConversionOptions::PRESENTATION);
        assert!(!args.iter().any(|a| a == "-vf"), "Original res: no scale");
        assert!(!args.iter().any(|a| a == "-r"), "Original rate: no -r");
        assert!(has_pair(&args, "-crf", "23"));
        assert!(has_pair(&args, "-c:a", "aac"));
        assert!(has_pair(&args, "-b:a", "128k"));
    }

    #[test]
    fn high_quality_preset_maps_to_passthrough_res_rate_crf18_aac192() {
        let args = args_for(ConversionOptions::HIGH_QUALITY);
        assert!(!args.iter().any(|a| a == "-vf"));
        assert!(!args.iter().any(|a| a == "-r"));
        assert!(has_pair(&args, "-crf", "18"));
        assert!(has_pair(&args, "-b:a", "192k"));
    }

    #[test]
    fn compact_preset_maps_to_720p_30fps_crf28_aac96() {
        let args = args_for(ConversionOptions::COMPACT);
        assert!(has_pair(&args, "-vf", "scale=-2:720"));
        assert!(has_pair(&args, "-r", "30"));
        assert!(has_pair(&args, "-crf", "28"));
        assert!(has_pair(&args, "-b:a", "96k"));
    }

    // --- Per-knob mappings (custom combos) ---

    #[test]
    fn each_resolution_maps_to_its_capped_height() {
        for (res, vf) in [
            (Resolution::P1080, "scale=-2:1080"),
            (Resolution::P720, "scale=-2:720"),
            (Resolution::P480, "scale=-2:480"),
        ] {
            let args = args_for(ConversionOptions {
                resolution: res,
                ..ConversionOptions::PRESENTATION
            });
            assert!(has_pair(&args, "-vf", vf), "{res:?}");
        }
    }

    #[test]
    fn each_framerate_maps_to_its_rate() {
        for (fr, r) in [
            (Framerate::Fps60, "60"),
            (Framerate::Fps30, "30"),
            (Framerate::Fps24, "24"),
        ] {
            let args = args_for(ConversionOptions {
                framerate: fr,
                ..ConversionOptions::PRESENTATION
            });
            assert!(has_pair(&args, "-r", r), "{fr:?}");
        }
    }

    #[test]
    fn no_audio_maps_to_an_and_drops_the_aac_codec() {
        let args = args_for(ConversionOptions {
            audio: Audio::None,
            ..ConversionOptions::PRESENTATION
        });
        assert!(args.iter().any(|a| a == "-an"));
        assert!(!args.iter().any(|a| a == "-c:a"), "no codec when -an");
        assert!(!args.iter().any(|a| a == "-b:a"), "no bitrate when -an");
    }

    #[test]
    fn custom_crf_value_passes_through() {
        let args = args_for(ConversionOptions {
            crf: 21,
            ..ConversionOptions::PRESENTATION
        });
        assert!(has_pair(&args, "-crf", "21"));
    }
}
