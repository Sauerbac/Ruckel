# ADR-0020: Frontend interaction model

- Status: Accepted
- Date: 2026-06-08
- Revises: the 2026-06-07 draft — the phase-mirroring model is unchanged; empty
  states, selection styling, the drop-target treatment, and error/collision color are
  updated to the ADR-0018 v2 system.

## Context

The UI must mirror the backend's pre-flight → encode phases ([ADR-0010](0010-preflight-encode-separation.md))
and present the four options ([ADR-0007](0007-user-options-and-presets.md)) and error/collision
flows ([ADR-0014](0014-collision-resolution.md), [ADR-0015](0015-error-handling.md)) clearly,
in the v2 visual language (ADR-0018): flat ink-bordered surfaces, mono data, a reserved
green accent, sharp corners.

## Decision

**Selection styling (applies everywhere):** a **selected** control inverts to solid
ink + paper text — never green. Green is reserved for live/active/success status, so a
green pulse or check always means "the system is doing/did this," never "you picked
this."

**Left panel — drop zone and file list (one persistent drop target, no mode switch):**

- *Empty state:* a sharp-cornered **dashed `--accent` marquee** filling the panel, a
  centered sharp-square glyph, mono uppercase "DROP VIDEO OR FOLDER" / "Converts to
  PowerPoint-ready MP4". On drag-over the dashes go solid and the panel fills a faint
  accent tint.
- *File rows are display-only* (status, not a browser — no row-selection invert; hover
  → `--fill`). *Phased reveal* per row:

  | Phase | Visible data |
  |---|---|
  | On drop | Filename + size + extension |
  | After pre-flight | + duration + resolution (mono) |
  | During conversion | Inline flat **bordered** progress bar (1.5px ink frame, accent fill with a hard ink right-tick; no animation/gradient) |
  | Done | Green `#1FB866` check badge |
  | Warning | Amber `#D9821A` badge — skipped/filtered files |
  | Error | Red `#C8341D` badge; row auto-expands with message in mono, red left border |

**Right panel — options:**

- *Empty state (waiting for files):* controls fully drawn but **disabled** — `--fill`
  backgrounds, `--muted` labels, borders lightened to `--line`, **no accent**. The
  absence of green is the "not ready" signal. (No opacity-based dimming — fades aren't
  in the utility vocabulary.)
- Three preset buttons above four segmented option groups; raw values shown; the
  selected preset/segment **inverts to ink** (per the selection rule above). Deviating
  from a preset shows a muted **Custom** indicator.

**Status / action bar (full-width solid-ink block):**

> **Superseded by [ADR-0024](0024-frontend-action-model-revision.md):** the primary
> action moved to a **pinned right-rail footer** (the single morphing slot:
> Convert → Cancel → Convert). This bar is now **status-only** — no `CLEAR`, no
> `CONVERT AGAIN`. The table below is retained for history.

| State | Left | Right |
|---|---|---|
| Ready | "N FILES · ~X MB" (mono) | Green "CONVERT" (`.btn.accent`) |
| Converting | Green `●` pulse + elapsed time | "CANCEL" |
| Done | "N DONE · N ERRORS" (green/red counts) | "CLEAR" / "CONVERT AGAIN" |

During conversion a flat accent progress fill tracks overall batch progress.

**Collision modal:** blocking; a flat panel with a `1.5px` ink border over a **hard
ink scrim** (no blur, no shadow, no rounding); each conflict marked **amber** `#D9821A`.
Lists conflicts with per-file Cancel/Override/Rename + bulk "Apply to all". Dismissing
cancels the whole conversion.

**Errors:** no completion modal — inline row expansion + status-bar summary only.

## Consequences

- The phased reveal makes backend progress legible without extra screens.
- Reserving green for status (selection uses ink-invert) keeps the three-way outcome
  signal (green/amber/red) unambiguous.
- Disabled-as-empty-state replaces the old opacity dimming and stays inside the flat
  utility vocabulary.
- Deferred to a future **Advanced mode** (collapsible chevron at the bottom of the
  right panel): raw CRF stepper (18–28) and a custom FFmpeg flags input. Out of scope
  for v1 to keep it simple.
