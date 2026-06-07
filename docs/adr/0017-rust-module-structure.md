# ADR-0017: Rust module structure

- Status: Accepted
- Date: 2026-06-07

## Context

The backend has clearly separable responsibilities: IPC handlers, pre-flight, encoding, and a
shared probe wrapper. The module layout should mirror the pre-flight/encode split.

## Decision

```
src-tauri/src/
├── main.rs              # Tauri setup, command registration
├── commands.rs          # #[tauri::command] handlers — thin, delegate immediately
├── preflight/
│   ├── mod.rs
│   ├── scanner.rs       # folder walk, extension filter, ffprobe dispatch
│   ├── collision.rs     # output path generation, collision detection
│   └── plan.rs          # ConversionJob / ConversionPlan types
├── encoder/
│   ├── mod.rs
│   ├── runner.rs        # spawns FFmpeg, reads stdout/stderr, emits events
│   ├── progress.rs      # parses -progress pipe:1 key=value lines
│   └── cancel.rs        # cancellation token / shared state
└── probe.rs             # ffprobe wrapper (used by both preflight and runner)
```

## Consequences

- The module boundaries match the architectural phases
  ([ADR-0010](0010-preflight-encode-separation.md)), keeping each unit testable.
- `commands.rs` stays thin; logic lives in `preflight/` and `encoder/`.
- `probe.rs` is the single ffprobe entry point, shared by pre-flight and the encoder.
