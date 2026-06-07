# S1 — Full IPC contract (ts-rs)

Status: ready-for-agent

## What to build

Freeze the **complete** ADR-0016 command/event surface as the single source of truth, owned by
Rust. Define every payload as a Rust `serde` struct/enum with `#[derive(TS)]` (via `ts-rs` or
`specta`) — not just what the tracer needs, but the **whole** surface: `ConversionJob` /
`ConversionPlan`, the four options, `PreflightResult` (plan + collisions), and every event payload
including `conversion:progress`, `file_done`, `file_error`, `done` (with the errors summary), and
`cancelled`. Freezing the full contract now is what lets Phase 2 fan out in parallel — later slices
fill in *behavior* against frozen types rather than extending the interface.

Generate the TypeScript counterparts into `src/lib/ipc/` plus an `events.ts` constant map for the
stringly-typed event names, so neither the payload shapes nor the event strings are hand-duplicated
on the frontend. A `cargo test` regenerates the TS; a clean tree means no drift.

The three command handlers (`preflight`, `start_conversion`, `cancel_conversion`) may exist as
thin stubs returning placeholder data — wiring real behavior is S2 onward. This slice is about the
types and the codegen gate.

## Acceptance criteria

- [ ] Every ADR-0016 command + event payload exists as a Rust type deriving `TS`
- [ ] `cargo test` generates `src/lib/ipc/*.ts` + an `events.ts` event-name constant map
- [ ] Running the codegen on a clean tree produces **zero diff** (drift fails the build)
- [ ] Frontend imports the generated types and event constants (no hand-written IPC types or magic event strings)

## Blocked by

None - can start immediately.
