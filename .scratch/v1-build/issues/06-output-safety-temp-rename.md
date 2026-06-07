# B1 — Output safety: temp file + atomic rename

Status: ready-for-agent

## What to build

Make the encoder write to a temporary file and atomically rename it into place only on success, so
a crashed or cancelled encode never leaves a partial `<stem>_ppt.mp4` looking like a finished output
(ADR-0012). Refines the tracer's direct-write encode in `encoder/runner.rs`.

## Acceptance criteria

- [ ] Encoder writes to a temp file in the destination folder, renames to the final `_ppt.mp4` only after FFmpeg exits successfully
- [ ] A failed/aborted encode leaves no final `_ppt.mp4` (temp is cleaned up)
- [ ] Self-generating real-FFmpeg smoke test (`ffmpeg -f lavfi -i testsrc` makes its own fixture) confirms the success path produces the renamed output; skipped when `binaries/` is empty

## Blocked by

- 04 (builds on the tracer's encoder runner)
