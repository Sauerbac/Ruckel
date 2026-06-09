# ADR-0024: Frontend action model — rail-footer Convert, status-only bar, de-chipped meta line

- Status: Accepted
- Date: 2026-06-09
- Revises: [ADR-0019](0019-frontend-window-and-layout.md) (status bar no longer
  carries the primary action), [ADR-0020](0020-frontend-interaction-model.md)
  (the status/action-bar table), and the format-chip amendment of
  [ADR-0023](0023-frontend-polish-v2.md) (the extension chip is de-chipped).

## Context

A grilling pass on 2026-06-09 revisited the v2 action model. Three things that
were "locked 2026-06-08" no longer held up:

1. The status bar was the single morphing **action** slot (`Convert` / `Cancel` /
   `Convert Again` / `Clear`). The intent was to make Convert *prominent*; on
   review, a green button sharing a crowded ink bar with status text is no more
   prominent than the alternative, and the bar carried four different button
   labels across phases.
2. `Clear` and `Convert Again` were both questioned. `Convert Again` is
   functionally identical to `Convert` (`convertAgain` → `beginBatch(rows)`), and
   `Clear`'s stated justification (per-row removal, ADR-0023) was the wrong one —
   per-row removal does not replace a bulk reset.
3. The meta line read as "mixed": one **boxed** source-extension chip
   (ADR-0023 amendment) sitting beside three plain dot-separated values. The box
   borrowed the badge vocabulary the system otherwise reserves.

The governing aesthetic (ADR-0018: flat, instant, one-visual-one-meaning) is
unchanged; this ADR only relocates and prunes affordances within it.

## Decision

**Primary action moves to a pinned right-rail footer.** The Convert control
leaves the status bar and becomes a large button pinned to the bottom of the
right (options) panel, directly below the knobs it acts on. It is the **single
morphing action slot**, just relocated:

| Phase | Rail-footer button |
|---|---|
| No files | **Convert**, visible but **disabled** — `--fill` / `--line` / `--muted`, no accent (same "absence of green = not ready" vocabulary as the options above it, ADR-0020) |
| Ready | **Convert** — green `.btn.accent` |
| Converting | **Cancel** — danger `#C8341D` |
| Done | **Convert** — green; re-running a finished batch legitimately surfaces collisions (the `_ppt` outputs now exist) through the existing modal (ADR-0014), **not** special-cased |

There is exactly one place to look for "the main action" in every phase. The
button is **pinned** (not in the panel's scroll flow) so it can never scroll out
of reach; at the fixed 860×560 size the rail does not actually overflow, but the
pin holds regardless.

**The status bar becomes status-only.** It keeps, with **no buttons**:

| Phase | Status bar |
|---|---|
| No files | `NO FILES` (mono, muted) |
| Ready | `N FILES · ~X MB` (mono) |
| Converting | green `●` blink + `N%` + elapsed, with the full-width accent **progress fill** along its base |
| Done | `N DONE · N ERRORS` (green / red counts) |

The full-width progress fill is the bar's reason to exist as a solid-ink block; it
is a global batch signal the rail footer cannot give.

**`Clear` is removed.** The reset path is: a **new drop auto-clears** the list
(`handleDrop` sets `rows = []`), or **per-row remove** (ADR-0023) trims
individuals. *Accepted tradeoff:* after a finished batch there is no one-tap
"empty the window without dropping anything"; for a utility this small that is
acceptable.

**`Convert Again` is removed.** The done-state button is simply **Convert**
(see the table above). No separate label, no separate handler.

**The meta line is de-chipped — and the box vocabulary is formalised into two
tiers.** The source-extension chip loses its outline box and becomes plain mono,
uppercase, dot-separated with the rest: `MOV · 80 MB · 1:23 · 1920×1080`. This
removes the "mixed" feeling and de-emphasises a low-value datum (input is already
filtered to video by ADR-0011; output is always mp4). The governing rule:

- **Filled box** (ink border + colour fill) = a **status outcome** — Done /
  Error / Warning badges only.
- **Outline box** (hairline `--line`, muted, no fill) = a **soft indicator** —
  the **Custom** deviation tag only.
- **Plain mono** = data values (filenames, sizes, durations, resolutions,
  extensions).

No box is used for plain data.

## Consequences

- The "single morphing primary action" principle is preserved, only relocated
  from the status bar to the rail footer; the user looks in one place per phase.
- The status bar sheds its action role and earns its keep through the global
  progress fill alone.
- Two affordances disappear (`Clear`, `Convert Again`), trimming the done-state
  to one button; re-converting a done batch routes through the normal collision
  flow rather than a special case.
- The box vocabulary is now a clean two-tier system; the meta line is uniform.
- Supersedes: the status/action-bar table in [ADR-0020](0020-frontend-interaction-model.md),
  the "carries … the primary action" line in [ADR-0019](0019-frontend-window-and-layout.md),
  and the "keeps its `·` separators / `[MP4]` chip" detail of the ADR-0023
  format-chip amendment.
- Implementation touches `OptionsPanel.svelte` (host the pinned footer button or
  lift it to `Shell.svelte`), `StatusBar.svelte` (drop all buttons; keep status +
  fill), `Shell.svelte` (rewire `onConvert` / `onCancel`; drop `onClear` /
  `convertAgain` from the bar), and `FileRow.svelte` (de-chip ext, lines 109–113).
