# ADR-0012: Output safety — temp file + atomic rename

- Status: Accepted
- Date: 2026-06-07

## Context

In the Override case ([ADR-0014](0014-collision-resolution.md)) the output path is also an
existing file the user wants to keep until the new one is known-good. A crash or cancel
mid-encode must never leave a half-written file in place of a real one.

## Decision

FFmpeg writes to a temp file `<name>_ppt.tmp.mp4`. On success it is renamed atomically to the
final path via `MoveFileEx`. On cancel or error the temp file is deleted.

## Consequences

- The destination is only ever replaced by a complete, valid file.
- Cancellation ([ADR-0013](0013-encoder-process-control.md)) leaves no partial files on disk.
- The temp file lives in the destination folder, so the rename is same-volume and truly atomic.
