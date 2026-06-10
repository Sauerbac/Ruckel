# Probe swap — in-process libavformat behind the unchanged ProbeResult

Status: ready-for-agent

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
