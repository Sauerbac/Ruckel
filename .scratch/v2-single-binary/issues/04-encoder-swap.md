# Encoder swap — in-process transcode loop behind the frozen contract

Status: ready-for-agent

## Parent

`.scratch/v2-single-binary/PRD.md` (step 6, "the big slice" — full loop spec lives there).
Decision records: ADR-0027, ADR-0031; output contract ADR-0006/0012 unchanged.

## What to build

Replace the sidecar encode with the in-process transcode loop, behind the **frozen
ADR-0016 command/event contract** — same commands, event names, payload shapes, ~500 ms
progress cadence, per-job lifecycle incl. ADR-0025 per-job cancel. The frontend must not
be able to tell. Zero behavior change is the bar.

- **Pure seam:** `args.rs`'s role becomes a pure `options → EncoderConfig` mapping (CRF,
  scale target, fps cap, AAC bitrate/none) with the same table-test philosophy; the
  ADR-0006 PowerPoint-safe contract (h264 high, yuv420p, **no level**, faststart, aac/-an
  equivalent) is asserted on the struct.
- **The loop** (per the PRD spec): demux → decode → minimal video filter graph
  (display-matrix autorotation via transpose/hflip/vflip; `scale=-2:H` when capped;
  mandatory `format=yuv420p`; `fps` only when capped) → libx264 (profile high, preset
  medium, crf from options, no level) + native AAC via swresample (fltp, supported rate,
  channel layout preserved) or no audio stream at all → mp4 mux with `+faststart`,
  writing to the ADR-0012 temp path, atomic rename on success.
- **Cancel:** per-job `AtomicBool` checked once per demuxed packet; on cancel drop
  contexts, delete temp, emit the existing cancelled event.
- **Progress:** muxed video packet PTS over probed duration, clamped, emitted through the
  existing event at the existing cadence.
- **Errors:** libav error codes → job failure → skip-and-continue + summary, exactly the
  ADR-0015 surface as today.

The ffmpeg sidecar stops being spawned; machinery retirement stays with issue 06.

## Acceptance criteria

- [ ] No frontend file changes; no change to the IPC contract types or generated TS
- [ ] `EncoderConfig` table tests cover all three presets and every knob value, asserting
      the ADR-0006 contract on the struct (incl. absence of a level)
- [ ] Converting the h264 fixture produces a `_ppt` output that plays and carries
      h264-high/yuv420p/aac/faststart; the rotated fixture comes out physically rotated
- [ ] The four options act: resolution cap yields even-width aspect-preserved output,
      fps cap yields CFR at the cap, audio-none yields no audio stream, CRF passes through
- [ ] Cancel mid-encode stops within ~a packet, deletes the temp file, leaves no `_ppt`
      output, and emits the existing cancelled event
- [ ] Progress events arrive with the existing payload shape at ~500 ms cadence and reach
      a sane terminal value
- [ ] A corrupt input fails the job and the batch continues (ADR-0015 unchanged)
- [ ] No code path spawns `ffmpeg` any more; full suite green

## Blocked by

- `02-ffmpeg-build-and-tracer-gate.md` (the gate must be green)

## Notes

- Runs in parallel with issue 03 if convenient — both fan out from the gate; merge order
  between 03 and 04 doesn't matter.
- Old `-progress pipe:1` parsing, spawn/pipe plumbing, and process-kill cancel code are
  superseded; delete rather than strand them (their tests too — the EncoderConfig and
  integration tests are the replacement coverage).
- Subtle parity notes (AAC resampling, faststart-on-temp, drain order) are written out in
  the PRD's "Architecture: the transcode loop" section — follow it.
