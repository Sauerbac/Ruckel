# ADR-0023: Frontend polish v2 — tooltips, layout stability, list & row affordances

- Status: Accepted
- Date: 2026-06-09
- Extends: [ADR-0018](0018-frontend-design-system.md) (design system — adds two
  primitives + reaffirms instant/flat), [ADR-0020](0020-frontend-interaction-model.md)
  (interaction model — group headers, row layout, list affordance, file intake,
  row removal), [ADR-0021](0021-tailwind-css.md). Adds a Tauri plugin dependency
  alongside [ADR-0022](0022-ffmpeg-fetched-into-binaries.md).

## Context

Eight concrete frontend annoyances surfaced in use of the v2 shell: settings
were undocumented; several elements (the Custom badge, the per-row progress bar,
the done output line) **reflowed** their neighbours when they appeared; the
option groups were ragged next to the full-width preset row; the file list showed
a scrollbar; the empty drop zone was drag-only; and there was no way to remove a
file once added.

Every fix is constrained by ADR-0018's governing rule: **flat, instant,
utilitarian neo-brutalism — no shadows, no gradients, no transitions** (the one
sanctioned motion is the converting blink), and **every control is always
visible** (hover only inverts fill, instantly). Two decisions below were steered
directly by that rule (tooltips show instantly; the remove control is persistent,
not hover-revealed).

## Decision

**Info tooltips (new primitive).** Each option group's header carries an info
icon next to its label. Hover **or** keyboard focus opens a flat popover (ink
border, paper bg, mono), **rendered at top level** so the right panel's
`overflow-auto` cannot clip it, opening **left** (toward the panel interior, away
from the window edge). Show/hide is **instant — no fade** (per ADR-0018). One
tooltip per group; its body **decodes the group's options**, not just a gloss.

**Uniform group headers (no-reflow).** Every option group header is a
**fixed-height row**: `[icon] LABEL` left, the **Custom** tag right (Preset group
only). Fixed height means the Custom indicator and the info icon appearing or
disappearing **never reflow** the panel — the root cause of the old Custom-badge
shift (the badge was ~2px taller than the bare label in a `justify-between` row).

**Row layout stability.** The per-row progress bar is unchanged (ADR-0020's flat
bordered bar). Rows grow **once**, uniformly, when conversion starts; **finishing
causes no second shift** — the **done output path occupies the line the progress
bar used**. The Done/Error badges are sized to the extension chip so line 1 never
changes height when a badge appears.

**Equal-width option segments.** The segmented control becomes **full-width with
equal (`flex-1`) cells**, so all four option groups sit flush to the panel width
like the preset row. (Presets were already equal-width; the groups were
content-width and ragged.) The panel reads as one uniform stack.

**List fade in place of a scrollbar.** The file list **hides its scrollbar**
(scroll still works via wheel/trackpad) and uses a **`mask-image`
linear-gradient** (~24px) to fade content to true transparency at top and bottom.
A scroll listener **drops each edge's gradient stop at its extreme** (no top fade
at the top, no bottom fade at the bottom, neither when the list fits). This static
spatial mask is an overflow affordance, **not eased motion**, so it stays within
ADR-0018; it is the one soft edge in an otherwise hard system.

**Click-to-browse file intake.** The empty drop zone becomes an actual **button**
(pointer cursor, hover affordance, Enter/Space). Clicking opens a **multi-select
Open dialog filtered to the video extensions** (mirroring `scanner.rs`
`VIDEO_EXTENSIONS`, [ADR-0011](0011-file-type-filtering.md)) plus an "All files"
entry for extensionless videos; selected paths flow through the **same pre-flight
path as a drag-drop**. This adds the **`tauri-plugin-dialog`** dependency (npm +
Cargo + a `dialog:allow-open` capability grant). **Folders stay drag-only** —
native multi-select file dialogs cannot also pick directories.

**Remove control on file rows (new primitive).** A **persistent square ✕** sits
at the far right of a row's first line, after the chips. Persistent — not
hover-revealed — because hover-reveal needs a forbidden fade and hiding
affordances is anti-brutalist. It is **subordinate**: extension-chip sized,
hairline `--line` border, `--muted` glyph, and it **inverts to `--danger` on
hover instantly** (the app's existing destructive language). It is **phase-gated**
— rendered only when the batch is **not converting**, because the encoder keeps
the row list 1:1 with the sent plan via `file_index`
([ADR-0016](0016-tauri-command-and-event-contract.md)) and mid-batch removal
would desync it. Removing the **last** row returns to the empty drop zone;
removing a row while `done` recomputes the status-bar succeeded/failed summary
from the remaining rows.

## Consequences

- Two new generic primitives join ADR-0018's signature set: the **info tooltip**
  (top-level, instant, opens-left) and the **remove ✕** (persistent, hairline,
  danger-on-hover). Both are Gallery-worthy states.
- The "no second reflow" rule for rows and the fixed-height group headers make
  the layout feel stable without violating the no-fade aesthetic.
- The list mask is the single deliberate exception to "hard edges everywhere,"
  justified as an overflow cue rather than decoration.
- `tauri-plugin-dialog` is the first non-log Tauri plugin and the first dialog
  capability; folder intake remains a drag-only asymmetry by platform constraint.
- Implementation work item: `.scratch/frontend-polish-v2/issues/01-frontend-polish.md`.

## Amendment (2026-06-09): tooltip structure, Quality label, segment order, format chip

Surfaced while reviewing the first implementation. Four refinements, all within
the governing aesthetic:

**Structured tooltips, no dot separators.** The tooltip body is no longer a
run-on sentence with `·` separators. Each popover is now a **heading** (the
group name), a one-line **gloss** of what the knob does, and an
**option→description table** (a two-column `auto 1fr` grid). The formatting
carries the meaning, so the dot separators are dropped entirely. This makes the
`Tooltip` primitive take `heading` / `gloss` / `rows`, not a single string.

**`Quality · CRF` → `Quality`.** That label held the panel's only `·`. It is
renamed to plain **Quality**; the CRF detail moves into the tooltip (heading
"Quality", gloss "Compression level (CRF)…"). All five group labels are now
single clean words with no separators.

**CRF segments ordered low→high quality (28 · 23 · 18).** Every other group runs
smallest/lowest on the **left** to highest on the **right** (480→ORIG, 24→ORIG,
96K→…). CRF was inverted (18 = best quality sat on the left). The segments are
reordered so the **left is lowest quality (CRF 28), the right is best (CRF 18)**,
matching the rest of the panel. Values are unchanged, so presets still match by
value, not position; only the display order moved. The tooltip rows follow the
same left→right order.

**Format chip moved to the meta line.** The extension chip (`[MP4]`) moved off
the row's first line (it sat beside the outcome badge and remove ✕) down to the
**meta line**, where the file's general data lives (`[MP4] 248 MB · 12:34 ·
1920×1080`). Line 1 is now just filename + outcome badge + remove, and the
dot-separated meta line (a left-panel data line, not chrome) keeps its `·`
separators.

> **Revised by [ADR-0024](0024-frontend-action-model-revision.md):** the extension is
> **de-chipped** — plain mono, uppercase, dot-separated with the rest
> (`MOV · 248 MB · 12:34 · 1920×1080`). Outline boxes are reserved for the **Custom**
> indicator; filled boxes for status badges; plain mono for all data values.
