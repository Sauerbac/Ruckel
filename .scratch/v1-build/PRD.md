# PRD: Ruckel v1 build-out

Status: ready-for-agent

## Summary

Implement Ruckel v1 — the Windows-only video → PowerPoint-safe MP4 converter — on top of the
existing compiling skeleton. The architecture, domain language, and frontend design are already
locked in `CONTEXT.md` and `docs/adr/0001–0021`. This PRD does **not** re-decide any of that; it
sequences the *implementation* into ~15 LLM-session-sized vertical slices, each independently
verifiable, so no single session has to do everything.

## Strategy

Built around three locked decisions from the planning session:

1. **Vertical tracer slices, contract-first.** The first feature session is a razor-thin
   end-to-end path (drop one file → real probe → real encode → progress → done). It retires the
   two biggest integration risks — the Tauri IPC round-trip and the real FFmpeg sidecar spawn —
   before any breadth is built. Later slices thicken each stage.

2. **A frozen IPC contract is the firewall.** Session S1 freezes the *complete* ADR-0016 surface
   (every command + event payload, including collisions/errors/cancelled) as Rust `serde` structs
   with `#[derive(TS)]`. Rust is the single source of truth; TypeScript types and an `events.ts`
   name-constant map are **generated**. Drift becomes a failed `cargo test`, not a silent runtime
   `undefined`. This frozen contract is what makes the Phase 2 parallel split safe.

3. **Two foundations pulled to the front.** Both the FFmpeg binaries (S0) and the design
   system + primitives + shell (F1) are contract-free and depend only on the existing pipeline.
   Doing them first means every later session has both a real FFmpeg to smoke-test against and the
   visual vocabulary to build with.

## Phasing

- **Phase 0 — Foundations:** S0 sidecars · F1 design system/shell · S1 contract
- **Phase 1 — Spine:** S2 tracer bullet
- **Phase 2 — Parallel fan-out:** F2 (frontend) ∥ B1/B2/B3 (backend), disjoint file trees
- **Phase 3 — Feature integration:** I1 multi-file · I2 options/presets · I3 collision · I4 errors · I5 cancel
- **Phase 4 — Release:** R1 polish · R2 CI + bundling + GPL + git-storage decision

## Verification philosophy

- **Frontend:** a dev-only **gallery route** renders every component in every locked state from
  mock props; verification is a **manual screenshot eyeball** against ADR-0018/0019/0020. No
  screenshot-regression diffs for v1 (brittle during heavy UI churn).
- **Backend:** **table-tested pure logic** (path generation, progress parsing, options→args,
  extension filter) plus **one self-generating real-FFmpeg smoke test** per encode path — the test
  synthesizes its own fixture via `ffmpeg -f lavfi -i testsrc`, so no media is committed. The smoke
  test is skipped when `src-tauri/binaries/` is empty, so contract-free sessions stay green without
  the ~90 MB sidecars.

## Notes carried forward

- **ADR-0005 amendment (in S0):** binaries are *fetched-into* `src-tauri/binaries/` via a pinned
  manifest (version + SHA-256) rather than *committed-into* it. They still land where ADR-0017
  expects. Record as an ADR-0005 amendment or a new ADR-0022.
- **Git-storage of binaries (LFS vs commit) is deferred to R2**, resolved right before first push.
  Until then `src-tauri/binaries/` is gitignored.

## Issue index

| # | Slice | Type | Blocked by |
|---|---|---|---|
| 01 | S0 — Fetch FFmpeg/ffprobe sidecars + pin manifest | AFK | none |
| 02 | F1 — Design system + primitives + shell + gallery route | AFK | none |
| 03 | S1 — Full IPC contract (ts-rs) | AFK | none |
| 04 | S2 — Tracer bullet | AFK | 01, 03 |
| 05 | F2 — Component states in gallery | AFK | 02, 03 |
| 06 | B1 — Output safety: temp + atomic rename | AFK | 04 |
| 07 | B2 — Options → ffmpeg args mapping | AFK | 03, 04 |
| 08 | B3 — Scanner: flat walk + extension filter | AFK | 04 |
| 09 | I1 — Multi-file | AFK | 05, 08 |
| 10 | I2 — Options/presets wired | AFK | 05, 07 |
| 11 | I3 — Collision | AFK | 05, 08 |
| 12 | I4 — Errors | AFK | 04, 05 |
| 13 | I5 — Cancel | AFK | 04, 05 |
| 14 | R1 — Final window chrome / edge-case polish | AFK | 09–13 |
| 15 | R2 — CI + bundling + GPL + git-storage decision | HITL | 14 |
