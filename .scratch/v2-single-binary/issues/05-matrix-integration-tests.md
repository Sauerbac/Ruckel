# Matrix integration tests — drift gate + per-fixture transcode assertions

Status: ready-for-agent

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
