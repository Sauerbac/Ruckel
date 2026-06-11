//! `#[tauri::command]` handlers. Kept thin — each handler delegates
//! immediately into `preflight` or `encoder` (ADR-0016, ADR-0017).

use std::path::Path;

use tauri::{AppHandle, Emitter, State};

use crate::encoder::cancel::{CancelToken, CancelledJobs};
use crate::encoder::runner::{self, EncodeError};
use crate::encoder::{
    events, CancelledEvent, ConversionError, DoneEvent, FileCancelledEvent, FileDoneEvent,
    FileErrorEvent, ProgressEvent,
};
use crate::preflight::collision::{output_path_for, Collision};
use crate::preflight::plan::{ConversionJob, ConversionOptions};
use crate::preflight::scanner;
use crate::preflight::{FileProbe, PreflightResult};
use crate::probe;

/// App-wide encoder state held in Tauri's managed-state registry: the
/// whole-batch abort flag the `cancel_conversion` command trips (ADR-0013) and
/// the per-job cancel set the `cancel_job` command writes (ADR-0025). Both are
/// reset at the start of each batch.
#[derive(Default)]
pub struct EncoderState {
    pub cancel: CancelToken,
    pub cancelled: CancelledJobs,
}

/// `preflight` (ADR-0016): resolve dropped paths into a Conversion Plan.
///
/// Dropped paths are scanned into candidate files — folders walked flat,
/// extension-filtered (B3, ADR-0009/0011) — then each candidate is probed for a
/// video stream and turned into a job whose planned `<stem>_ppt.mp4` output is
/// computed (ADR-0008). Disk collision detection is *not* done here (ADR-0025):
/// it is checked fresh at convert time via `check_collisions`, since whether an
/// output exists only matters when encoding starts and goes stale once a batch
/// writes its outputs. Every job starts on the Presentation default; the
/// frontend overrides `options` from the chosen preset/controls before
/// `start_conversion` (I2).
#[tauri::command]
pub async fn preflight(paths: Vec<String>) -> Result<PreflightResult, String> {
    Ok(build_preflight(scanner::scan(paths), probe::probe))
}

/// Resolve scanned candidates into a plan, skipping every invalid one (ADR-0026).
///
/// A candidate is kept only if it probes `Ok` *and* carries a video stream;
/// everything else — no video stream, or any probe error (file won't open or
/// parse, ADR-0026) — is skipped and counted. The failure modes are treated
/// identically (no per-reason branching): to the user they all mean "not a
/// usable video, skip it". One stray file in a folder drop therefore no longer
/// discards the good files alongside it, finally conforming to ADR-0015
/// (pre-flight errors are skip-and-surface, never abort).
///
/// `probe` is injected so the loop's skip/keep logic is unit-testable without
/// opening real media files.
fn build_preflight<P>(candidates: Vec<std::path::PathBuf>, probe: P) -> PreflightResult
where
    P: Fn(&Path) -> Result<probe::ProbeResult, String>,
{
    let mut plan: Vec<ConversionJob> = Vec::new();
    let mut files: Vec<FileProbe> = Vec::new();
    let mut skipped = 0u32;

    for source in candidates {
        match probe(&source) {
            Ok(probed) if probed.has_video => {
                let output = output_path_for(&source);
                plan.push(ConversionJob {
                    source_path: source.to_string_lossy().into_owned(),
                    output_path: output.to_string_lossy().into_owned(),
                    options: ConversionOptions::PRESENTATION,
                });
                files.push(FileProbe {
                    duration_secs: probed.duration_secs,
                    width: probed.width,
                    height: probed.height,
                    size_bytes: std::fs::metadata(&source).map(|m| m.len() as f64).unwrap_or(0.0),
                });
            }
            // No video stream, or unreadable — skip and count (ADR-0026).
            _ => skipped += 1,
        }
    }

    PreflightResult { plan, files, skipped }
}

/// `check_collisions` (ADR-0016, ADR-0025): the single, authoritative collision
/// source. Tests each job's already-computed `output_path` against disk with no
/// re-probe, returning one [`Collision`] per planned output that already exists.
/// Run fresh on every Convert so re-converting a finished batch routes through
/// the modal instead of silently overwriting (ADR-0008, ADR-0014). The returned
/// `job_index` aligns to the job's position in the plan passed in.
#[tauri::command]
pub fn check_collisions(plan: Vec<ConversionJob>) -> Vec<Collision> {
    plan.iter()
        .enumerate()
        .filter(|(_, job)| Path::new(&job.output_path).exists())
        .map(|(index, job)| Collision {
            job_index: index as u32,
            source_path: job.source_path.clone(),
            output_path: job.output_path.clone(),
        })
        .collect()
}

/// `start_conversion` (ADR-0016): run the plan, reporting progress via events.
/// Returns immediately; the batch runs on a background thread.
#[tauri::command]
pub async fn start_conversion(
    app: AppHandle,
    state: State<'_, EncoderState>,
    plan: Vec<ConversionJob>,
) -> Result<(), String> {
    let cancel = state.cancel.clone();
    let cancelled = state.cancelled.clone();
    // Both cancellation mechanisms reset for the fresh batch (ADR-0025).
    cancel.reset();
    cancelled.reset();
    std::thread::spawn(move || run_batch(&app, &plan, &cancel, &cancelled));
    Ok(())
}

/// `cancel_conversion` (ADR-0016): trip the whole-batch flag; the encode thread
/// kills FFmpeg and cleans up at its next progress read (ADR-0013).
#[tauri::command]
pub fn cancel_conversion(state: State<'_, EncoderState>) {
    state.cancel.cancel();
}

/// `cancel_job` (ADR-0025): mark a single job (by its `file_index`) for
/// cancellation without aborting the batch. The runner skips it if still queued,
/// or kills FFmpeg and continues if it is the active job — emitting
/// `conversion:file_cancelled` either way.
#[tauri::command]
pub fn cancel_job(state: State<'_, EncoderState>, file_index: u32) {
    state.cancelled.cancel(file_index);
}

/// Run every job sequentially (ADR-0009), emitting the contract events. A
/// failed file is skipped and reported in the end-of-batch summary (ADR-0015);
/// a whole-batch cancel stops the batch and emits `conversion:cancelled`
/// (ADR-0013), while a per-job cancel skips just that job, emits
/// `conversion:file_cancelled`, and continues (ADR-0025).
fn run_batch(
    app: &AppHandle,
    plan: &[ConversionJob],
    cancel: &CancelToken,
    cancelled: &CancelledJobs,
) {
    let total = plan.len() as u32;
    let mut succeeded = 0u32;
    let mut failed = 0u32;
    let mut errors: Vec<ConversionError> = Vec::new();

    for (index, job) in plan.iter().enumerate() {
        if cancel.is_cancelled() {
            break;
        }
        let file_index = index as u32;

        // Per-job cancel of a still-queued job: skip it without ever spawning
        // FFmpeg, and report it (ADR-0025). The batch carries on.
        if cancelled.contains(file_index) {
            let _ = app.emit(events::FILE_CANCELLED, FileCancelledEvent { file_index });
            continue;
        }

        let file_name = file_name_of(&job.source_path);

        // Duration drives the real percentage (ADR-0013); re-probed here rather
        // than threaded through the user-facing job contract.
        let duration = probe::probe(Path::new(&job.source_path))
            .map(|p| p.duration_secs)
            .unwrap_or(0.0);

        // Stop this job on either the whole-batch abort or its own per-job
        // cancel (ADR-0025); the runner doesn't care which.
        let should_cancel = || cancel.is_cancelled() || cancelled.contains(file_index);
        let result = runner::encode(job, duration, &should_cancel, |update| {
            let _ = app.emit(
                events::PROGRESS,
                ProgressEvent {
                    file_index,
                    total_files: total,
                    file_name: file_name.clone(),
                    percent: update.percent,
                    fps: update.fps,
                    speed: update.speed,
                },
            );
        });

        match result {
            Ok(output) => {
                succeeded += 1;
                let _ = app.emit(
                    events::FILE_DONE,
                    FileDoneEvent {
                        file_index,
                        output_path: output.to_string_lossy().into_owned(),
                    },
                );
            }
            Err(EncodeError::Cancelled) => {
                // Whole-batch abort wins and stops everything; otherwise this was
                // a per-job cancel of the active job — report it and continue.
                if cancel.is_cancelled() {
                    let _ = app.emit(events::CANCELLED, CancelledEvent {});
                    return;
                }
                let _ = app.emit(events::FILE_CANCELLED, FileCancelledEvent { file_index });
                continue;
            }
            Err(EncodeError::Failed(message)) => {
                failed += 1;
                errors.push(ConversionError {
                    file_name: file_name.clone(),
                    error_message: message.clone(),
                });
                let _ = app.emit(
                    events::FILE_ERROR,
                    FileErrorEvent {
                        file_index,
                        file_name,
                        error_message: message,
                    },
                );
            }
        }
    }

    if cancel.is_cancelled() {
        let _ = app.emit(events::CANCELLED, CancelledEvent {});
        return;
    }

    let _ = app.emit(
        events::DONE,
        DoneEvent {
            succeeded,
            failed,
            errors,
        },
    );
}

/// Display name for events — the file's own name, not the full path.
fn file_name_of(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn video() -> probe::ProbeResult {
        probe::ProbeResult { duration_secs: 5.0, has_video: true, width: 1920, height: 1080 }
    }
    fn audio_only() -> probe::ProbeResult {
        probe::ProbeResult { duration_secs: 5.0, has_video: false, width: 0, height: 0 }
    }

    #[test]
    fn keeps_every_valid_candidate_with_zero_skipped() {
        let candidates = vec![PathBuf::from("a.mp4"), PathBuf::from("b.mov")];
        let result = build_preflight(candidates, |_| Ok(video()));
        assert_eq!(result.plan.len(), 2);
        assert_eq!(result.files.len(), 2);
        assert_eq!(result.skipped, 0);
    }

    #[test]
    fn skips_non_video_candidate_but_keeps_the_good_one() {
        // A real clip alongside an audio-only / renamed file (ADR-0026): the clip
        // is kept, the other is skipped — the drop is not discarded.
        let candidates = vec![PathBuf::from("clip.mp4"), PathBuf::from("song.mp3")];
        let result = build_preflight(candidates, |p| {
            if p.extension().and_then(|e| e.to_str()) == Some("mp4") {
                Ok(video())
            } else {
                Ok(audio_only())
            }
        });
        assert_eq!(result.plan.len(), 1);
        assert!(result.plan[0].source_path.ends_with("clip.mp4"));
        assert_eq!(result.skipped, 1);
    }

    #[test]
    fn skips_unreadable_candidate_instead_of_aborting() {
        // A probe Err (file won't open or parse) is skipped, not propagated —
        // one corrupt file no longer kills the whole drop.
        let candidates = vec![PathBuf::from("good.mp4"), PathBuf::from("corrupt.mp4")];
        let result = build_preflight(candidates, |p| {
            if p.file_name().and_then(|n| n.to_str()) == Some("corrupt.mp4") {
                Err("probe failed".into())
            } else {
                Ok(video())
            }
        });
        assert_eq!(result.plan.len(), 1);
        assert_eq!(result.skipped, 1);
    }

    #[test]
    fn an_all_invalid_drop_yields_an_empty_plan_not_an_error() {
        let candidates = vec![PathBuf::from("x.bin"), PathBuf::from("y.bin")];
        let result = build_preflight(candidates, |_| Err("unreadable".into()));
        assert!(result.plan.is_empty());
        assert_eq!(result.skipped, 2);
    }
}
