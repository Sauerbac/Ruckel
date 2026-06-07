# ADR-0013: Encoder process control — progress and cancellation

- Status: Accepted
- Date: 2026-06-07

## Context

The encoder spawns FFmpeg per job ([ADR-0004](0004-ffmpeg-sidecar-not-ffi.md)) and must report
live progress and support a clean cancel.

## Decision

**Progress:**

- Invoke FFmpeg with `-progress pipe:1 -stats_period 0.5`.
- FFmpeg writes `key=value` progress pairs to stdout; stderr carries only errors.
- `out_time_us / duration_us` (duration from the pre-flight probe) → real percentage.
- Frontend receives progress events at ~2 Hz.

**Cancellation:**

- `child.kill()` on the running FFmpeg process, then delete the temp output file
  ([ADR-0012](0012-output-safety-temp-and-rename.md)).

## Consequences

- Accurate percentage because duration is known up front, not parsed from FFmpeg's own
  estimate.
- Clean stdout/stderr split makes progress parsing and error capture independent.
- Cancel is immediate and leaves no partial files.
