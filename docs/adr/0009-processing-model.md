# ADR-0009: Sequential processing, flat folder walk

- Status: Accepted
- Date: 2026-06-07

## Context

A folder drop may contain many videos. We must decide concurrency and how deep to scan.

## Decision

- **Sequential:** convert one file at a time.
- **Flat walk:** include only the immediate files in the dropped folder; no subdirectory
  recursion.

## Consequences

- FFmpeg at `-preset medium` already saturates the CPU; parallel encodes would fight over
  cores and complicate progress and cancellation. Sequential keeps both simple and keeps the
  machine usable.
- Flat scanning gives predictable, surprise-free behavior — dropping a folder won't recurse
  into unrelated nested directories.
- Progress is reported per-file plus an overall batch position
  ([ADR-0013](0013-encoder-process-control.md)).

## Alternatives considered

- **Parallel encodes** — rejected: no throughput win on a CPU-bound encoder, much harder
  cancellation/progress.
- **Recursive walk** — rejected for v1: unpredictable scope; could be a future opt-in.
