# Minimal FFmpeg build pipeline + the tracer gate

Status: complete

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

## Comments

**2026-06-10 (agent, on completion):** Both halves landed; **the tracer gate is green**
(`cargo test --test tracer` → in-process h264-aac.mp4 → mp4 with a video stream), so
issues 03/04 are unblocked. Pins: FFmpeg 7.1.4, x264 stable@b35605ac, dav1d 1.5.3,
NASM 2.16.03 (pinned as a manifest *tool* and auto-fetched). All acceptance criteria
verified: `-Clean` end-to-end run, stale-guard negative test (cargo build fails loudly
on a stamp mismatch), full suite green (41 tests, sidecar paths untouched), manifest
records the resolved linking facts, no MinGW objects (cl.exe + nasm only).

Empirical findings, all recorded in the manifest `linking`/notes sections:

- **Discovery resolved to env vars, not pkg-config** — rusty_ffmpeg's Windows build
  path only supports `FFMPEG_LIBS_DIR`/`FFMPEG_INCLUDE_DIR` (no pkg-config branch
  exists on Windows). Set repo-relatively in the committed `.cargo/config.toml`.
  Bindgen needs libclang at every cargo build: `LIBCLANG_PATH` user env var (pip
  libclang wheel).
- **rusty_ffmpeg links all seven libav libs unconditionally**, including avdevice →
  FFmpeg builds a 29 KB avdevice stub (zero devices enabled); deviation from
  ADR-0027's "avdevice compiled out" letter, documented in the manifest.
- **Static-lib naming**: everything normalized to `<name>.lib`, except `libx264.lib`
  keeps its name because FFmpeg's configure hardcodes `-lx264` → `libx264.lib` for
  msvc. build.rs links `static=libx264`, `static=dav1d`, plus `bcrypt` (enabled by a
  plain system check regardless of `--disable-autodetect`; av_get_random_seed).
- **Link directives ride the lib target**: the tracer must `extern crate ruckel_lib`
  (which also makes the gate prove libav+Tauri link into one binary).
- **x264 tarball comes from the GitHub mirror** — videolan's GitLab archive endpoint
  sits behind an Anubis proof-of-work wall; commit hash + SHA-256 still pin content.
- MSYS2 gotchas baked into the scripts: clear inherited `ORIGINAL_PATH` (else
  `inherit` mode silently drops the vcvars PATH), and front-run PATH with the MSVC
  bin dir so coreutils `/usr/bin/link` never shadows link.exe.
- Static libs total ~28 MB (debug-stripped, -MD, 8-bit/420-only x264) — final exe
  cost will land in ADR-0028's 10–25 MB estimate after dead-code elimination.
