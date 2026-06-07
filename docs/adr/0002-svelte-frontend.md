# ADR-0002: Svelte for the frontend

- Status: Accepted
- Date: 2026-06-07

## Context

The UI is small and largely static in structure: a fixed-size window, a two-panel layout, a
file list, and a set of option controls. It needs reactive state (drop → pre-flight → encode
phases) but no heavy routing or large component ecosystem.

## Decision

Use **Svelte** as the frontend framework inside Tauri's webview.

## Consequences

- Minimal runtime overhead and small bundle — appropriate for a lightweight utility.
- Reactive stores map cleanly onto the phased file-row model (see
  [ADR-0020](0020-frontend-interaction-model.md)).
- Tauri event subscriptions ([ADR-0016](0016-tauri-command-and-event-contract.md)) drive
  Svelte stores directly.

## Alternatives considered

- **React** — larger runtime, more ceremony than this UI warrants.
- **Vanilla JS** — would re-implement reactivity by hand for the phased states.
