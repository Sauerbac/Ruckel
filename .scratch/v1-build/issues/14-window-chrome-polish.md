# R1 — Final window chrome / edge-case polish

Status: complete

## What to build

Final pass once the features are wired: tighten the custom title-bar behavior (drag, close/minimize),
empty/loaded/converting/done transitions, focus and keyboard niceties, and any visual edge cases
that only surface with real data flowing through the panels. No new features — convergence on the
locked design (ADR-0018/0019/0020, **as revised by ADR-0024 rail-footer/status-only and ADR-0025
append/per-job-cancel/notice**) under real conditions.

## Acceptance criteria

- [ ] Title bar drag + close/minimize behave correctly; no maximize/resize
- [ ] All four phase transitions (empty → loaded → converting → done) look correct with real data
- [ ] Long filenames, many files, and small/odd resolutions don't break layout
- [ ] A full screenshot pass matches the locked frontend spec

## Blocked by

- 09, 10, 11, 12, 13 (all features wired)

## Comments

### Closed — 2026-06-09

A grilling pass re-pegged this against the current spec (ADR-0018/0019/0020 **as
revised by 0024 + 0025** — the original "locked spec" reference predated both).

- **Live-eyeball items confirmed fine** by a visual pass on the running app:
  title-bar drag/close/minimize, the empty → loaded → converting → done
  transitions with real data, long filenames, many-file scroll/fade, and the
  re-convert loop. No layout breakage found.
- **Two static nits** surfaced by a component read-through were rehomed to the
  `finishing-touches` feature (`issues/02`): the `0×0` resolution render for a
  dimensionless video, and defensive status-notice truncation.
- The genuine functional gap found in the same pass (pre-flight aborting a whole
  folder drop on one bad file) is `finishing-touches/issues/01` + ADR-0026.

Nothing further to do under R1 itself; closed.
