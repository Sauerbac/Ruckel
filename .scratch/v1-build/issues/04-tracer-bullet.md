# S2 — Tracer bullet

Status: ready-for-agent

## What to build

The thinnest possible **end-to-end happy path**, touching every seam exactly once. It proves the
two biggest integration risks — the Tauri IPC round-trip and the real FFmpeg sidecar spawn — before
any breadth is built.

Flow: real **drag-drop of a single file** onto the real left panel (from F1) → `invoke('preflight')`
→ **real ffprobe** reads duration and confirms a video stream is present → a one-job
`ConversionPlan` → `invoke('start_conversion')` → **real ffmpeg** encodes with the PowerPoint-safe
flags (`libx264 / -profile:v high / yuv420p / +faststart / aac`, no hardcoded `-level`) using
hardcoded **Presentation** preset values → `conversion:progress` events carrying a **real percent**
(`out_time` ÷ probed duration) → a single progress bar advances → `conversion:file_done` +
`conversion:done` → the UI shows "done" and a valid `<stem>_ppt.mp4` exists in the source folder.

**Out of scope** (deferred to later slices): folder walk / multiple files, collision handling,
the options UI / presets, error UX, cancel, design polish beyond what F1 provides.

This establishes the real `probe.rs`, a minimal `encoder/runner.rs` + `progress.rs`, and the thin
`commands.rs` handlers, in their ADR-0017 module homes.

## Acceptance criteria

- [ ] Dropping one video file produces a valid `<stem>_ppt.mp4` next to the source, encoded with the PowerPoint-safe flags
- [ ] ffprobe-derived duration drives a real progress percentage shown on a progress bar
- [ ] `preflight` → `start_conversion` round-trips over real Tauri IPC; `conversion:progress/file_done/done` events drive the UI
- [ ] The real FFmpeg/ffprobe sidecars are spawned via the encoder/probe modules (no mock)

## Blocked by

- 01 (sidecars must be present and spawnable)
- 03 (the IPC contract types the round-trip uses)
