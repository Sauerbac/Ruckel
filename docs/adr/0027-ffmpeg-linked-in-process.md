# ADR-0027: FFmpeg linked in-process via the libav* API (rsmpeg)

- Status: Accepted
- Date: 2026-06-09
- Supersedes: [ADR-0004](0004-ffmpeg-sidecar-not-ffi.md)

## Context

v1 is feature-complete on the sidecar architecture: `ffmpeg.exe`/`ffprobe.exe` (~135 MB each,
BtbN full-GPL builds) spawned as child processes. The v2 goal is a **single copyable binary** —
`ruckel.exe` alone is the product, nothing beside it. That requires FFmpeg *inside* the process,
which is exactly the "future path" ADR-0004 kept open.

Two ways to put FFmpeg in-process were considered:

1. **fftools-in-process** (the ffmpeg-kit approach): compile `fftools/ffmpeg.c` with `main`
   renamed and call it with the existing argument vector from `args.rs`.
2. **libav\* C API directly**: write the transcode loop ourselves against `libavformat`,
   `libavcodec`, `libswscale`, `libswresample`.

Ruckel has exactly one encode shape (H.264 high / yuv420p / AAC / `+faststart` mp4) with four
small knobs. The fftools path drags in the entire CLI option-parsing and format-negotiation
machinery — precisely the bloat v2 exists to remove — and since FFmpeg 7 the fftools carry a
multithreaded scheduler that is hard to embed and cancel cleanly.

## Decision

Link a **minimal, self-built static FFmpeg** (see [ADR-0029](0029-self-built-minimal-static-ffmpeg.md))
into `ruckel.exe` and program against the **libav\* C API directly**:

- **Libraries linked:** `libavformat` (demux/mux), `libavcodec` (decode, libx264, native AAC),
  `libavutil`, `libswscale`, `libswresample`, and a **minimal `libavfilter`** (see below).
  `libavdevice`, network protocols, and everything else: compiled out.
- **Bindings:** the **`rsmpeg`** crate (CCExtractor; safe RAII wrappers over `rusty_ffmpeg`
  sys bindings; FFmpeg 7.x-compatible; designed for bring-your-own static FFmpeg). We do not
  hand-roll bindgen or our own `unsafe` RAII layer — that is the highest-risk,
  lowest-differentiation code in the project. Escape hatch if rsmpeg ever stalls: drop to its
  underlying `rusty_ffmpeg` sys layer; that is a contained refactor, not a rewrite.
- **The ffmpeg/ffprobe split dissolves.** Probing is `avformat_open_input` +
  `avformat_find_stream_info` — three library calls behind the unchanged `ProbeResult` struct.
  No JSON, no process, no `ffprobe` anything.
- **Tracer gate:** the first slice of v2 is a standalone test that links rsmpeg against our
  static libs and transcodes one h264 fixture in-process on `x86_64-pc-windows-msvc`.
  **No production code changes until that gate passes** — it retires the MSVC-link risk, the
  rsmpeg-discovery risk, and the `--disable-everything` risk in one place.

### Minimal libavfilter — a flagged refinement

The grilling leaned toward skipping `libavfilter` entirely (raw `libswscale` for the resolution
cap). Writing the plan down surfaced two **behavior-parity requirements** that the ffmpeg CLI
fulfills via auto-inserted filters and `swscale` alone cannot:

1. **Autorotation** — the CLI physically rotates phone videos per their display-matrix side
   data by default (`-autorotate`). `swscale` cannot transpose.
2. **CFR frame duplication/drop** — the `-r` output option's drop/duplicate semantics for the
   framerate cap.

Re-implementing both by hand is more code and more risk than a minimal filter graph. Decision:
include `libavfilter` with **only** `buffer`, `buffersink`, `scale`, `format`, `transpose`,
`hflip`, `vflip`, `fps`, `null` enabled. Audio stays on `libswresample` directly — no audio
filters.

## Consequences

- `args.rs`'s role — single source of truth for what an encode runs with — survives as a pure
  `options → EncoderConfig` mapping with table tests; the PowerPoint-safe contract (ADR-0006)
  is asserted against the config struct instead of flag pairs. The flag-vector tests are
  retired with the sidecar.
- Process-control mechanics (`child.kill()`, `-progress pipe:1` parsing) are replaced by the
  in-process runtime model — see [ADR-0031](0031-in-process-transcode-runtime.md).
- **Crash blast radius is accepted:** a true access violation in libav now takes Ruckel down
  instead of a child process. Mitigation is upstream (the curated decoders are the most-fuzzed
  code in FFmpeg), the attack surface is files the user themselves dropped, normal corruption
  still surfaces as error codes (skip-and-continue per ADR-0015 unchanged), and the
  temp+rename contract (ADR-0012) guarantees a crash never corrupts an output.
- GPL: linking GPL x264/FFmpeg in-process is clean — Ruckel is already GPLv3 (ADR-0003).
  ADR-0004's "keeps GPL out of the process boundary" flexibility is consciously spent.
- The frontend must not know this rewrite happened: the Tauri command surface and event
  contract (ADR-0016) are **frozen byte-for-byte**.

## Alternatives considered

- **fftools-in-process** — rejected: embeds a general-purpose CLI to call it with one fixed
  argument shape; maximum bloat, unstable internals, hard-to-embed FFmpeg-7 scheduler.
- **Embed-and-extract** (embed sidecar exes via `include_bytes!`, extract to temp, spawn) —
  rejected: one copyable file for 5% of the work, but the in-process build is the goal itself,
  not merely the means.
- **`ffmpeg-next`** — historically lags new FFmpeg majors; **hand-rolled bindgen** — owns every
  FFI footgun for zero product value. Both rejected in favor of rsmpeg.
