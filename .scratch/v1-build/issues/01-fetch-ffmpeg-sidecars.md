# S0 — Fetch FFmpeg/ffprobe sidecars + pin manifest

Status: ready-for-agent

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

- [ ] `ffmpeg -version` and `ffprobe -version` run from the fetched binaries
- [ ] A committed manifest pins version + SHA-256 + upstream source/build config; fetch verifies the checksum and fails loudly on mismatch
- [ ] Binaries are gitignored; `tauri.conf.json` declares them as `externalBin`
- [ ] ADR-0005 amended (or ADR-0022 added) recording the fetch-into-binaries decision

## Blocked by

None - can start immediately.
