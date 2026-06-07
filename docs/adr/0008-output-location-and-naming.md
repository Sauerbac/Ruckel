# ADR-0008: Output location and `_ppt` naming contract

- Status: Accepted
- Date: 2026-06-07

## Context

Users drop files or folders and expect the converted result to be easy to find, without a
configuration step for output directories.

## Decision

Each output is written **in the source file's own folder**, named `<source-stem>_ppt.mp4`.

- `presentation.avi` → `presentation_ppt.mp4`
- Folder drops convert each file in place, beside its original.

## Consequences

- Zero-config output location; results sit next to their sources.
- Re-running on an already-converted `x_ppt.mp4` would produce `x_ppt_ppt.mp4` — acceptable
  and visible, not silently destructive.
- An existing `_ppt.mp4` is a **collision**, resolved in pre-flight
  ([ADR-0014](0014-collision-resolution.md)).
- Originals are never overwritten except via an explicit Override choice, and even then only
  after a safe temp-file encode ([ADR-0012](0012-output-safety-temp-and-rename.md)).
