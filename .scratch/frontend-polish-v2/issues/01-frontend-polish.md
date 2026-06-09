# Frontend polish v2 — settings tooltips, layout stability, list affordances

Status: complete

Eight frontend refinements agreed in a grilling session. Durable rationale and
the decision record live in **ADR-0023** (`docs/adr/0023-frontend-polish-v2.md`),
which extends ADR-0018/0020/0021. Everything must obey ADR-0018: flat, sharp
corners, 1.5px ink borders, **no shadows / gradients / transitions** (the one
motion is the converting blink), every control always visible (hover inverts fill
instantly).

## Files in play

- `src/lib/components/OptionsPanel.svelte` — group headers, Custom badge, presets
- `src/lib/components/SegmentedControl.svelte` — segment width
- `src/lib/components/FileRow.svelte` — badges, done line, remove button
- `src/lib/shell/Shell.svelte` — drop zone, file list overflow, remove wiring
- `src/lib/ipc-client.ts` — add the file-dialog wrapper
- `src/app.css` — any shared utility (e.g. scrollbar-hide)
- `src-tauri/Cargo.toml`, `src-tauri/capabilities/*.json`, `package.json` — dialog plugin
- New: a small `Tooltip.svelte` (top-level popover) and an info-icon usage

## What to build

### A. Info tooltips + uniform group headers (#1, #2)
Give every option group in `OptionsPanel` a **fixed-height header row**:
`[ⓘ] LABEL` on the left, the existing **Custom** tag on the right (Preset group
only). Fixed height so the Custom badge and icon never reflow the panel.

The `ⓘ` opens a **styled popover** (ink border, paper bg, mono), **rendered at
top level** (portal/`fixed`) so the `aside`'s `overflow-auto` can't clip it,
opening **left**. Trigger on **hover + keyboard focus**, shown/hidden
**instantly** (no fade). One tooltip per group; draft copy (tweak freely):

- **Preset** — "One-tap bundles. PRESENTATION: balanced for slides (original
  size, CRF 23). HIGH: best quality, larger file (CRF 18). COMPACT: smallest file
  (720p, CRF 28, 30 fps)."
- **Resolution** — "Output frame height. 480 / 720 / 1080 downscale; ORIG keeps
  the source size. Smaller = smaller file."
- **Quality · CRF** — "Compression level. 18 = best quality, large file · 23 =
  balanced · 28 = small, softer."
- **Framerate** — "Frames per second. 24 / 30 / 60 cap the rate; ORIG keeps the
  source. Lower = smaller file."
- **Audio** — "Audio bitrate. 96K = smallest · 128K = standard · 192K = best ·
  NONE strips audio."

### B. Row layout stability (#3, #4)
Keep the progress bar as-is. Accept the one grow when convert starts. Make the
**done** phase render the output path **on the same line the bar used** (no
second shift). Shrink the **Done/Error badges to ext-chip size** (`py-px`, match
`[MP4]`) so line 1 never nudges.

### C. Equal-width segments (#6)
`SegmentedControl`: make the wrapper `w-full` (was `inline-flex`) and each
segment `flex-1`, so all four option groups stretch flush to the panel width like
the presets.

### D. Fading list (#5)
In `Shell.svelte`'s file-list scroll container: hide the scrollbar
(`scrollbar-width: none` + `::-webkit-scrollbar { display:none }`, e.g. a
utility in `app.css`). Apply a `mask-image` linear-gradient (~24px) at top and
bottom. Add a scroll listener that drops the top stop when `scrollTop === 0` and
the bottom stop when scrolled to the end (and shows neither when not scrollable).

### E. Click-to-browse (#7)
Add `tauri-plugin-dialog`: npm `@tauri-apps/plugin-dialog`, Cargo
`tauri-plugin-dialog`, register in the builder, and grant `dialog:allow-open` in
the capability file. Add a wrapper in `ipc-client.ts` that opens a
**multi-select** Open dialog filtered to the video extensions (mirror
`src-tauri/src/preflight/scanner.rs` `VIDEO_EXTENSIONS`) plus an "All files"
entry, returning paths. Make the empty drop zone a **button** (pointer cursor,
hover affordance, Enter/Space) that calls it and feeds results into the existing
`handleDrop()`. Wire only while the zone is shown. Folders stay drag-only.

### F. Remove (✕) button (#8)
In `FileRow.svelte`, add a **persistent square ✕** at the far right of line 1,
after the chips: ext-chip sized (~18px, `text-[10px]`, `leading-none`),
`border border-line` + `text-muted`, hover `hover:bg-danger hover:text-paper`,
sharp/flat, `cursor-default`, focus-visible accent outline, mono glyph. Render
**only when `phase !== 'converting'`**. Emit a `remove` event; `Shell.svelte`
filters the row from `rows`. Last row removed → `reset()`. Removed while `done` →
recompute `succeeded`/`failed` from remaining rows.

## Acceptance criteria

- [ ] Each option group shows an `ⓘ`; hover or focus opens a flat popover that is
      not clipped by the panel and decodes that group's options; closes instantly
- [ ] Switching away from a preset shows **Custom** with **zero** panel reflow;
      the info icons add no reflow either
- [ ] Starting conversion grows each row exactly once; **finishing causes no
      further shift** (output path occupies the bar's line); Done/Error badges
      don't change line-1 height
- [ ] All four option groups are full-width with equal segments, flush like the
      presets
- [ ] The file list has **no visible scrollbar**; it fades at top and bottom; the
      top fade vanishes at the top, the bottom fade at the bottom, neither shows
      when the list fits
- [ ] Clicking the empty drop zone opens a multi-select video-filtered dialog;
      chosen files load exactly as a drag-drop would; keyboard-activatable
- [ ] Each removable row has a quiet ✕ that turns danger on hover, removes the
      row, is **absent during converting**, returns to the drop zone when the
      last row is removed, and keeps the done summary truthful
- [ ] `cargo --manifest-path src-tauri/Cargo.toml build` and the frontend build
      both pass; the Gallery still renders every component state

## Notes

- Run `cargo` with `--manifest-path src-tauri/Cargo.toml` (see CLAUDE.md).
- The Gallery (`src/Gallery.svelte`) renders components outside Tauri — guard any
  new dialog/IPC call behind `inTauri` so the gallery still works.
- Tooltip + remove button are new component states worth adding to the Gallery.
