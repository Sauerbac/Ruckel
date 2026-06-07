# ADR-0007: Four user-facing options and three presets

- Status: Accepted
- Date: 2026-06-07

## Context

Beyond the fixed PowerPoint-safe flags ([ADR-0006](0006-powerpoint-safe-output-encoding.md)),
users need a small, comprehensible set of quality/size tradeoffs — not the full FFmpeg surface.

## Decision

Expose exactly **four options**, with raw values (no Low/Medium/High abstraction):

| Option | Values | Default | FFmpeg flag |
|---|---|---|---|
| Resolution | Original / 1080p / 720p / 480p | Original | `-vf scale=W:H` (aspect-preserving) |
| Quality (CRF) | CRF 18 / 23 / 28 | CRF 23 | `-crf N` |
| Framerate cap | Original / 60 / 30 / 24 | Original | `-r N` |
| Audio | 192k / 128k / 96k / No audio | 128k | `-b:a Nk` or `-an` |

Three **presets** snap all four at once:

| Preset | Resolution | CRF | Framerate | Audio |
|---|---|---|---|---|
| Presentation | Original | 23 | Original | 128k |
| High Quality | Original | 18 | Original | 192k |
| Compact | 720p | 28 | 30 | 96k |

When the four controls match no preset, a display-only **Custom** indicator appears (not a
selectable preset).

## Consequences

- Small decision space; presets cover the common cases, raw values respect power users.
- The CRF control surfaces three of the 18–28 range; the full stepper is deferred to a future
  Advanced mode (see [ADR-0020](0020-frontend-interaction-model.md)).
- "No audio" maps to `-an`; an audio bitrate on a source without an audio stream is a no-op.
