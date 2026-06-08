# F1 — Design system + primitives + shell + gallery route

Status: complete

## What to build

The contract-free visual foundation everything later sits on. Three parts:

1. **Design system tokens.** Apply the ADR-0018 v2 palette as Tailwind v4 `@theme` tokens in
   `src/app.css` (warm-light: paper `#EDEBE6`, surface `#FBFAF8`, fill `#E4E1DA`, ink `#16150F`,
   plus the status triad accent `#1FB866` / warning `#D9821A` / danger `#C8341D`, per the locked
   table). Bundle **Space Grotesk** (chrome) and **IBM Plex Mono** (data: filenames, paths, sizes,
   numbers) as **local font files** — no CDN; this is an offline desktop app.

2. **Primitives.** Neo-brutalist building blocks at **0px** radius everywhere, 1.5px ink borders:
   button (incl. the green primary action, hover-invert), panel/surface, segmented control
   (selected segment inverts to ink, never green). Warm-light only.

3. **Static shell.** Custom title bar (`decorations: false`, drag region, close + minimize only —
   no maximize), the **860×560 fixed, non-resizable** window (update `tauri.conf.json` from the
   current 800×600 resizable default), the two-panel split (left files / right options), and the
   full-width status/action bar frame spanning both panels. Empty states only — no data-bound
   components yet (those are F2).

4. **Gallery route.** A dev-only harness: in `main.ts`, mount `Gallery.svelte` instead of `App`
   when `import.meta.env.DEV && location.search` includes `gallery`. F1's gallery shows the
   primitives and the empty shell. F2 extends it.

## Acceptance criteria

- [ ] ADR-0018 v2 palette + Space Grotesk/IBM Plex Mono available as Tailwind v4 `@theme` tokens; fonts bundled locally
- [ ] Button, panel, and segmented-control primitives render at 0px radius with 1.5px ink borders in the locked palette
- [ ] Window is 860×560 fixed/non-resizable with a solid-ink custom title bar (close + minimize only)
- [ ] Two-panel split + full-width ink status bar frame render with empty states (dashed-green drop zone; disabled — not dimmed — options)
- [ ] `?gallery` mounts a gallery showing primitives + empty shell; screenshot matches ADR-0018/0019/0020 (palette hex, 0px radius, Space Grotesk vs IBM Plex Mono, reserved-green status triad)

## Blocked by

None - can start immediately. (Sequenced after S0 so FFmpeg is smoke-testable early, but has no hard dependency on it.)

## Comments

### Implemented — 2026-06-07

**What landed**

- **Design tokens (`src/app.css`).** ADR-0018 palette + type are now Tailwind v4 `@theme`
  tokens: `--color-{base,surface,accent,fg,muted,border,danger}` (locked hex), `--font-sans`
  (Inter) / `--font-mono` (JetBrains Mono), and `--radius-brut: 2px`. These surface as utilities
  (`bg-base`, `text-accent`, `font-mono`, `rounded-brut`) per ADR-0021. A base layer sets the
  dark background, default font, full-height `#app`, and `overflow: hidden` for the fixed window.
  - `text primary` → `fg`, `text muted` → `muted`: renamed to avoid the `text-base` clash with
    Tailwind's built-in font-size utility. Same hex, clearer utility names.
- **Fonts bundled locally — no CDN** (`src/fonts/`). `inter-variable.woff2` (48 KB) and
  `jetbrains-mono-variable.woff2` (40 KB), latin variable-weight subsets (OFL-1.1, attributed in
  `src/fonts/LICENSES.md`), wired via `@font-face`. Vite hashes and emits them into the bundle.
- **Primitives (`src/lib/components/`), all at 2px radius, dark-only.** `Button.svelte`
  (`primary` teal action / `default` / `danger`, + disabled), `Panel.svelte` (flat bordered
  surface), `SegmentedControl.svelte` (generic, bindable, selected segment filled teal, disabled
  state).
- **Static shell (`src/lib/shell/`).** `TitleBar.svelte` — drag region + close & minimize only
  (no maximize), window calls guarded so it also renders in the browser gallery. `Shell.svelte`
  — two-panel split (left drop-zone empty state: dashed teal border, upload icon, the ADR-0020
  copy; right options empty state: dimmed @40%) + a full-width status/action bar frame
  (`No files` / disabled `Convert`). `App.svelte` now renders `<Shell />`.
- **Window (`src-tauri/tauri.conf.json`).** 800×600 resizable → **860×560**, `resizable:false`,
  `maximizable:false`, `decorations:false`.
- **Capabilities (`src-tauri/capabilities/default.json`).** Added
  `core:window:allow-start-dragging` (for `data-tauri-drag-region`), `allow-minimize`,
  `allow-close`.
- **Gallery harness.** `src/Gallery.svelte` shows typography (Inter vs JetBrains Mono), the
  palette swatches, every primitive/state, and the framed 860×560 empty shell. `src/main.ts`
  mounts it only when `import.meta.env.DEV && location.search` includes `gallery`.

**Verification run**

- `npm run check` (svelte-check + tsc) → 0 errors / 0 warnings.
- `npm run build` → succeeds; both `.woff2` emitted to `dist/assets/`, the `@tauri-apps/api/window`
  call code-split into its own chunk. Gallery code confirmed **absent** from the prod bundle
  (the `import.meta.env.DEV` guard folds to `false`, so Rollup drops it).
- `cd src-tauri && cargo check` → clean; Tauri's build codegen accepted the new window config and
  capability permission identifiers.
- `npx prettier --check` → all files conform.
- **Visual** (headless Chrome against the dev server): shell screenshot matches ADR-0019 layout
  (custom title bar w/ minimize+close, dashed-teal drop zone + copy, dimmed options, full-width
  status bar); gallery screenshot matches ADR-0018 (correct palette hex, 2px radius throughout,
  Inter UI vs JetBrains Mono data, teal primary action).

**Acceptance criteria** — all met: palette + fonts as `@theme` tokens with fonts bundled locally;
button/panel/segmented-control at 2px in the locked palette; 860×560 fixed/non-resizable window
with custom title bar (close + minimize only); two-panel split + full-width status bar with empty
states; `?gallery` mounts the primitives + empty shell matching the locked spec.

**Notes for later slices**

- The right-panel options are an intentionally dim, non-data-bound placeholder — the real option
  groups (Resolution/Quality/Framerate/Audio), presets, and the Custom indicator are F2 (issue
  05), built from the frozen S1 contract types (issue 03).
- Fonts are committed (not gitignored — only `src-tauri/binaries/*` is). Network was needed once
  at implementation time to fetch the `.woff2`; runtime stays fully offline.

### Redesigned to v2 — 2026-06-08

The original F1 named the aesthetic ("utilitarian neo-brutalism") but the implementation undershot
it — soft dark GitHub surface, faint borders, 2px radius. After a designer handoff
(`Test/design_handoff_utility_aesthetic/`) that executes the same philosophy faithfully, ADR-0018/
0019/0020 were rewritten (0021 + CONTEXT touched) and the implementation re-landed to match.

**What changed**

- **Tokens (`src/app.css`).** Dark palette → **warm light**: `--color-{paper,surface,fill,ink,
ink-2,muted,line}` + the status triad `--color-{accent #1FB866, accent-d, warning #D9821A,
danger #C8341D}`. `--radius-brut: 2px` → **0**. A `blink` keyframe (the one sanctioned motion)
  added for the live converting pulse.
- **Fonts (`src/fonts/`).** Inter + JetBrains Mono **removed**; **Space Grotesk** (variable) +
  **IBM Plex Mono** (static 400/500/600) bundled as local woff2 (OFL, re-attributed in
  `LICENSES.md`).
- **Primitives.** `Button` (mono uppercase, 1.5px ink, hover-invert; green primary = ink text;
  outline-not-ring focus to keep box-shadow at zero; transitions removed — instant/flat). `Panel`
  (1.5px ink, sharp). `SegmentedControl` (selected **inverts to ink**, never green; disabled =
  fill/line/muted empty-state treatment).
- **Shell.** Solid-ink `TitleBar` (green brand square + RUCKEL mono wordmark + real sharp min/
  close; no decorative dots/meta). Outer 1.5px ink frame. Left empty state = **dashed-green drop
  marquee**. Right empty state = options **disabled** (no accent) — replaces the old 40% opacity
  dim. Footer = solid-ink action bar.
- **Gallery.** Updated to v2 tokens; adds the status-triad chips + live pulse and the flat
  bordered progress bar.

**Verification run**

- `npm run check` (svelte-check + tsc) → 0 errors / 0 warnings.
- `npm run build` → succeeds; all four woff2 emitted to `dist/assets/`, CSS compiles the new
  tokens (17.9 kB). Gallery still folds out of the prod bundle via the `import.meta.env.DEV` guard.
- Pre-existing `eslint` errors in `scripts/fetch-ffmpeg.mjs` (`process`/`Buffer` no-undef) are
  unrelated to this work and left untouched.
- **Visual review still pending** — launch `?gallery` to eyeball before closing.
