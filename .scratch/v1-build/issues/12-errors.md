# I4 — Errors

Status: needs-triage

## What to build

Per-file error handling that never aborts the batch (ADR-0015). When a job fails (unreadable file,
no video stream, FFmpeg error), the encoder records the error and **continues** with the next job.
The failing file's row expands inline to show the message in JetBrains Mono (red badge); there is
**no error modal**. At the end, the status bar summarizes "N done · N errors", and
`conversion:done` carries the per-file error list.

## Acceptance criteria

- [ ] A failing job emits `conversion:file_error` and the batch continues to completion
- [ ] The failing row shows a red badge and auto-expands with the error message in JetBrains Mono
- [ ] No modal appears for errors — inline rows + status-bar summary only
- [ ] `conversion:done` reports accurate succeeded/failed counts and the error list; status bar shows "N done · N errors"

## Blocked by

- 04 (encoder + done/error events)
- 05 (error-row + status-bar component states)
