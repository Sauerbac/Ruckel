# ADR-0016: Tauri command surface and event contract

- Status: Accepted
- Date: 2026-06-07

## Context

The frontend drives the backend through Tauri IPC. We want focused commands and a clear event
stream rather than monolithic handlers.

## Decision

**Commands** (thin handlers that delegate immediately):

| Command | Input | Returns |
|---|---|---|
| `preflight` | `paths: Vec<String>` | `PreflightResult` (plan + collisions) |
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
- Events drive Svelte stores directly ([ADR-0002](0002-svelte-frontend.md)).
- Under Tauri v2 ([ADR-0001](0001-tauri-v2-windows-desktop.md)), these use the v2 command/event
  APIs.
