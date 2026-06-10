# Encoder swap — in-process transcode loop behind the frozen contract

Status: complete

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

## Comments

**2026-06-10 (agent, on completion):** The sidecar encode is gone; `runner::encode` now
runs the full in-process loop via rsmpeg, behind the unchanged `encode()` signature, so
`commands.rs`/the IPC contract/the frontend are untouched (codegen test still green). Full
suite green: 46 tests.

Shape:
- **`args.rs` is now the pure `options → EncoderConfig` seam.** Fixed ADR-0006 fields
  (libx264 · high · medium · yuv420p · faststart · `level: None`) plus the four resolved
  knobs (crf, `scale_height`, `fps_cap`, `AudioConfig`). Table tests assert the contract
  (incl. the deliberate absence of a level) and every preset/knob.
- **`runner.rs` is the loop:** demux → decode → minimal video graph → libx264 + native
  AAC (via swresample + an `AVAudioFifo`) → mp4 `+faststart` → temp path → atomic rename.
  - Video graph built **lazily from the first decoded frame** (its real pixel
    format/size/SAR — codecpar isn't reliable for all codecs), via `parse_ptr` on a filter
    string: `[transpose/hflip/vflip] , [scale=-2:H] , format=yuv420p , [fps=N]`. The nine
    compiled-in filters are exactly enough. Rotation comes from the stream display matrix
    (`av_display_rotation_get`), CLI-parity autorotate, matrix **not** copied to output.
  - Encoder + output streams + header are opened lazily on the first *filtered* frame
    (post-filter dims). Audio decoded before then accumulates in the FIFO and flows once the
    header is written; final drain flushes resampler → FIFO → encoder.
  - AAC: source rate kept when the encoder supports it, else nearest; channel layout
    preserved; `fltp` via swresample. Cancel checked once per demuxed packet → drop
    contexts (RAII) + delete temp. Progress = muxed video PTS ÷ probed duration, throttled
    to ~500 ms, with an fps/speed estimate from wall-clock.
- **`progress.rs`** lost the `-progress` text parser (sidecar-only); kept `percent_of`.

Acceptance verified by `tests/encode_inprocess.rs` (in-process, no sidecar): the h264
fixture comes out h264-high/yuv420p/aac with **moov before mdat** (a structural faststart
check the v1 suite never did); resolution cap → even aspect-preserved height; fps cap →
CFR at the cap (`r_frame_rate`); `Audio::None` → no audio stream; cancel → no output, no
temp; corrupt input → `Failed`, no output. `smoke.rs` (sidecar-synthesized fixture, now
in-process encode) still green.

Carry-overs for issue 05:
- The committed rotated fixture is **64×64 (square)**, so physical-rotation-by-dimension-
  swap can't be asserted. Issue 05's per-fixture transcode needs a **non-square** rotated
  sample to verify autorotation visually; for now the rotate path is exercised end-to-end
  (transcodes cleanly) and `video_filter_desc` is unit-tested.
- flv1 stays undecodable in the matrix (carried from issue 03) — that fixture won't
  transcode until issue 05 adds the decoder or allowlists it.
- The lazy-graph-from-first-frame design was chosen specifically so issue 05's broad matrix
  doesn't trip over codecs whose pixel format is only known after decoding.
