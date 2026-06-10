# Minimal FFmpeg build pipeline + the tracer gate

Status: ready-for-human

## Parent

`.scratch/v2-single-binary/PRD.md` (steps 3 + 4, deliberately combined — the build is only
provable by the tracer test). Decision records: ADR-0029 (build), ADR-0027 (rsmpeg + gate).

## What to build

Two halves that land together:

### A. The build pipeline (ADR-0029)

- A committed manifest (`src-tauri/ffmpeg-build-manifest.json`) pinning FFmpeg (n7.1.x
  series), x264, and dav1d source tarballs by version + SHA-256, plus the **full configure
  flag sets**. The FFmpeg flags are the ADR-0028 matrix expressed mechanically:
  `--disable-everything`-style baseline, GPL on, network/avdevice/programs/docs off,
  static only, `--toolchain=msvc`, and the exact demuxer/decoder/parser/filter/encoder/
  muxer/protocol enumerations from ADR-0028.
- A committed build script (`scripts/build-ffmpeg/`) that verifies tool prerequisites
  (MSYS2, VS Build Tools environment, meson, ninja, nasm), downloads + SHA-verifies the
  tarballs, builds dav1d → x264 → FFmpeg with `cl.exe` (one CRT family end to end, no
  MinGW objects), and installs static libs + headers + `.pc` files into a **gitignored**
  `src-tauri/ffmpeg-libs/` together with a manifest-hash stamp file.
- A `build.rs` stale-build guard: cargo build fails loudly if the stamp doesn't match the
  committed manifest.

### B. The tracer gate (ADR-0027)

Add the `rsmpeg` dependency wired to the libs via whatever discovery mechanism proves out
(pkg-config path vs. explicit include/libs env vars — resolve empirically, then record the
choice in the manifest notes and script). Then a standalone `cargo test` that links
against `ffmpeg-libs/` and transcodes the h264+aac/mp4 fixture **in-process** to a valid
mp4, asserting the output opens and has a video stream.

**This is the project's risk-retirement gate: issues 03 and 04 must not start until it is
green.** It retires the MSVC-link risk, the rsmpeg-discovery risk, and the
disable-everything risk in one place, before any production code is touched.

## Acceptance criteria

- [ ] `scripts/build-ffmpeg/` runs end-to-end on this machine and populates
      `src-tauri/ffmpeg-libs/` (gitignored) from a clean state, driven entirely by the
      committed manifest — no undocumented manual steps
- [ ] Editing the manifest without rebuilding makes `cargo build` fail loudly (stale guard)
- [ ] The tracer test passes on `x86_64-pc-windows-msvc`: in-process transcode of the h264
      fixture to a readable mp4
- [ ] The existing app still builds and all existing tests pass (interim state: libav
      linked in but production code still spawns sidecars — that's expected)
- [ ] The manifest records the resolved Windows system libs (`bcrypt`, …) and the rsmpeg
      discovery mechanism actually used
- [ ] No MinGW-built objects anywhere in the link

## Blocked by

- `01-fixture-corpus.md` (the tracer transcodes a committed fixture)

## Notes

- **Why HITL:** prerequisites (MSYS2, VS Build Tools, meson, ninja, nasm) may need
  installing or confirming on this machine — system-level changes a human should approve —
  and the gate itself deserves a human eyeball before 03/04 fan out from it.
- Known-finicky items to resolve empirically (list from the PRD): static-lib naming as
  emitted by msvc toolchain mode, CRT selection (`/MT` vs `/MD`) vs. Rust's msvc defaults,
  pkg-config availability from the cargo environment.
- Build time ~10–20 min is acceptable; this script runs rarely (manifest changes only).
