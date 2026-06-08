# B1 — Output safety: temp file + atomic rename

Status: complete

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

## Comments

**Implemented** in `encoder/runner.rs::encode`:

- FFmpeg now writes to `temp_path_for(final) = <stem>.tmp.mp4` in the **destination
  folder** (`ffmpeg_args(job, &temp_path)`), so the success promote is a same-volume
  rename and therefore truly atomic (ADR-0012). `temp_path_for` is a small pure
  helper (unit-tested).
- On a clean exit the temp is `std::fs::rename`d to the final `<stem>_ppt.mp4`
  (Rust maps this to `MoveFileExW` with replace-existing on Windows — the ADR-0012
  Override case is handled). A rename failure deletes the temp and surfaces a
  `Failed`.
- Every non-success exit — mid-loop cancel, post-wait cancel, FFmpeg failure —
  deletes the **temp**, never a real file. The destination is only ever replaced by
  a complete, valid output, and no partial `_ppt.mp4` is ever left behind.

**Verification**

- `cargo test` — 33 green (30 unit, 1 codegen, 2 smoke).
- The real-FFmpeg smoke tests now assert the temp lifecycle directly:
  - `encode_produces_valid_powerpoint_safe_ppt_mp4` — the final `_ppt.mp4` exists,
    is valid (`h264 / yuv420p / High`), **and** `clip_ppt.tmp.mp4` no longer exists
    (proving the rename, not a copy).
  - `cancel_leaves_no_output_file` — a pre-tripped token leaves neither the final
    output nor the temp file.
  Both skip when `binaries/` is empty.
- `cargo clippy` clean for the changed code; IPC drift gate clean.
