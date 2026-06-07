# ADR-0004: FFmpeg/ffprobe as sidecar processes, not FFI

- Status: Accepted
- Date: 2026-06-07

## Context

Ruckel needs to (a) decode/encode video and (b) probe files for duration and stream info. The
two ways to use FFmpeg are: bundle the `ffmpeg`/`ffprobe` executables and spawn them as child
processes (sidecar), or link the `libav*` libraries into the Rust binary and call them via FFI.

We need full control over cancellation, stderr capture, and progress streaming.

## Decision

Bundle **`ffmpeg.exe` and `ffprobe.exe` as sidecars** in `src-tauri/binaries/`, spawned via
`std::process::Command` from Rust. Do **not** use the Tauri shell plugin — direct process
management gives precise control over kill, stdio, and event emission.

## Consequences

- Clean cancellation (`child.kill()` — see [ADR-0013](0013-encoder-process-control.md)) and
  straightforward progress parsing from a dedicated stdout pipe.
- Keeps GPL code out of the Ruckel process boundary, which preserves licensing flexibility
  (see [ADR-0003](0003-gplv3-license.md)).
- `ffprobe` is reused by both pre-flight and the encoder via a single `probe.rs` wrapper
  ([ADR-0017](0017-rust-module-structure.md)).

## Alternatives considered

- **FFI to `libav*`** — kept open as a future path if in-process control ever becomes
  necessary. Tradeoff: it links GPL into Ruckel (acceptable under [ADR-0003](0003-gplv3-license.md))
  but adds build complexity and removes the clean process boundary. Not needed for v1.
- **Tauri shell plugin** — rejected: less control over the child process lifecycle than raw
  `std::process::Command`.
