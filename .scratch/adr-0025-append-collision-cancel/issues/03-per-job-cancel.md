# Per-job cancel + `'cancelled'` phase + always-rendered context-aware ✕

Status: ready-for-agent

## Parent

Spec: **ADR-0025** (`docs/adr/0025-append-on-drop-convert-time-collision-and-per-job-cancel.md`),
"Per-job cancel, with cancelled rows marked in place" and "The ✕ is always rendered
and context-aware; FileRow cosmetics aligned". Revises ADR-0013 (cancellation gains
a per-job path beside the whole-batch abort) and ADR-0016 (`cancel_job` command +
`conversion:file_cancelled` event).

## What to build

Let a single job be dropped from a running batch without aborting the whole batch,
and stop the per-row ✕ from vanishing mid-encode (bugs #3, #4). Row identity stays
coupled to `file_index` — a cancelled job is **marked in place**, never removed, so
the array never mutates mid-batch and event routing stays valid.

End-to-end behaviour after this slice:

**Backend**
- `EncoderState` keeps `CancelToken` as the **abort-all** flag and adds a shared
  **cancelled-jobs set** (`Arc<Mutex<HashSet<u32>>>`); both reset at
  `start_conversion`.
- `encode`'s cancel argument becomes a predicate (`&dyn Fn() -> bool` or equivalent)
  evaluating `abort.is_cancelled() || cancelled.contains(index)`; the existing
  kill + temp-file delete path (ADR-0013) is unchanged — the encoder still only asks
  "should I stop?", not why.
- New command `cancel_job(file_index)` inserts the index into the set.
- New event `conversion:file_cancelled { file_index }`.
- `run_batch` skips a cancelled **queued** job without spawning FFmpeg, and on a
  per-job cancel of the **active** job emits `file_cancelled` and **continues** to
  the next job (no partial output left, per the existing temp-delete).
- Footer Cancel (whole-batch abort) is unchanged in mechanism; the interrupted
  active row now reports as cancelled (see frontend), queued rows after an abort
  stay `ready`.

**Frontend**
- New `'cancelled'` `FileRowPhase`, neutral badge: `border-ink bg-surface text-muted`
  with the muted square-dot motif, label "Cancelled", **no secondary line**.
- ✕ on a queued/active row calls `cancel_job(i)` and marks the row `'cancelled'`
  **optimistically** (queued cancel shows instantly); the `file_cancelled` event is
  confirmation. `onProgress` ignores rows already `'cancelled'`.
- Footer-abort (`onCancelled`) marks the interrupted active row `'cancelled'`.
- Cancelled rows **persist into `done`** and re-run on the next Convert
  (`beginBatch` resets every row to `ready`), so a misclick is recoverable.
- Derive a cancelled count; the done summary becomes "N done · N errors · N cancelled"
  (add the cancelled count to `StatusBar`'s done state).

**FileRow ✕ + alignment (bug #4)**
- The ✕ is rendered in **every** phase (kills the end-of-batch layout shift),
  context-driven and self-documented via the existing `Tooltip`:

  | Context | Appearance | Tooltip |
  |---|---|---|
  | Not converting | normal, danger-on-hover | "Remove" |
  | Converting, row `ready`/`converting` | normal, danger-on-hover | "Cancel this job" |
  | Converting, row `done`/`error`/`cancelled` | inert/dimmed, `cursor-default` | "Can't remove while converting" |

  The inert variant is **styled** inert, **not** the native `disabled` attribute, so
  its hover tooltip still fires.
- The Done / Error / Cancelled badges gain explicit `h-[18px]` + `leading-none` +
  `items-center`, matching the ✕'s 18px so they share a baseline.

## Acceptance criteria

- [ ] `cancel_job(file_index)` command registered in `lib.rs` + `ipc-client.ts`
      wrapper; `conversion:file_cancelled { file_index }` event added to the contract
      and subscribed; ts-rs bindings regenerated.
- [ ] `EncoderState` holds the cancelled-jobs set alongside `CancelToken`; both
      reset on `start_conversion`. `encode` takes a cancel predicate.
- [ ] Cancelling a **queued** job removes it from the run without spawning FFmpeg;
      cancelling the **active** job kills it (no partial output) and the batch
      continues to the next job.
- [ ] Frontend ✕ during converting cancels the job and marks the row `'cancelled'`
      optimistically; the event confirms; `onProgress` never overwrites a
      `'cancelled'` row.
- [ ] Footer Cancel still aborts the whole batch; the interrupted active row shows
      `'cancelled'`, queued rows stay `ready`.
- [ ] Cancelled rows persist into `done` and re-run (return to `ready`) on the next
      Convert.
- [ ] Done summary reads "N done · N errors · N cancelled" with a frontend-derived
      cancelled count.
- [ ] The ✕ is rendered in all phases with the three context-aware states + tooltips
      above; no end-of-batch layout shift; Done/Error/Cancelled badges share the 18px
      baseline with the ✕.
- [ ] `cargo build`/`clippy`/`test` (via `--manifest-path src-tauri/Cargo.toml`) and
      the frontend typecheck/lint pass.

## Blocked by

- None - can start immediately. (Independent subsystem; the only overlap with
  Slices 01/02 is a separate edit to `StatusBar`'s done-state count.)
