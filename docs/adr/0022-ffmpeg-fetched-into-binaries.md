# ADR-0022: FFmpeg sidecars are *fetched into* `binaries/`, not committed into it

- Status: Accepted
- Date: 2026-06-07
- Amends: [ADR-0005](0005-prebuilt-ffmpeg-binaries.md)

## Context

[ADR-0005](0005-prebuilt-ffmpeg-binaries.md) decided to vendor prebuilt static `ffmpeg.exe` and
`ffprobe.exe` into `src-tauri/binaries/`, pinned by version and SHA-256. It said to "drop them
into" that directory but left open *how the bytes are obtained and whether they live in git*. The
binaries are ~135 MB each; committing them directly bloats the repository, and the
LFS-vs-commit storage decision is explicitly deferred to R2 (release/CI), not this slice.

## Decision

The binaries are **fetched into** `src-tauri/binaries/` by a reproducible script, not committed
into it:

- A committed manifest, `src-tauri/binaries/ffmpeg-manifest.json`, pins the upstream source
  (BtbN/FFmpeg-Builds, an immutable dated `autobuild-*` release tag), the exact asset, the
  FFmpeg version, the full GPL build configuration, and the SHA-256 of both the archive and each
  extracted executable.
- `npm run fetch:ffmpeg` (→ `scripts/fetch-ffmpeg.mjs`, dependency-free Node) downloads the
  pinned archive, **verifies every SHA-256, and fails loudly on any mismatch**, then writes the
  two executables under Tauri's `<name>-<target-triple>.exe` sidecar convention
  (`ffmpeg-x86_64-pc-windows-msvc.exe`, `ffprobe-x86_64-pc-windows-msvc.exe`).
- The binary **bytes are gitignored**; only the manifest is committed. The git-storage decision
  (Git LFS vs commit) remains deferred to R2.
- `tauri.conf.json` declares the two binaries as `externalBin`, which is the Tauri mechanism
  that recognizes and bundles them as sidecars. **No Tauri shell plugin or shell capability is
  added** — per [ADR-0004](0004-ffmpeg-sidecar-not-ffi.md) the sidecars are spawned directly via
  `std::process::Command`, so the shell plugin is deliberately not in the dependency graph.
  (The S0 issue prose mentioned wiring a "Windows shell capability"; that conflicts with the
  locked ADR-0004 decision, so it is intentionally not done. The issue's acceptance criteria ask
  only for `externalBin`, which is satisfied.)

## Consequences

- The repository stays lean; a fresh checkout runs `npm run fetch:ffmpeg` once to populate
  `binaries/`. CI (R2) runs the same script, so the verified-checksum guarantee is identical
  locally and in CI.
- The pinned-and-verified contract from ADR-0005 is strengthened, not weakened: tampering or an
  upstream change is caught by a checksum mismatch before any binary is written.
- The binaries still land exactly where [ADR-0017](0017-rust-module-structure.md) and the sidecar
  spawn path expect them.
- GPL compliance ([ADR-0003](0003-gplv3-license.md)): the exact upstream source, version, and
  build configuration are recorded in the committed manifest.

## Alternatives considered

- **Commit the bytes directly (plain git)** — rejected for now: ~270 MB of binaries bloat every
  clone. Revisited in R2.
- **Git LFS** — a candidate for R2; not decided here.
- **Build from source** — already rejected for v1 by ADR-0005.
