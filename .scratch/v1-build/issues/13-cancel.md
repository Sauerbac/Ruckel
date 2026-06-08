# I5 — Cancel

Status: complete

## What to build

Cancellation of an in-flight batch (ADR-0013). `cancel_conversion` signals a shared cancellation
token; the encoder kills the active FFmpeg child process, deletes the in-progress temp file (B1),
stops the batch, and emits `conversion:cancelled`. The status bar's Cancel button (shown during
converting) triggers it; the UI returns to a clean, ready-able state.

## Acceptance criteria

- [ ] Cancel during an encode kills the FFmpeg process promptly and stops the batch
- [ ] The in-progress temp file is removed; no partial `_ppt.mp4` remains; already-finished outputs are kept
- [ ] `conversion:cancelled` is emitted and the UI returns to a clean state
- [ ] The Cancel button is available only during the converting state

## Blocked by

- 04 (encoder runner + process control)
- 05 (converting status-bar state with Cancel)
