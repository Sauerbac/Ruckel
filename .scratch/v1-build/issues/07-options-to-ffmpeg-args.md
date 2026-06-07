# B2 — Options → ffmpeg args mapping

Status: ready-for-agent

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
