# Append-on-drop + status-bar notice

Status: ready-for-agent

## Parent

Spec: **ADR-0025** (`docs/adr/0025-append-on-drop-convert-time-collision-and-per-job-cancel.md`),
"Drop appends; the batch is only ever trimmed by removing rows" and "Transient
feedback via the status bar". Reverses the "new drop auto-clears" reset path of
ADR-0024.

## What to build

A drop should **merge** its files into the current batch instead of replacing it.
Today `handleDrop` runs `rows = []` before pre-flight, so adding files after an
initial drop throws the first set away (bug #1).

End-to-end behaviour after this slice:

- A drop **merges** new files into the current batch, **deduplicated by
  `source_path`**. Re-dropping an already-present file is a no-op — a `done` row
  stays `done` with its badge. There is no replace gesture; the list empties only
  by removing rows down to zero (existing `reset()` path).
- Append applies in `idle` / `ready` / `done`. Dropping into a finished (`done`)
  batch returns the phase to `ready` while existing `done` / `error` / `cancelled`
  rows **keep their badges** until the next Convert.
- Drops stay **ignored** during `reading` and `converting` (existing `busy` guard) —
  mid-encode the batch is locked, silently.
- `StatusBar` gains an optional `notice?: string | null` prop that **overrides** the
  status line for ~2.5s then auto-reverts (the derived `status` is the ground truth
  underneath). It is fired only for **"No convertible video found"** when a drop
  yields zero new files. Mid-convert ignored drops stay silent. This replaces the
  old empty-state drop-zone error surface, which no longer shows once a list is loaded.

Because Slice 01 already moved collision detection to convert time, `handleDrop`
simply stops touching collision state entirely — no per-drop collision array to
re-align on merge.

## Acceptance criteria

- [ ] Dropping a second batch appends its rows to the existing list rather than
      replacing it; total row count is the union.
- [ ] Re-dropping a file already in the list is a no-op (dedup by `source_path`);
      a `done` row stays `done`.
- [ ] Dropping into a `done` batch flips the phase to `ready` while existing
      done/error/cancelled badges persist until the next Convert.
- [ ] Drops during `reading`/`converting` are ignored with no notice.
- [ ] A drop yielding zero convertible files shows the "No convertible video found"
      notice in the status bar for ~2.5s, then the bar reverts to its real status.
- [ ] `StatusBar` `notice?` prop overrides the status line when set and is `null`
      otherwise; existing four states unchanged when no notice is active.
- [ ] Frontend typecheck/lint pass.

## Blocked by

- `01-convert-time-collision-check.md` (append builds on the convert-time collision
  model so `handleDrop` no longer manages a pre-flight collision array).
