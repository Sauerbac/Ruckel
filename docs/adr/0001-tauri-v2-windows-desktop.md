# ADR-0001: Tauri v2 as the app framework (Windows-only)

- Status: Accepted
- Date: 2026-06-07
- Supersedes: the original "Tauri v1" choice from the initial planning session

## Context

Ruckel is a small desktop utility: a window, drag-and-drop, a handful of controls, and a
Rust backend that drives FFmpeg. It targets Windows only. We need a framework that gives a
native window, a web-tech frontend, and a Rust process layer with full control over spawning
and cancelling child processes.

The original planning session locked **Tauri v1**. By mid-2026 Tauri v1 is in
maintenance/EOL territory; Tauri v2 has been stable since late 2024 and is the version that
will keep receiving fixes.

## Decision

Build on **Tauri v2**, Windows-only for v1 of the app.

## Consequences

- Longer support runway; no fresh build on a deprecated major version.
- Cleaner, better-documented sidecar/shell APIs and the capabilities/permissions security
  model. Sidecars are declared via `externalBin` in `tauri.conf.json`.
- We still spawn FFmpeg directly from Rust via `std::process::Command` (see
  [ADR-0004](0004-ffmpeg-sidecar-not-ffi.md)), so the v2 IPC/event API changes barely touch
  the backend.
- Frontend uses the v2 plugin packages (e.g. `@tauri-apps/plugin-*`) where needed.

## Alternatives considered

- **Tauri v1** — rejected: deprecated for a new project; v1→v2 migration later costs more
  than starting on v2 now.
- **Electron** — heavier runtime, no Rust-native process control story, larger installer.
