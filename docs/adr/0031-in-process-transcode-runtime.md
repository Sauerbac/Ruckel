# ADR-0031: In-process transcode runtime — threading, cancellation, progress, probe

- Status: Accepted
- Date: 2026-06-09
- Supersedes the *mechanics* of [ADR-0013](0013-encoder-process-control.md) (its user-facing
  contract — live progress, responsive cancel — is unchanged)
- Conforms to: [ADR-0016](0016-tauri-command-and-event-contract.md) (frozen),
  [ADR-0012](0012-output-safety-temp-and-rename.md), [ADR-0015](0015-error-handling.md),
  [ADR-0009](0009-processing-model.md)

## Context

ADR-0013's process-control model — `child.kill()` for cancel, `-progress pipe:1` text parsing
at 2 Hz for progress — dies with the sidecar ([ADR-0027](0027-ffmpeg-linked-in-process.md)).
Something equivalent-or-better must replace it, **without the frontend noticing**.

## Decision

### The frontend contract is frozen

The Tauri command surface and event contract (ADR-0016) stay **byte-for-byte identical**:
same commands, same event names, same payload shapes, same ~500 ms progress cadence, same
per-job lifecycle (ADR-0025 per-job cancel included). Svelte code is untouched. v2's
definition of done includes "the frontend cannot tell."

### Replacement mechanics

- **Threading:** each Conversion Job's transcode loop runs on a dedicated worker thread
  (`spawn_blocking`-style), one job at a time — the sequential model (ADR-0009) is unchanged.
- **Cancellation:** an `AtomicBool` per job, checked once per demuxed packet — replaces
  `child.kill()`. Cancel latency becomes ~one packet (milliseconds). On cancel: drop the
  libav contexts, delete the temp file. Strictly better than killing a process.
- **Progress:** computed from the PTS of packets as they are muxed, against the duration the
  probe reported; emitted through the existing event, throttled to the same ~500 ms cadence.
  More accurate than today — no text parsing, no 2 Hz pipe quantization.
- **Probe:** `avformat_open_input` + `avformat_find_stream_info` on the worker thread, behind
  the **unchanged `ProbeResult` struct** — pre-flight (ADR-0026 skip semantics, ADR-0011
  extensionless fallback) does not change at all. Probing gets cheaper: no process spawn.
- **Output safety:** the temp-file + atomic-rename contract (ADR-0012) is unchanged and now
  also covers the in-process crash case — a crash never leaves a corrupt `_ppt` output, only
  an orphaned temp file.

### Error model

Normal decode/demux corruption surfaces as libav error codes → the job fails → skip-and-continue
and the summary notice, exactly as ADR-0015 specifies today. A true access violation inside
libav takes the whole app down — accepted, with rationale recorded in ADR-0027.

## Consequences

- `encoder/runner.rs` (spawn/pipe plumbing), `encoder/progress.rs` (`-progress` text parsing),
  and `encoder/cancel.rs` (process kill) are rewritten around the loop; `encoder/args.rs`
  becomes the pure `options → EncoderConfig` seam (ADR-0006 contract asserted on the struct).
- Cancel and progress get *better* without the frontend changing — the whole point of freezing
  the contract.
