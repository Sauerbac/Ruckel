# ADR-0030: The product artifact is the bare `ruckel.exe`

- Status: Accepted
- Date: 2026-06-09

## Context

"Single copyable binary" is a packaging contract, not just a linking one. With FFmpeg linked
in ([ADR-0027](0027-ffmpeg-linked-in-process.md)) and `externalBin` gone, the plain
`cargo tauri build` output is genuinely self-contained: frontend assets are already embedded
by Tauri, FFmpeg lives in the executable. One dependency remains: Tauri renders via the
**WebView2 runtime**, a system component preinstalled on Windows 11 and any updated Windows 10.

## Decision

- The shipped artifact is **`ruckel.exe` itself** — "copy it to a USB stick, run it on the
  presentation laptop" is the user story this whole architecture serves. No installer
  ceremony; release = the exe.
- **WebView2-evergreen is assumed present**, exactly as v1 already assumed it. It is the
  single accepted system dependency. On a machine without it, the exe does not start; any
  machine realistically running PowerPoint has it.
- Bundling config is trimmed accordingly: `externalBin` removed (with the sidecar retirement),
  bundling disabled outright (`bundle.active: false` — the plain `cargo tauri build` release
  exe is the artifact; `bundle.icon` still embeds the exe icon via tauri-build), and the
  stray `bundle.android` key deleted.

## Alternatives considered

- **Fixed-version WebView2 embedded** — rejected: ~150 MB shipped as *extracted files beside
  the exe*, which structurally defeats the single-file contract.
- **NSIS installer as a secondary artifact** (bootstraps WebView2 if missing) — not pursued;
  can be revisited without touching the architecture if distribution needs ever change.
