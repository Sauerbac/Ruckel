# Probe swap — in-process libavformat behind the unchanged ProbeResult

Status: complete

## Parent

`.scratch/v2-single-binary/PRD.md` (step 5). Decision records: ADR-0027 (the ffprobe split
dissolves), ADR-0031 (runtime model).

## What to build

Reimplement the probe (`probe.rs`) in-process: `avformat_open_input` +
`avformat_find_stream_info` via rsmpeg, reading duration, the first video stream's
dimensions, and video-stream presence directly from the structs. The **`ProbeResult`
struct does not change** — pre-flight, the ADR-0026 skip semantics, and the ADR-0011
extensionless fallback must not notice the swap. Probe failures (file won't open, no
streams) map to the same `Err(String)` surface pre-flight already treats as skip.

The ffprobe sidecar stops being spawned anywhere. Full machinery retirement (manifest
entries, `externalBin`, fetch script) stays with issue 06 — this issue only removes the
spawn path and the JSON-parsing code it fed.

This is **independently mergeable**: the interim hybrid (in-process probe + sidecar
encode) is an explicitly fine state per the PRD.

## Acceptance criteria

- [ ] `ProbeResult` is byte-identical (same fields, same types); no caller changes outside
      `probe.rs` internals
- [ ] Every fixture from issue 01 probes in-process with duration and dimensions matching
      the old sidecar's values (small tolerance on duration)
- [ ] The extensionless fixture passes the ADR-0011 fallback path; an audio-only and a
      corrupt input are skipped per ADR-0026, never abort the drop
- [ ] No code path spawns `ffprobe` any more (grep is clean outside retired/test fixtures)
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml` green; the app converts end-to-end
      in the hybrid state (sidecar encode still in place)

## Blocked by

- `02-ffmpeg-build-and-tracer-gate.md` (the gate must be green)

## Notes

- Probing runs on a worker thread as today's blocking call does — keep the command shape;
  this issue changes the inside of `probe()`, nothing else.
- Duration source: format-level duration (`AVFormatContext.duration`), matching what
  `-show_format` reported; fall back to 0.0 exactly as the current parser does.

## Comments

**2026-06-10 (agent, on completion):** `probe()` now reads the file in-process via
`AVFormatContextInput::open` (which runs `avformat_find_stream_info`), pulling duration
from `AVFormatContext.duration` (`AV_TIME_BASE` units → secs, `AV_NOPTS_VALUE`/negative →
0.0) and dimensions + `has_video` from the first video stream's `codecpar`. `ProbeResult`
is byte-identical; no caller changed. The JSON-parsing path and its sidecar spawn are gone
(`grep sidecar_command("ffprobe") src/` is clean). Full suite green: 40 tests.

Verification of acceptance #2 is a new parity test, `fixtures_probe::in_process_probe_matches_ffprobe_sidecar`
(skip-guarded on `binaries/`), which compares the in-process probe against the still-present
v1 ffprobe sidecar as ground truth across the whole corpus. **Every fixture matches exactly
on `has_video`, duration, and dimensions — with one pinned exception: `flv1-mp3.flv`.**

- **Decode-matrix gap found (hand off to issue 05):** the corpus carries `flv1-mp3.flv`,
  but the manifest's `--enable-decoder` list (ADR-0028 matrix) has `h263` and **not** `flv1`
  (FLV1/Sorenson Spark is a *distinct* decoder). FLV1 stores no dimensions in the container
  header, so `find_stream_info` would have to decode a frame to learn them — and with no
  flv1 decoder compiled in, it can't, leaving width/height at 0 (ffprobe's full build reads
  64×64). `has_video` is still correct, so pre-flight is unaffected and this is *not* a probe
  defect. The parity test pins the gap set to exactly `["flv1-mp3.flv"]`, so issue 05 must
  decide: add `flv1` to the decoder matrix (manifest edit + rebuild) or allowlist it. If
  flv1 stays out of the matrix, issue 04 can't transcode it either — the same fixture will
  resurface there.
- Tried bumping `analyzeduration`/`probesize` on open to close the flv1 gap; confirmed it
  doesn't help (no decoder to run), so reverted to a plain `open()`.
- `fixtures_probe.rs` lost its stale sidecar skip-guard on the `has_video` sweep (probe is
  in-process now, no sidecar needed); the guard moved to the parity test only. Added
  `serde_json` as a dev-dependency to parse the ground-truth ffprobe JSON.
- Hybrid state intact: encode still runs through the ffmpeg sidecar (`runner.rs`); `smoke.rs`
  still green. Sidecar machinery retirement stays with issue 06.
