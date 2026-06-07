# ADR-0020: Frontend interaction model

- Status: Accepted
- Date: 2026-06-07

## Context

The UI must mirror the backend's pre-flight → encode phases ([ADR-0010](0010-preflight-encode-separation.md))
and present the four options ([ADR-0007](0007-user-options-and-presets.md)) and error/collision
flows ([ADR-0014](0014-collision-resolution.md), [ADR-0015](0015-error-handling.md)) clearly.

## Decision

**Left panel — drop zone and file list (one persistent drop target, no mode switch):**

- *Empty state:* full-panel dashed teal border, centered teal icon, "Drop any video file or
  folder" / "Converts to PowerPoint-ready MP4".
- *Phased reveal* per file row:

  | Phase | Visible data |
  |---|---|
  | On drop | Filename (Inter) + size + extension |
  | After pre-flight | + duration + resolution (JetBrains Mono) |
  | During conversion | Inline per-file progress bar |
  | Done | Teal checkmark badge |
  | Error | Red badge; row auto-expands with message in JetBrains Mono, red left border |

**Right panel — options:**

- *Empty state:* all controls at 40% opacity, no label — dimming alone signals "waiting for
  files."
- Three preset buttons above four segmented control groups; raw values shown; selected segment
  highlighted teal. Deviating from a preset shows a muted **Custom** indicator.

**Status / action bar (full width):**

| State | Left | Right |
|---|---|---|
| Ready | "N files · ~X MB" | Teal "Convert" |
| Converting | Elapsed time | "Cancel" |
| Done | "N done · N errors" | "Clear" / "Convert Again" |

During conversion a teal progress bar fills the bar background to overall batch progress.

**Collision modal:** blocking; lists conflicts; per-file Cancel/Override/Rename + bulk "Apply
to all". Dismissing cancels the whole conversion.

**Errors:** no completion modal — inline row expansion + status-bar summary only.

## Consequences

- The phased reveal makes backend progress legible without extra screens.
- Dimming-as-empty-state avoids placeholder clutter.
- Deferred to a future **Advanced mode** (collapsible chevron at the bottom of the right
  panel): raw CRF stepper (18–28) and a custom FFmpeg flags input. Out of scope for v1 to keep
  it simple.
