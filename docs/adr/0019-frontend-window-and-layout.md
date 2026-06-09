# ADR-0019: Frontend window and layout

- Status: Accepted
- Date: 2026-06-08
- Revises: the 2026-06-07 draft — the layout split is unchanged; the title-bar and
  status-bar *styling* is updated to the ADR-0018 v2 chrome language (solid-ink
  blocks).

## Context

A focused single-task utility benefits from a fixed, predictable frame rather than a
resizable, responsive layout. The chrome should read as flat solid-ink blocks with
hard dividers (ADR-0018), adapted to a one-button converter — we adopt the handoff's
chrome *grammar* but not its always-connected, data-dense *semantics* (no decorative
window-dots, no `//` title separator, no fake connection meta).

## Decision

**Window:** 860×560px, fixed, no resize. Chosen to fit 1366×768 screens and 1080p
screens at 150% DPI scaling.

**Title bar (`decorations: false` in Tauri, custom-drawn):** a **solid-ink block**,
`--paper` text, sharp corners. Left: one `--accent` brand square + `RUCKEL` in mono
uppercase (Space Grotesk wordmark acceptable; mono is fine too). `data-tauri-drag-region`
for drag-to-move. Right: **real** Close and Minimize controls only — sharp squares,
hover-invert — no maximize (meaningless at fixed size). No decorative dots or meta.

**Status / action bar:** a **solid-ink block** spanning both panels. A green `●`
pulse appears here only while encoding.

> **Revised by [ADR-0024](0024-frontend-action-model-revision.md):** the bar is now
> **status-only** — it carries live state and the full-width progress fill, but **no
> buttons**. The primary action (Convert/Cancel) moved to a pinned right-rail footer.

**Layout:** two-panel split between title bar and status bar —

```
┌─────────────────────────────────────────────┐
│  Solid-ink title bar  ■ RUCKEL        ─  ✕  │
├──────────────────────┬──────────────────────┤
│  Left panel          │  Right panel         │
│  (file drop + list)  │  (options)           │
├──────────────────────┴──────────────────────┤
│  Solid-ink status / action bar              │
└─────────────────────────────────────────────┘
```

Outer app frame is `1.5px solid` ink; the panel divider is `1.5px solid` ink.

## Consequences

- No responsive/resize logic to build or test.
- Fixed dimensions let the two-panel proportions be tuned once.
- The custom title bar is required because Tauri decorations are disabled
  ([ADR-0001](0001-tauri-v2-windows-desktop.md)); it now reads as a flat ink chrome
  block rather than a subtle web header.
