//! Spawns FFmpeg per job, reads stdout/stderr, emits events (ADR-0013).
//!
//! The Tauri-facing batch loop lives in `commands.rs`; this module owns the
//! single-job encode so it is testable without a running Tauri app. The command
//! line comes from the pure [`super::args::ffmpeg_args`] mapping (B2); the
//! temp-file + atomic rename (B1) still pends.

use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::Stdio;

use super::args::ffmpeg_args;
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

/// Encode one job with the bundled FFmpeg sidecar, invoking `on_progress` for
/// every `-progress` block. Returns the final output path on success.
///
/// Output safety (ADR-0012): FFmpeg writes to `<stem>.tmp.mp4` in the
/// destination folder; only after a clean exit is it renamed atomically to the
/// final `<stem>_ppt.mp4` (same-volume, so the rename is truly atomic). A
/// crash, error, or cancel deletes the temp file, so the destination is only
/// ever replaced by a complete, valid file and no partial `_ppt.mp4` is ever
/// left behind. Cancellation (ADR-0013) kills FFmpeg and returns
/// [`EncodeError::Cancelled`].
pub fn encode<F>(
    job: &ConversionJob,
    duration_secs: f64,
    cancel: &CancelToken,
    mut on_progress: F,
) -> Result<PathBuf, EncodeError>
where
    F: FnMut(EncodeUpdate),
{
    let final_path = PathBuf::from(&job.output_path);
    let temp_path = temp_path_for(&final_path);

    let mut child = crate::sidecar_command("ffmpeg")
        .args(ffmpeg_args(job, &temp_path))
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
            let _ = std::fs::remove_file(&temp_path);
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
        let _ = std::fs::remove_file(&temp_path);
        return Err(EncodeError::Cancelled);
    }

    if !status.success() {
        let _ = std::fs::remove_file(&temp_path);
        return Err(EncodeError::Failed(ffmpeg_error_message(&log)));
    }

    // Atomic promote: the temp lives in the destination folder, so this is a
    // same-volume rename (ADR-0012). On failure, drop the temp so no partial
    // file is left behind.
    std::fs::rename(&temp_path, &final_path).map_err(|e| {
        let _ = std::fs::remove_file(&temp_path);
        EncodeError::Failed(format!("failed to finalize output: {e}"))
    })?;

    Ok(final_path)
}

/// The in-progress temp path beside the final output (ADR-0012):
/// `<stem>.tmp.mp4` in the destination folder, keeping the success rename
/// same-volume and therefore atomic.
fn temp_path_for(final_path: &Path) -> PathBuf {
    let stem = final_path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "output".into());
    let dir = final_path.parent().unwrap_or_else(|| Path::new("."));
    dir.join(format!("{stem}.tmp.mp4"))
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

    #[test]
    fn temp_path_sits_beside_final_as_tmp_mp4() {
        let temp = temp_path_for(Path::new("/videos/clip_ppt.mp4"));
        assert_eq!(temp.file_name().unwrap(), "clip_ppt.tmp.mp4");
        assert_eq!(temp.parent().unwrap(), Path::new("/videos"));
    }

    #[test]
    fn error_message_takes_the_last_nonempty_stderr_line() {
        let log = "Input #0 ...\n\n[libx264 @ ..] error: bad thing\n\n";
        assert_eq!(ffmpeg_error_message(log), "[libx264 @ ..] error: bad thing");
        assert_eq!(ffmpeg_error_message("   "), "ffmpeg failed");
    }
}
