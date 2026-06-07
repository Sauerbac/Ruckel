# I3 — Collision

Status: needs-triage

## What to build

Detect and resolve output collisions **in pre-flight only** (ADR-0014). When a planned
`<stem>_ppt.mp4` already exists on disk, pre-flight reports it. The frontend shows the collision
modal listing all conflicts with a per-file choice of **Cancel / Override / Rename** plus a bulk
"Apply to all". The user's resolutions mutate the Conversion Plan (rename → new output path,
override → overwrite flag, cancel → drop that job), and the resolved plan is sent to
`start_conversion`. No encoding begins until all collisions are resolved.

## Acceptance criteria

- [ ] Pre-flight detects every existing `_ppt.mp4` collision before any encode starts
- [ ] Modal lists all conflicts with per-file Cancel/Override/Rename + a working "Apply to all"
- [ ] Each resolution behaves correctly: Rename writes to the new name, Override overwrites, Cancel skips that file and continues the batch
- [ ] The resolved plan round-trips back to `start_conversion`; encoding reflects the resolutions

## Blocked by

- 05 (collision modal component state)
- 08 (scanner/plan producing the output paths to check)
