# F2 — Component states in gallery

Status: ready-for-agent

## What to build

Every locked frontend component, in every locked state, rendered in the gallery from **mock props**
— visual only, **not** wired to live IPC (that happens in the Phase 3 feature slices). Prop shapes
come from the frozen S1 contract types.

States to render (per the locked frontend design in ADR-0019/0020):

- **File row** across its phased-reveal states: on-drop (filename + size + extension), post-preflight
  (adds duration + resolution), converting (inline progress bar at e.g. 40%), done (teal checkmark),
  error (red badge, row auto-expanded with the message in JetBrains Mono).
- **Right-panel options**: the segmented option groups (Resolution / Quality / Framerate / Audio),
  the three presets, and the **Custom** indicator state.
- **Status/action bar**: all three states — ready (file count + size + Convert), converting (overall
  progress fill + elapsed + Cancel), done (N done · N errors + Clear/Convert Again).
- **Collision modal**: the conflict list with per-file Cancel/Override/Rename + bulk "Apply to all".

## Acceptance criteria

- [ ] Each component renders every locked state in the gallery from mock props
- [ ] Components consume the generated S1 contract types for their prop shapes (no ad-hoc shapes)
- [ ] Data values (filenames, sizes, durations, percentages) use JetBrains Mono; UI text uses Inter
- [ ] Per-state gallery screenshots match the locked frontend spec (ADR-0019/0020)
- [ ] No live IPC calls — components are driven purely by props

## Blocked by

- 02 (primitives + gallery harness)
- 03 (contract types for prop shapes)
