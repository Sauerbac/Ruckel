# Matrix integration tests — drift gate + per-fixture transcode assertions

Status: complete

## Parent

`.scratch/v2-single-binary/PRD.md` (step 7). Decision records: ADR-0028 (the matrix is the
contract these tests enforce), testing strategy in the PRD.

## What to build

Two test layers that make the ADR-0028 matrix mechanically enforced rather than aspirational:

1. **Drift test:** enumerate the enabled decoders and demuxers from the committed build
   manifest (`ffmpeg-build-manifest.json`) and assert that each is covered by a fixture in
   the corpus **or** an entry in the allowlisted-gap file (issue 01). Manifest and corpus
   can never drift apart silently — forgetting a fixture when growing the matrix, or
   stranding a fixture when shrinking it, fails the suite.

2. **Per-fixture integration tests:** for every fixture, probe + transcode **in-process**
   and assert the output structurally:
   - opens and demuxes as mp4
   - video is h264 high profile, yuv420p, even dimensions
   - audio is aac (when the fixture has audio and options include it)
   - duration within tolerance of the source
   - **`moov` precedes `mdat`** — faststart actually verified, byte-level, something the
     v1 suite never checked
   - the rotated fixture's output has swapped dimensions (autorotation proven)

## Acceptance criteria

- [ ] Drift test parses the committed manifest (not a hardcoded list) and fails when a
      decoder/demuxer lacks both a fixture and an allowlist entry — verified by a
      deliberate red run during development
- [ ] Every corpus fixture transcodes in-process with all structural assertions passing,
      including the faststart byte-order check and the rotation check
- [ ] The whole thing runs under `cargo test --manifest-path src-tauri/Cargo.toml` with no
      sidecar or network dependency
- [ ] Runtime stays reasonable (tiny fixtures; the suite is expected to stay well under a
      minute)

## Blocked by

- `01-fixture-corpus.md` (the corpus)
- `03-probe-swap.md` (in-process probe)
- `04-encoder-swap.md` (in-process encode)

## Notes

- The human PowerPoint sanity check (drop one output into actual PowerPoint) stays manual,
  once per release — explicitly not automated here.
- If a fixture exposes a missing parser/BSF (the "matrix tests are the arbiter" clause in
  ADR-0028), the fix is a manifest edit + lib rebuild — record it in the manifest, don't
  special-case the test.

## Comments

**2026-06-11 (agent, on completion):** `tests/matrix.rs` lands both layers; full suite
green at 54 tests, matrix sweep itself ~1.3 s (well under the runtime budget), no sidecar
or network. The arbiter clause earned its keep: the sweep surfaced and forced fixes for
one matrix gap, two real transcode-loop bugs, and two build-pipeline traps.

Drift gate:
- Bidirectional, parsed from the committed manifest (decoder/demuxer enumerations) and the
  corpus filenames (issue 01 convention), with `allowlisted-gaps.json` honored. Name maps:
  `libdav1d`↔`av1`, `flv`↔`flv1`; container-extension→demuxer table for the demuxer side.
- Core checks are pure functions; the "deliberate red run" acceptance is satisfied by two
  synthetic-input tests (`drift_gate_trips_on_*`) rather than editing the manifest (any
  edit invalidates the lib stamp). The stranded-fixture direction had already gone red for
  real: **flv1-mp3.flv was stranded** (manifest had `h263`, but FLV1/Sorenson needs the
  distinct `flv` decoder). Arbiter decision (user-approved): `flv` added to the manifest,
  libs rebuilt. The issue-03 probe-parity exception for flv1 is gone — exact parity
  corpus-wide.
- New third layer beyond the spec: `runtime_lib_matches_manifest_configuration` asserts
  `avcodec_configuration()` of the *linked* lib contains the manifest's decoder/demuxer/
  parser/filter flags — catching stale libs behind a fresh stamp (see traps below).

Per-fixture sweep (32 fixtures): h264-high/yuv420p/even dims, aac iff the source has
audio, duration within 0.5 s (sources probing <0.2 s — the raw-elementary mpegvideo
estimate — treated as unknown), byte-level moov<mdat faststart, and the rotated fixture's
**dimension swap proven**: `h264-aac-rotated.mp4` regenerated **non-square (96×64)** via
`generate-fixtures.mjs` (user-approved corpus change), output is 64×96.

Real bugs the sweep caught (fixed forward in `runner.rs`):
1. **Silent audio drop** — `AudioPipe` setup failure degraded to video-only output.
   `mjpeg-pcm_s16le.avi` lost its audio: AVI carries only a channel count, so the layout
   order is UNSPEC, which aac/swresample reject. Now: UNSPEC normalizes to the default
   layout for the channel count (CLI parity), and any audio-setup failure on a source
   with audio **fails the job** instead of silently stripping audio.
2. **Nonzero container start time** — VOB (~0.44 s) and TS (~1.4 s) inputs produced
   outputs with inflated duration and desynced audio (video PTS passed through unshifted;
   synthesized audio PTS started at 0). Now: all timestamps shift to a zero-based
   timeline (`ifmt.start_time` subtracted; audio sample clock anchored to the first
   decoded frame's real position), matching the CLI. Also fixes progress percent for
   such files.

Build-pipeline traps found while landing the flv rebuild (both fixed in `build.ps1`):
- **`Rename-StaticLibs` kept stale libs on incremental re-runs** — when `<name>.lib`
  already existed it deleted the freshly built `lib<name>.a` and restamped anyway,
  defeating the stale guard. Now the fresh lib always replaces the target.
- **rustc bundles the static libs into `librusty_ffmpeg.rlib`** (`+bundle` default), so
  even with new `.lib` files on disk, every later link reuses the rlib's embedded old
  objects — `cargo clean -p ruckel` is not enough. `build.ps1` now ends with
  `cargo clean -p rusty_ffmpeg`; the runtime-configuration test is the backstop.
  Documented in `scripts/build-ffmpeg/README.md`.
