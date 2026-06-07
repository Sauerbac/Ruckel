# ADR-0006: Fixed PowerPoint-safe output encoding flags

- Status: Accepted
- Date: 2026-06-07
- Supersedes: the `-profile:v main -level 3.1` flags from the initial planning session

## Context

Every Ruckel output must reliably play in modern PowerPoint on Windows. Certain encode
properties are non-negotiable for that and are never exposed to the user. The original spec
pinned `-profile:v main -level 3.1`, but:

- **`-level 3.1` caps at ~720p30.** The resolution knob offers Original/1080p/720p/480p
  (default Original, which can be 1080p or 4K), so a fixed level 3.1 either becomes a no-op
  (x264 silently raises it) or emits an out-of-spec stream. PowerPoint 2013+ plays
  content well above level 3.1.
- **`main` profile** omits the 8×8 transform and leaves ~5–15% compression efficiency on the
  table. PowerPoint has supported High profile since 2010.

## Decision

Every output is muxed with this fixed, non-user-facing flag set:

```
-c:v libx264
-profile:v high
-pix_fmt yuv420p          # PowerPoint requires 8-bit 4:2:0
-movflags +faststart      # moov atom at front — instant load in PowerPoint
-c:a aac                  # FFmpeg native AAC encoder (redistributable, no libfdk)
```

- **No hardcoded `-level`** — x264 computes a conforming level from the actual
  resolution/framerate.
- Encode speed fixed at `-preset medium`, never exposed.

## Consequences

- Outputs remain 100% PowerPoint-compatible while gaining High-profile efficiency.
- The resolution knob can hand x264 any size without conflicting with a level cap.
- The native AAC encoder avoids the non-redistributable libfdk-aac licensing problem.

## Alternatives considered

- **Keep `main` / `level 3.1`** — rejected: contradicts the resolution options and needlessly
  caps quality/efficiency.
