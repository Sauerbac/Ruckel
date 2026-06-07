# S0 — Fetch FFmpeg/ffprobe sidecars + pin manifest

Status: complete

## What to build

Get real, dependency-free static `ffmpeg.exe` and `ffprobe.exe` (Windows x86_64, from BtbN's
FFmpeg-Builds or gyan.dev) into `src-tauri/binaries/`, renamed to Tauri's target-triple sidecar
convention (`ffmpeg-x86_64-pc-windows-msvc.exe`, `ffprobe-x86_64-pc-windows-msvc.exe`). Provide a
reproducible fetch path (a script invokable via an npm script) that downloads the **pinned**
version, verifies its SHA-256 against a committed manifest, and places the renamed binaries.

The binary bytes are **gitignored for now** — only the pin manifest (upstream source, version,
SHA-256, build config) is committed. The git-storage decision (LFS vs commit) is deferred to R2.

Wire `externalBin` in `tauri.conf.json` and the Windows shell capability so the binaries are
recognized as sidecars (spawning them is exercised later in S2, not here).

Record the ADR-0005 amendment: binaries are *fetched-into* rather than *committed-into*
`binaries/` (amend ADR-0005 or add ADR-0022).

## Acceptance criteria

- [x] `ffmpeg -version` and `ffprobe -version` run from the fetched binaries
- [x] A committed manifest pins version + SHA-256 + upstream source/build config; fetch verifies the checksum and fails loudly on mismatch
- [x] Binaries are gitignored; `tauri.conf.json` declares them as `externalBin`
- [x] ADR-0005 amended (or ADR-0022 added) recording the fetch-into-binaries decision

## Blocked by

None - can start immediately.

## Comments

### Implemented — 2026-06-07

**What landed**

- `src-tauri/binaries/ffmpeg-manifest.json` (committed) — pins BtbN/FFmpeg-Builds release
  `autobuild-2026-06-06-13-18`, asset `ffmpeg-n7.1.4-9-gc06af95f12-win64-gpl-7.1.zip`
  (FFmpeg **n7.1.4**, static `win64-gpl`, dependency-free, `--enable-libx264`). Records the
  archive SHA-256, each executable's SHA-256, the target triple, and the full GPL build
  configuration line.
- `scripts/fetch-ffmpeg.mjs` + `npm run fetch:ffmpeg` — dependency-free Node. Downloads the
  pinned archive, verifies the archive checksum, extracts `ffmpeg.exe`/`ffprobe.exe` with a
  built-in (zlib-only) zip reader, verifies each executable's checksum, and writes them as
  `ffmpeg-x86_64-pc-windows-msvc.exe` / `ffprobe-x86_64-pc-windows-msvc.exe`. Idempotent
  (skips when outputs already match), caches the archive, writes atomically (tmp+rename), and
  **exits non-zero on any checksum mismatch without writing a binary**.
- `tauri.conf.json` → `bundle.externalBin: ["binaries/ffmpeg", "binaries/ffprobe"]`.
- `.gitignore` ignores `src-tauri/binaries/*` except `ffmpeg-manifest.json`.
- `docs/adr/0022-ffmpeg-fetched-into-binaries.md` added; ADR-0005 cross-linked via "Amended by".

**Verification run**

- `npm run fetch:ffmpeg` from a clean dir → both binaries written & verified; re-run is a no-op.
- `ffmpeg -version` / `ffprobe -version` → `n7.1.4-9-gc06af95f12-20260606`, libx264 enabled.
- Tampering an expected SHA in the manifest → script aborts (exit 1), writes nothing.

**⚠ Deviation needing sign-off:** the issue prose asked to wire "the Windows shell capability."
That conflicts with **ADR-0004**, which explicitly rejects the Tauri shell plugin in favour of
spawning sidecars via `std::process::Command`. I followed the locked ADR: `externalBin` is what
makes Tauri recognize/bundle the sidecars, and no shell plugin/capability was added. The issue's
acceptance criteria only require `externalBin`, so they are met. Rationale recorded in ADR-0022.

Committed (binaries excluded by .gitignore per ADR-0022; bytes fetched via `npm run fetch:ffmpeg`).
