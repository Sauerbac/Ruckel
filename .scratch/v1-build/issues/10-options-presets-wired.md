# I2 — Options/presets wired

Status: needs-triage

## What to build

Wire the right-panel controls to real encoding. The segmented option groups and the three presets
(Presentation / High Quality / Compact) drive the four options on each Conversion Job; selecting a
preset snaps all four controls; deviating from a preset shows the **Custom** indicator (display-only,
not selectable). The chosen options flow through B2's mapping into the actual FFmpeg invocation. The
right panel is dimmed at 40% with no selection until files are present (ADR-0020).

## Acceptance criteria

- [ ] Selecting a preset sets all four controls; the output reflects that preset's values
- [ ] Deviating any control from a preset shows the Custom indicator; presets remain the only selectable bundles
- [ ] The four options chosen in the UI are the ones applied by the encoder (verified end-to-end on output)
- [ ] Right panel is dimmed/inert until at least one file is loaded

## Blocked by

- 05 (option-group + preset component states)
- 07 (options→ffmpeg args mapping)
