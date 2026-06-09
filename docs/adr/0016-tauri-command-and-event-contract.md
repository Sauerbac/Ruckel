# ADR-0016: Tauri command surface and event contract

- Status: Accepted
- Date: 2026-06-07
- Amended by: [ADR-0025](0025-append-on-drop-convert-time-collision-and-per-job-cancel.md)
  (`PreflightResult.collisions` removed; `check_collisions` + `cancel_job` commands
  and the `conversion:file_cancelled` event added) and
  [ADR-0026](0026-preflight-skips-invalid-candidates.md)
  (`PreflightResult` gains `skipped: u32`). The command/event tables below predate
  both — see those ADRs for the current surface.

## Context

The frontend drives the backend through Tauri IPC. We want focused commands and a clear event
stream rather than monolithic handlers.

## Decision

**Commands** (thin handlers that delegate immediately):

| Command | Input | Returns |
|---|---|---|
| `preflight` | `paths: Vec<String>` | `PreflightResult` (plan + files + collisions) |
| `start_conversion` | `plan: Vec<ConversionJob>` | `()` (progress via events) |
| `cancel_conversion` | — | `()` |

**Events:**

| Event | Payload |
|---|---|
| `conversion:progress` | `{ file_index, total_files, file_name, percent, fps, speed }` |
| `conversion:file_done` | `{ file_index, output_path }` |
| `conversion:file_error` | `{ file_index, file_name, error_message }` |
| `conversion:done` | `{ succeeded, failed, errors: [{ file_name, error_message }] }` |
| `conversion:cancelled` | `{}` |

## Consequences

- Pre-flight and encode are separate round-trips, matching the phase split
  ([ADR-0010](0010-preflight-encode-separation.md)) and the frontend phased reveal
  ([ADR-0020](0020-frontend-interaction-model.md)).
- `PreflightResult` carries per-file probe metadata (`files: Vec<FileProbe>` — duration,
  width, height, plus the source's on-disk `size_bytes`) 1:1 with `plan` by position, so
  the frontend can show size, duration + resolution after pre-flight without a second
  round-trip. `size_bytes` is filesystem metadata (not an ffprobe field) and is an `f64`
  so the generated TS stays a plain `number`. `ConversionJob` stays clean as the encoder
  input. `PreflightResult` also carries `skipped: u32` — the count of scanned candidates
  that failed to probe or had no video stream, skipped rather than aborted (ADR-0026).
- `PreflightResult.collisions` *(removed by ADR-0025)* originally listed each planned
  `<stem>_ppt.mp4` already on disk; collision detection moved out of pre-flight to the
  convert-time `check_collisions` command. The three-way resolution (Override / Rename /
  Cancel, ADR-0014) and the round-trip back through `start_conversion` are unchanged.
- Events drive Svelte stores directly ([ADR-0002](0002-svelte-frontend.md)).
- Under Tauri v2 ([ADR-0001](0001-tauri-v2-windows-desktop.md)), these use the v2 command/event
  APIs.
