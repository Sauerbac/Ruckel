//! Spawns FFmpeg per job, reads stdout/stderr, emits events (ADR-0013).
//!
//! The Tauri-facing batch loop lives in `commands.rs`; this module owns the
//! single-job encode so it is testable without a running Tauri app. The tracer
//! (S2) hardcodes the Presentation values and writes the output directly;
//! later slices refine it — the options→args mapping (B2) and the temp-file +
//! atomic rename (B1).

use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::Stdio;

use super::cancel::CancelToken;
use super::progress::{percent_of, ProgressParser};
use crate::preflight::plan::ConversionJob;

/// A live progress update for one job, already resolved to a real percentage.
#[derive(Debug, Clone, PartialEq)]
pub struct EncodeUpdate {
    pub percent: f64,
    pub fps: f64,
    pub speed: f64,
}

/// Build the FFmpeg argument vector for a job. The PowerPoint-safe flags
/// (ADR-0006) are always present and no `-level` is ever emitted. The tracer
/// hardcodes the **Presentation** values (CRF 23 / AAC 128k, resolution and
/// framerate untouched); B2 replaces the quality/scale/rate/audio portion with
/// the options→args mapping derived from `job.options`.
pub fn ffmpeg_args(job: &ConversionJob, output: &Path) -> Vec<String> {
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

    // PowerPoint-safe video, fixed and non-user-facing (ADR-0006).
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
    // Quality — tracer: Presentation CRF 23.
    args.extend(["-crf", "23"].map(String::from));
    // PowerPoint-safe audio (ADR-0006) — tracer: Presentation 128k.
    args.extend(["-c:a", "aac", "-b:a", "128k"].map(String::from));
    // Faststart muxer flag — instant load in PowerPoint (ADR-0006).
    args.extend(["-movflags", "+faststart"].map(String::from));
    // Machine-readable progress on stdout at ~2 Hz (ADR-0013).
    args.extend(["-progress", "pipe:1", "-stats_period", "0.5", "-nostats"].map(String::from));

    args.push(output.to_string_lossy().into_owned());
    args
}

/// Encode one job with the bundled FFmpeg sidecar, invoking `on_progress` for
/// every `-progress` block. Returns the written output path on success.
///
/// Cancellation (ADR-0013): when the token trips, FFmpeg is killed and the
/// partial output deleted, returning [`EncodeError::Cancelled`].
pub fn encode<F>(
    job: &ConversionJob,
    duration_secs: f64,
    cancel: &CancelToken,
    mut on_progress: F,
) -> Result<PathBuf, EncodeError>
where
    F: FnMut(EncodeUpdate),
{
    let output = PathBuf::from(&job.output_path);

    let mut child = crate::sidecar_command("ffmpeg")
        .args(ffmpeg_args(job, &output))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| EncodeError::Failed(format!("failed to spawn ffmpeg: {e}")))?;

    // Drain stderr on its own thread so a full pipe never deadlocks the encode;
    // with `-loglevel error` it stays empty unless something goes wrong.
    let stderr = child.stderr.take();
    let stderr_drain = stderr.map(|mut s| {
        std::thread::spawn(move || {
            let mut buf = String::new();
            let _ = s.read_to_string(&mut buf);
            buf
        })
    });

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| EncodeError::Failed("ffmpeg produced no stdout".into()))?;
    let mut parser = ProgressParser::new();

    for line in BufReader::new(stdout).lines() {
        if cancel.is_cancelled() {
            let _ = child.kill();
            let _ = child.wait();
            let _ = std::fs::remove_file(&output);
            return Err(EncodeError::Cancelled);
        }
        let Ok(line) = line else { break };
        if let Some(snap) = parser.feed_line(&line) {
            on_progress(EncodeUpdate {
                percent: percent_of(snap.out_time_us, duration_secs),
                fps: snap.fps,
                speed: snap.speed,
            });
        }
    }

    let status = child
        .wait()
        .map_err(|e| EncodeError::Failed(format!("waiting on ffmpeg failed: {e}")))?;

    let log = stderr_drain
        .and_then(|h| h.join().ok())
        .unwrap_or_default();

    if cancel.is_cancelled() {
        let _ = std::fs::remove_file(&output);
        return Err(EncodeError::Cancelled);
    }

    if !status.success() {
        let _ = std::fs::remove_file(&output);
        return Err(EncodeError::Failed(ffmpeg_error_message(&log)));
    }

    Ok(output)
}

/// Distinguishes a clean cancel from a real failure so the batch loop can
/// emit the right event (ADR-0013, ADR-0015).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EncodeError {
    Cancelled,
    Failed(String),
}

impl std::fmt::Display for EncodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EncodeError::Cancelled => write!(f, "cancelled"),
            EncodeError::Failed(msg) => write!(f, "{msg}"),
        }
    }
}

/// Trim FFmpeg's stderr to the last, most relevant line(s) for the UI.
fn ffmpeg_error_message(log: &str) -> String {
    let tail = log.trim();
    if tail.is_empty() {
        return "ffmpeg failed".into();
    }
    tail.lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("ffmpeg failed")
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preflight::plan::ConversionOptions;

    fn job() -> ConversionJob {
        ConversionJob {
            source_path: "in.mov".into(),
            output_path: "out_ppt.mp4".into(),
            options: ConversionOptions::PRESENTATION,
        }
    }

    #[test]
    fn args_carry_every_powerpoint_safe_flag_and_no_level() {
        let args = ffmpeg_args(&job(), Path::new("out_ppt.mp4"));
        let joined = args.join(" ");
        for needle in [
            "-c:v libx264",
            "-profile:v high",
            "-pix_fmt yuv420p",
            "-movflags +faststart",
            "-c:a aac",
        ] {
            assert!(joined.contains(needle), "missing {needle:?} in {joined:?}");
        }
        assert!(!args.iter().any(|a| a == "-level"), "must not emit -level");
    }

    #[test]
    fn args_request_progress_on_stdout() {
        let args = ffmpeg_args(&job(), Path::new("out_ppt.mp4"));
        assert!(args.windows(2).any(|w| w == ["-progress", "pipe:1"]));
    }

    #[test]
    fn output_path_is_the_last_arg() {
        let args = ffmpeg_args(&job(), Path::new("out_ppt.mp4"));
        assert_eq!(args.last().unwrap(), "out_ppt.mp4");
    }

    #[test]
    fn error_message_takes_the_last_nonempty_stderr_line() {
        let log = "Input #0 ...\n\n[libx264 @ ..] error: bad thing\n\n";
        assert_eq!(ffmpeg_error_message(log), "[libx264 @ ..] error: bad thing");
        assert_eq!(ffmpeg_error_message("   "), "ffmpeg failed");
    }
}
