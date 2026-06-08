# S2 — Tracer bullet

Status: complete

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

## Comments

**Implemented.** Real modules in their ADR-0017 homes:

- `probe.rs` — `ffprobe -print_format json` → `{ duration_secs, has_video }`;
  pure JSON parse is table-tested.
- `encoder/progress.rs` — parses `-progress pipe:1` blocks (`out_time_us`, `fps`,
  `speed`, `progress=`); `percent_of(out_time, duration)` clamps + guards zero.
- `encoder/runner.rs` — `ffmpeg_args` carries the PowerPoint-safe flags (libx264 /
  high / yuv420p / +faststart / aac), **no `-level`**, Presentation values
  hardcoded (B2 swaps in the options→args map); `encode()` spawns ffmpeg, drains
  stderr on a thread (no pipe deadlock), feeds progress to a callback, returns the
  output path. Direct-write for now; temp + atomic rename is B1.
- `encoder/cancel.rs` — cloneable `CancelToken`; `commands.rs` — thin `preflight` /
  `start_conversion` / `cancel_conversion` + the sequential batch loop emitting the
  contract events. `lib.rs` registers the handlers + a `sidecar_command` helper
  (resolves dev `binaries/<triple>.exe` vs bundled `<name>.exe`, `CREATE_NO_WINDOW`).

Sidecars are spawned via `std::process::Command` (ADR-0004), not the shell plugin.

**Frontend.** `Shell.svelte` drives the tracer on the real left panel: Tauri
drag-drop → `preflight` → auto `start_conversion` → `conversion:progress` advances a
real bordered progress bar → `done`/`error`. Right-panel options stay in the F1
disabled state (presets are I2). Gallery harness skips IPC (browser, no Tauri).

**Verification**

- `cargo test` — 20 green (17 unit + 1 codegen + 2 smoke).
- Self-generating real-FFmpeg smoke test (`tests/smoke.rs`, skipped if `binaries/`
  empty): synthesizes a 2 s `lavfi` clip, probes it, encodes, and asserts the
  output is `codec=h264 / pix_fmt=yuv420p / profile=High`, non-empty, re-probeable,
  with progress advancing past 0 %. A pre-cancelled token leaves no `_ppt.mp4`.
- `npm run check` (svelte-check + tsc) clean; `npm run build` succeeds. The full
  Tauri app compiles (`generate_context!` / `generate_handler!` expand cleanly).

**Needs a human eyeball** (per the PRD verification philosophy — GUI drag-drop
can't be exercised headlessly): run `npm run tauri dev`, drop one video on the left
panel, and confirm the bar advances and a valid `<stem>_ppt.mp4` appears beside the
source. The backend path is proven by the smoke test; only the live IPC + OS
drag-drop round-trip is unverified here.
