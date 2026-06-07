# R1 — Final window chrome / edge-case polish

Status: needs-triage

## What to build

Final pass once the features are wired: tighten the custom title-bar behavior (drag, close/minimize),
empty/loaded/converting/done transitions, focus and keyboard niceties, and any visual edge cases
that only surface with real data flowing through the panels. No new features — convergence on the
locked design (ADR-0018/0019/0020) under real conditions.

## Acceptance criteria

- [ ] Title bar drag + close/minimize behave correctly; no maximize/resize
- [ ] All four phase transitions (empty → loaded → converting → done) look correct with real data
- [ ] Long filenames, many files, and small/odd resolutions don't break layout
- [ ] A full screenshot pass matches the locked frontend spec

## Blocked by

- 09, 10, 11, 12, 13 (all features wired)
