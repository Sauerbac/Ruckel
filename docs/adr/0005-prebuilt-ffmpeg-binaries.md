# ADR-0005: Ship prebuilt static FFmpeg binaries for v1

- Status: Accepted
- Date: 2026-06-07
- Supersedes: the original "compile FFmpeg from source via MSYS2" plan from the initial
  planning and backend sessions

## Context

The original plan compiled FFmpeg + libx264 from vendored source tarballs using an
MSYS2/MINGW64 toolchain, a `build-ffmpeg.sh` script, a PowerShell launcher, and a GitHub
Actions Windows runner with `setup-msys2`. The stated benefits were static linking (no runtime
deps) and a minimal `--disable-everything` build.

Those benefits don't hold up: prebuilt **static** Windows builds (BtbN's FFmpeg-Builds,
gyan.dev) are already dependency-free *and* ship broad demuxer/decoder support with libx264
enabled. The minimal-build argument actively conflicts with Ruckel's "any video file" goal — a
trimmed build is more likely to silently fail on a legitimate input. The only real upside of
building from source is smaller binary size (~15–25 MB vs ~70–90 MB), which does not justify
owning an MSYS2 build pipeline for v1.

## Decision

For v1, **vendor prebuilt static `ffmpeg.exe` and `ffprobe.exe`** (BtbN or gyan.dev), pinned by
**version and SHA-256**. Drop them into `src-tauri/binaries/`. No build-from-source step, no
MSYS2, no compile step in CI.

## Consequences

- Dramatically less build surface area; CI just verifies checksums and bundles the sidecars.
- Broad input support comes for free, matching the "any video file" value proposition.
- GPL compliance: pin and record the exact upstream FFmpeg source + build config for the
  binary shipped (see [ADR-0003](0003-gplv3-license.md)).
- **Future paths kept open:** building from source and FFI remain valid later options —
  from-source if binary size ever becomes a hard constraint, FFI if in-process control is
  needed. Both are documented, neither is built for v1.

## Alternatives considered

- **Compile from source (original plan)** — rejected for v1: heavy pipeline for a benefit
  (minimal binary) we explicitly don't want. Retained as a future option.
