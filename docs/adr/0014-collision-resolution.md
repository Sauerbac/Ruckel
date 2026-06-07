# ADR-0014: Collision resolution in pre-flight

- Status: Accepted
- Date: 2026-06-07

## Context

A planned `_ppt.mp4` output ([ADR-0008](0008-output-location-and-naming.md)) may already exist.
Discovering this mid-batch would force an interruption while encoding is underway.

## Decision

Detect and resolve all collisions **in pre-flight**, before any encoding starts. Three per-file
resolutions:

- **Cancel:** skip this file, continue with the rest of the batch.
- **Override:** overwrite the existing output (safely, via temp + rename —
  [ADR-0012](0012-output-safety-temp-and-rename.md)).
- **Rename:** user supplies a new output filename.

A bulk "Apply to all" offers Override or Rename across every conflict. Dismissing the
resolution UI without resolving cancels the entire conversion.

## Consequences

- No mid-batch prompts; the encoder runs an already-resolved plan.
- Originals are protected even under Override.
- UI treatment: a blocking modal ([ADR-0020](0020-frontend-interaction-model.md)).
