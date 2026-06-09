//! `#[tauri::command]` handlers. Kept thin — each handler delegates
//! immediately into `preflight` or `encoder` (ADR-0016, ADR-0017).

use std::path::Path;

use tauri::{AppHandle, Emitter, State};

use crate::encoder::cancel::CancelToken;
use crate::encoder::runner::{self, EncodeError};
use crate::encoder::{
    events, CancelledEvent, ConversionError, DoneEvent, FileDoneEvent, FileErrorEvent,
    ProgressEvent,
};
use crate::preflight::collision::{output_path_for, Collision};
use crate::preflight::plan::{ConversionJob, ConversionOptions};
use crate::preflight::scanner;
use crate::preflight::{FileProbe, PreflightResult};
use crate::probe;

/// App-wide encoder state held in Tauri's managed-state registry: the shared
/// cancellation flag the `cancel_conversion` command trips (ADR-0013).
#[derive(Default)]
pub struct EncoderState {
    pub cancel: CancelToken,
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
    let mut plan: Vec<ConversionJob> = Vec::new();
    let mut files: Vec<FileProbe> = Vec::new();

    for source in scanner::scan(paths) {
        let probed = probe::probe(&source)?;
        if !probed.has_video {
            // Pre-flight error (ADR-0015): no video stream to convert.
            return Err(format!("{}: no video stream found", source.display()));
        }

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

    Ok(PreflightResult { plan, files })
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
        .filter_map(|(index, job)| {
            Path::new(&job.output_path).exists().then(|| Collision {
                job_index: index as u32,
                source_path: job.source_path.clone(),
                output_path: job.output_path.clone(),
            })
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
    cancel.reset();
    std::thread::spawn(move || run_batch(&app, &plan, &cancel));
    Ok(())
}

/// `cancel_conversion` (ADR-0016): trip the shared flag; the encode thread
/// kills FFmpeg and cleans up at its next progress read (ADR-0013).
#[tauri::command]
pub fn cancel_conversion(state: State<'_, EncoderState>) {
    state.cancel.cancel();
}

/// Run every job sequentially (ADR-0009), emitting the contract events. A
/// failed file is skipped and reported in the end-of-batch summary (ADR-0015);
/// a cancel stops the batch and emits `conversion:cancelled` (ADR-0013).
fn run_batch(app: &AppHandle, plan: &[ConversionJob], cancel: &CancelToken) {
    let total = plan.len() as u32;
    let mut succeeded = 0u32;
    let mut failed = 0u32;
    let mut errors: Vec<ConversionError> = Vec::new();

    for (index, job) in plan.iter().enumerate() {
        if cancel.is_cancelled() {
            break;
        }
        let file_index = index as u32;
        let file_name = file_name_of(&job.source_path);

        // Duration drives the real percentage (ADR-0013); re-probed here rather
        // than threaded through the user-facing job contract.
        let duration = probe::probe(Path::new(&job.source_path))
            .map(|p| p.duration_secs)
            .unwrap_or(0.0);

        let result = runner::encode(job, duration, cancel, |update| {
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
                let _ = app.emit(events::CANCELLED, CancelledEvent {});
                return;
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
