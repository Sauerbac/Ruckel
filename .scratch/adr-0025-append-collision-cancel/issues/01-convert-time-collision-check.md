# Convert-time collision check (replaces pre-flight collision detection)

Status: ready-for-agent

## Parent

Spec: **ADR-0025** (`docs/adr/0025-append-on-drop-convert-time-collision-and-per-job-cancel.md`),
"Collision detection moves from pre-flight to a convert-time check". Revises ADR-0014
(when/how often collisions are detected) and ADR-0016 (IPC contract).

## What to build

Move collision detection out of pre-flight and into a fresh, authoritative check
that runs the moment the user presses Convert. Today `preflight` probes the disk
for existing `_ppt.mp4` outputs and caches the result in `PreflightResult.collisions`;
that cache goes stale the instant a batch writes its outputs, so re-pressing Convert
on a finished batch silently overwrites them (bug #2 — violates ADR-0008's
"never silently destructive").

End-to-end behaviour after this slice:

- `preflight` still computes each job's `output_path` but no longer probes
  `output.exists()` and no longer returns a `collisions` field.
- A new command `check_collisions(plan) -> Vec<Collision>` runs the
  `output_path_for` + `output.exists()` loop with **no re-probe** — it is the single
  source of collision truth. `Collision.job_index` aligns to the row's position in
  the plan passed in.
- `onConvert` becomes async: it calls `check_collisions`, and the modal opens off
  that **fresh** result rather than a cached one. The existing three-way resolution
  (Cancel / Override / Rename, ADR-0014) is unchanged — only *when* collisions are
  detected changes.

This is an **IPC contract change** (ADR-0016): `PreflightResult.collisions` removed,
`check_collisions` command added. The ts-rs bindings in `src/lib/ipc/` regenerate
accordingly. `Collision` stays defined in `preflight/collision.rs` and exported.

## Acceptance criteria

- [ ] `preflight` no longer reads the disk for output existence and `PreflightResult`
      has no `collisions` field; ts-rs bindings regenerated (`src/lib/ipc/`).
- [ ] `check_collisions(plan)` command registered in `lib.rs` and exposed via an
      `ipc-client.ts` wrapper; returns one `Collision` per planned output that
      already exists on disk, `job_index` = plan position.
- [ ] `onConvert` awaits `check_collisions` on the current batch and opens the modal
      only when the fresh result is non-empty; an empty result proceeds straight to
      `beginBatch`.
- [ ] Re-pressing Convert on a `done` batch whose `_ppt.mp4` files now exist
      re-opens the collision modal (no silent overwrite).
- [ ] Override / Rename / Cancel resolution still works against the fresh collisions.
- [ ] `cargo build`/`clippy`/`test` (via `--manifest-path src-tauri/Cargo.toml`) and
      the frontend typecheck/lint pass.

## Blocked by

- None - can start immediately.
