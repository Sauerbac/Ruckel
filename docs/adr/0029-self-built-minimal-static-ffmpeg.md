# ADR-0029: Self-built minimal static FFmpeg, manifest-pinned, built on the dev machine

- Status: Accepted
- Date: 2026-06-09
- Supersedes: [ADR-0022](0022-ffmpeg-fetched-into-binaries.md), and the "prebuilt binaries"
  decision of [ADR-0005](0005-prebuilt-ffmpeg-binaries.md)

## Context

ADR-0005 rejected building FFmpeg from source *for v1*; v2's single-binary goal
([ADR-0027](0027-ffmpeg-linked-in-process.md)) requires static libraries containing only the
curated matrix ([ADR-0028](0028-curated-decode-matrix.md)) — no prebuilt distribution offers
that. We become our own upstream. The Rust toolchain is `x86_64-pc-windows-msvc` (Tauri
standard; not switching), so the libs must link cleanly with MSVC's `link.exe`.

## Decision

Build **FFmpeg + x264 + dav1d from pinned sources** as MSVC-native static libraries:

- **Toolchain:** an MSYS2 shell driving **`cl.exe`** — FFmpeg's `--toolchain=msvc`, x264's
  `CC=cl` configure path, dav1d via meson/ninja with native MSVC. One CRT family end to end;
  no MinGW objects ever enter the MSVC link (that CRT/libgcc mixing was rejected outright).
  Required tools: MSYS2, VS Build Tools, meson, ninja, nasm.
- **Reproducibility contract (inherited from ADR-0022's philosophy):** a committed manifest
  (`src-tauri/ffmpeg-build-manifest.json`) pins the FFmpeg, x264, and dav1d source tarballs by
  version + SHA-256 **and the full configure flag set** — the flags *are* the ADR-0028 matrix,
  expressed mechanically. FFmpeg is pinned to the **n7.1.x stable series** (matching the v1
  sidecar's major, and rsmpeg's supported range).
- **Committed script, not a wiki page:** `scripts/build-ffmpeg/` runs the entire build from the
  manifest inside MSYS2. Output lands in a **gitignored** `src-tauri/ffmpeg-libs/`
  (static libs + headers + pkg-config files), tagged with the manifest hash.
- **Stale-build guard:** `build.rs` compares the manifest hash recorded in `ffmpeg-libs/`
  against the committed manifest and **fails loudly** on mismatch — a stale local build can
  never silently link.
- **Where it runs:** on the dev machine, for the foreseeable future. The manifest+script
  contract is deliberately CI-shaped so the build can later move to CI (publishing
  `ffmpeg-libs-<manifest-hash>` artifacts plus a verifying fetch script) **unchanged**.
  No fetch script exists until CI does.

## Consequences

- The manifest doubles as the GPL-compliance record (exact sources, versions, configuration —
  ADR-0003). dav1d is BSD-2 and needs its license text shipped alongside the GPL notices.
- ADR-0022's fetch machinery (`scripts/fetch-ffmpeg.mjs`, `binaries/ffmpeg-manifest.json`,
  `externalBin`) is retired at the end of the v2 migration, once nothing spawns sidecars.
- Bindings discovery (`rusty_ffmpeg` env vars — pkg-config path vs. explicit include/libs
  dirs) and the exact set of Windows system libs to link (`bcrypt`, `ole32`, …) are resolved
  empirically in the ADR-0027 tracer slice; the manifest/script record whatever the tracer
  proves out.
- Matrix changes (ADR-0028) are manifest edits: bump flags + hashes, rebuild, fixture it.

## Alternatives considered

- **MinGW-built `.a` linked into MSVC** (BtbN-style) — rejected: CRT/libgcc mixing is a
  permanent tax on every upgrade.
- **vcpkg's ffmpeg port** — rejected: feature bundles only, no per-decoder
  `--disable-everything` granularity; overlay-port hacks fight vcpkg's grain.
- **CI-built artifacts fetched locally** — deferred, not rejected: development is single-machine
  for now; the contract is already CI-shaped for when that changes.
