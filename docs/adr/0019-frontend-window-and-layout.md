# ADR-0019: Frontend window and layout

- Status: Accepted
- Date: 2026-06-07

## Context

A focused single-task utility benefits from a fixed, predictable frame rather than a resizable,
responsive layout.

## Decision

**Window:** 860×560px, fixed, no resize. Chosen to fit 1366×768 screens and 1080p screens at
150% DPI scaling.

**Title bar:** custom-drawn in Svelte (`decorations: false` in Tauri). Dark background, app
name in Inter, `data-tauri-drag-region` for drag-to-move. Close and minimize only — no maximize
(meaningless at fixed size).

**Layout:** two-panel split between title bar and status bar —

```
┌─────────────────────────────────────────────┐
│  Custom title bar                           │
├──────────────────────┬──────────────────────┤
│  Left panel          │  Right panel         │
│  (file drop + list)  │  (options)           │
├──────────────────────┴──────────────────────┤
│  Full-width status / action bar             │
└─────────────────────────────────────────────┘
```

## Consequences

- No responsive/resize logic to build or test.
- Fixed dimensions let the two-panel proportions be tuned once.
- The custom title bar is required because Tauri decorations are disabled
  ([ADR-0001](0001-tauri-v2-windows-desktop.md)).
