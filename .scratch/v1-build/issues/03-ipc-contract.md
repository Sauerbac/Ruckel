# S1 — Full IPC contract (ts-rs)

Status: complete

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

## Comments

**Implemented (ts-rs 12).** Every ADR-0016 command + event payload is a Rust `serde`
type deriving `TS`, in its ADR-0017 module home rather than a new `ipc` module:

- `preflight/plan.rs`: `Resolution` / `Framerate` / `Audio` / `ConversionOptions`
  (+ the three preset constants), `ConversionJob`, `ConversionPlan` (alias for
  `Vec<ConversionJob>` → TS `ConversionJob[]`, no distinct type emitted).
- `preflight/collision.rs`: `Collision`. `preflight/mod.rs`: `PreflightResult`.
- `encoder/mod.rs`: `ProgressEvent` / `FileDoneEvent` / `FileErrorEvent` /
  `ConversionError` / `DoneEvent` / `CancelledEvent`, plus an `events` module of
  name constants (the single source of truth the runner emits with).

**Codegen gate.** `tests/codegen.rs` (`cargo test`) writes `src/lib/ipc/<Type>.ts`
(via ts-rs `export_to_string`), an `events.ts` constant map built from
`encoder::events::ALL`, and an `index.ts` barrel. Verified deterministic:
regenerating twice produces byte-identical output (zero diff). `src/lib/ipc/` is
excluded from Prettier + ESLint (generated; Rust is the source of truth). The
`git diff --exit-code` CI step that *fails* the build on drift is R2's to wire.

**Frontend consumes it.** `src/lib/ipc-client.ts` imports the generated payload
types + `CONVERSION_EVENTS` (no hand-written IPC types, no magic event strings).

**⚠ Contract gap to decide (affects 05/F2).** ADR-0020's phased reveal shows
*duration + resolution* on a file row after pre-flight, but ADR-0016's
`PreflightResult` is `plan + collisions` only — neither `ConversionJob` nor any
event carries probed duration/resolution. The frozen contract therefore can't
feed that reveal today. Options: amend ADR-0016 to add per-file probe metadata to
the pre-flight result (recommended, keeps the firewall intact for Phase 2), or
decide the reveal drops those fields. Left unresolved here; flagged for triage
before 05.
