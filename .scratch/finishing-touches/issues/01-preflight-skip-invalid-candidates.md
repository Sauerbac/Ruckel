# Pre-flight skips invalid candidates instead of aborting

Status: complete

The one genuine functional gap in the finishing-touches spree. Durable rationale
and the decision record live in **ADR-0026**
(`docs/adr/0026-preflight-skips-invalid-candidates.md`), which makes the
implementation conform to ADR-0015 and amends the ADR-0016 contract.

## Problem

`commands.rs::preflight` returns `Err` on the **first** scanned candidate that
fails to probe or has no video stream (`probe::probe(&source)?` and
`if !probed.has_video { return Err(...) }`). So one stray file in a folder drop —
an audio file, a renamed text file, a corrupt clip — throws away every good file
dropped alongside it. ADR-0015 already says pre-flight errors are skip-and-surface,
never abort; this is the implementation finally conforming.

## Files in play

- `src-tauri/src/commands.rs` — `preflight`: skip-and-count instead of abort
- `src-tauri/src/preflight/mod.rs` — `PreflightResult` gains `skipped: u32`
- `src-tauri/tests/codegen.rs` (regen) → `src/lib/ipc/PreflightResult.ts`
- `src/lib/shell/Shell.svelte` — `handleDrop` notice matrix
- `docs/adr/0016-...md` already amended (no further change needed here)

## What to build

### A. Backend: skip-and-count (`commands.rs::preflight`)

Replace the two abort paths with per-candidate skips. For each scanned candidate:

- `probe::probe(&source)` returns `Err` (won't spawn / non-zero exit / malformed
  JSON) → **skip** (don't `?`-propagate).
- `Ok` but `!has_video` → **skip**.
- `Ok` with a video stream → push the job + `FileProbe` as today.

Count every skipped candidate. Return `skipped` on the result. All four probe
failure modes are treated identically — no per-reason branching (ADR-0026).

### B. Contract: `PreflightResult.skipped: u32`

Add the field to the `PreflightResult` struct (its `TS`-deriving home in
`preflight/mod.rs`). Run `cargo test` to regenerate `src/lib/ipc/PreflightResult.ts`;
the codegen drift gate must stay green (commit the regenerated `.ts`).

### C. Frontend: drop-outcome notice matrix (`Shell.svelte::handleDrop`)

Use `result.skipped` plus the existing dedup logic to drive the status-bar
`notice` (ADR-0025), one aggregate count, never itemised:

| Drop outcome | Surface | Copy |
|---|---|---|
| Some added + `skipped > 0` (empty-start or append) | `showNotice` | `N SKIPPED (NOT VIDEO)` |
| Append, nothing added, `skipped > 0` | `showNotice` | `NO CONVERTIBLE VIDEO` |
| Append, nothing added, `skipped === 0` (all duplicates) | `showNotice` | `ALREADY ADDED` |
| Empty-start, nothing added at all | `dropError` (existing) | `No convertible video found` |

- On an **empty-start drop that yields some valid files**, still flip to `ready`
  and *also* fire the `N SKIPPED` notice if any were skipped.
- The current `catch (e) { showNotice(String(e)) }` becomes a near-dead safety
  net (preflight no longer errors for file reasons) — keep it, but it should no
  longer be a path users hit.
- `N` pluralises naturally via the count; copy stays uppercase warning-style to
  match the existing notice.

## Acceptance criteria

- [ ] A folder drop of N valid videos + M invalid files loads exactly N rows and
      fires `N… M SKIPPED (NOT VIDEO)` — never zero rows
- [ ] An all-invalid empty-start drop shows `No convertible video found` in the
      drop zone (unchanged)
- [ ] Re-dropping only already-loaded files shows `ALREADY ADDED`, not
      `No convertible video found`
- [ ] `preflight` returns `Ok` for every drop that scans at least one candidate;
      a corrupt/renamed-text file is skipped, not surfaced as a hard error
- [ ] `PreflightResult.skipped` is on the regenerated contract; the codegen drift
      gate (`git diff --exit-code src/lib/ipc`) is clean
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml` and the frontend build
      both pass; new `preflight` skip behaviour is unit-covered

## Notes

- Run `cargo` with `--manifest-path src-tauri/Cargo.toml` (see CLAUDE.md).
- `preflight` is an `async` command but does blocking probes today — keep its
  existing shape; this issue only changes the loop's error handling + return.
- A skipped-vs-duplicate test fixture: drop a folder containing one real clip and
  one extensionless / audio-only file (the scanner keeps both as candidates;
  pre-flight must keep the clip and skip the other).
