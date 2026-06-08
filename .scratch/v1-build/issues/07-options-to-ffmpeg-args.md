# B2 — Options → ffmpeg args mapping

Status: complete

## What to build

A pure function mapping the four user options (Resolution, Quality/CRF, Framerate cap, Audio
bitrate) from a `ConversionJob` into the exact FFmpeg argument vector. The PowerPoint-safe flags
(`-c:v libx264 -profile:v high -pix_fmt yuv420p -movflags +faststart -c:a aac`) are always present
and a hardcoded `-level` is never emitted (ADR-0006). The encoder consumes this instead of the
tracer's hardcoded Presentation values.

## Acceptance criteria

- [ ] Pure options→args function covered by table tests for each preset (Presentation / High Quality / Compact) and representative custom combinations
- [ ] PowerPoint-safe flags asserted present in every generated arg vector; no `-level` ever emitted
- [ ] Resolution/framerate "Original" passes through without forcing scale/fps filters; non-original values apply the correct filters
- [ ] Encoder runner uses the mapping rather than hardcoded flags

## Blocked by

- 03 (the four-options contract type)
- 04 (the encoder runner to plug into)

## Comments

**Implemented.** The arg-building moved out of `runner.rs` into a new pure module
`encoder/args.rs`, the single source of truth for the command line:

- `ffmpeg_args(job, output)` — pure (no IO/process), so every preset and custom
  combo is table-tested. Assembles the fixed PowerPoint-safe video flags
  (`libx264 / high / yuv420p / preset medium`, **never `-level`**, ADR-0006), then
  maps the four options (ADR-0007), then `+faststart` + `-progress`.
- Option → flag mapping, each `Original` value passing through with no filter:
  - Resolution → `-vf scale=-2:H` (H = 1080/720/480; `-2` preserves aspect and
    keeps the width even for `yuv420p`). v1 simplification: the cap is a target,
    not a no-upscale ceiling (documented in the `scale_filter` doc-comment).
  - CRF → `-crf N` (raw value straight off `options.crf`).
  - Framerate → `-r N` (60/30/24).
  - Audio → `-c:a aac -b:a {192k,128k,96k}`, or `-an` for **No audio** (which drops
    the AAC codec/bitrate entirely).
- `runner.rs::encode` now calls `super::args::ffmpeg_args`; the `// B2 replaces
  this` hardcoded Presentation block is gone.

**Verification**

- `cargo test` — 26 green (23 unit incl. 8 new `args` tests, 1 codegen, 2 smoke).
  The real-FFmpeg smoke test still produces a valid `h264 / yuv420p / High` output,
  confirming the runner drives the new mapping end-to-end.
- Codegen drift gate clean: `git diff --exit-code src/lib/ipc` (no contract types
  changed — `args.rs` adds no `TS` types).
