# ADR-0010: Pre-flight / encode separation

- Status: Accepted
- Date: 2026-06-07

## Context

Encoding is slow and irreversible-feeling; the fast checks (does this file have a video stream?
where does its output go? does that path already exist?) should never be discovered mid-batch.

## Decision

Split the work into two phases. **Pre-flight** runs all fast checks and produces a
fully-resolved **Conversion Plan**; the **encoder** then executes that plan blindly.

Pre-flight steps, in order:

1. Scan folder → collect candidate files (flat — [ADR-0009](0009-processing-model.md)).
2. Filter by file type ([ADR-0011](0011-file-type-filtering.md)).
3. `ffprobe` each candidate → `duration_us`, confirm a video stream is present.
4. Reject audio-only files (no video stream) → error summary.
5. Compute output paths, detect collisions.
6. Resolve collisions via UI if any ([ADR-0014](0014-collision-resolution.md)).
7. Emit the final Conversion Plan to the encoder.

## Consequences

- No mid-batch surprises: every decision is made before the first frame is encoded.
- The encoder is dumb and testable — it consumes a plan and emits events.
- `duration_us` from pre-flight feeds the progress percentage
  ([ADR-0013](0013-encoder-process-control.md)).
- Maps directly onto the frontend's phased reveal
  ([ADR-0020](0020-frontend-interaction-model.md)).
