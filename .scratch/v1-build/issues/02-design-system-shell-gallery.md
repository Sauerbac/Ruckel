# F1 — Design system + primitives + shell + gallery route

Status: ready-for-agent

## What to build

The contract-free visual foundation everything later sits on. Three parts:

1. **Design system tokens.** Apply the ADR-0018 palette as Tailwind v4 `@theme` tokens in
   `src/app.css` (base `#0D1117`, surface `#161B22`, accent teal `#14B8A6`, text/border/danger per
   the locked table). Bundle **Inter** (UI) and **JetBrains Mono** (data: filenames, paths, sizes,
   numbers) as **local font files** — no CDN; this is an offline desktop app.

2. **Primitives.** Neo-brutalist building blocks at **2px** radius everywhere: button (incl. the
   teal primary action), panel/surface, segmented control. Dark-mode only.

3. **Static shell.** Custom title bar (`decorations: false`, drag region, close + minimize only —
   no maximize), the **860×560 fixed, non-resizable** window (update `tauri.conf.json` from the
   current 800×600 resizable default), the two-panel split (left files / right options), and the
   full-width status/action bar frame spanning both panels. Empty states only — no data-bound
   components yet (those are F2).

4. **Gallery route.** A dev-only harness: in `main.ts`, mount `Gallery.svelte` instead of `App`
   when `import.meta.env.DEV && location.search` includes `gallery`. F1's gallery shows the
   primitives and the empty shell. F2 extends it.

## Acceptance criteria

- [ ] ADR-0018 palette + Inter/JetBrains Mono available as Tailwind v4 `@theme` tokens; fonts bundled locally
- [ ] Button, panel, and segmented-control primitives render at 2px radius in the locked palette
- [ ] Window is 860×560 fixed/non-resizable with a custom title bar (close + minimize only)
- [ ] Two-panel split + full-width status bar frame render with empty states
- [ ] `?gallery` mounts a gallery showing primitives + empty shell; screenshot matches ADR-0018/0019/0020 (palette hex, 2px radius, Inter vs JetBrains Mono)

## Blocked by

None - can start immediately. (Sequenced after S0 so FFmpeg is smoke-testable early, but has no hard dependency on it.)
