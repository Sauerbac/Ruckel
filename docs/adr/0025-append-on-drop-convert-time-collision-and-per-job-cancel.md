# ADR-0025: Append-on-drop, convert-time collision check, and per-job cancel

- Status: Accepted
- Date: 2026-06-09
- Revises: [ADR-0008](0008-output-location-and-naming.md) /
  [ADR-0014](0014-collision-resolution.md) (collision detection moves out of
  pre-flight to a convert-time check), [ADR-0013](0013-encoder-process-control.md)
  (cancellation gains a per-job path beside the whole-batch abort),
  [ADR-0016](0016-tauri-command-and-event-contract.md) (`PreflightResult.collisions`
  removed; `check_collisions` + `cancel_job` commands and a `conversion:file_cancelled`
  event added), and the drop/reset paths of
  [ADR-0024](0024-frontend-action-model-revision.md) /
  [ADR-0020](0020-frontend-interaction-model.md) /
  [ADR-0023](0023-frontend-polish-v2.md) (a drop now appends rather than replaces;
  the ✕ is always rendered and context-aware).

## Context

A grilling pass on 2026-06-09 surfaced four behaviours that read as bugs:

1. **A drop replaced the batch.** `handleDrop` ran `rows = []` before pre-flight
   (the "new drop auto-clears" reset path of ADR-0024), so adding files after an
   initial drop threw the first set away.
2. **The collision modal never reappeared on re-convert.** Collisions were
   detected once in pre-flight and cached in `PreflightResult.collisions`. After a
   clean first run the cache was empty, so pressing Convert again silently
   overwrote the just-written `_ppt.mp4` files — violating ADR-0008's
   "never silently destructive."
3. **The per-row ✕ vanished mid-encode.** It was phase-gated off during
   `converting` (to keep rows 1:1 with the encoder's `file_index`), so finished
   rows shifted when it reappeared at the end — and there was no way to drop a
   single job from a running batch.
4. **The ✕ and the Done badge were a few px out of alignment** (the ✕ is a fixed
   18px square; the badge's height fell out of text line-height).

The unifying realisation: collision state was *captured too early* and *never
refreshed*, and row identity was *coupled to array position*, which is what
forced both the destructive drop and the disappearing ✕.

## Decision

### Drop appends; the batch is only ever trimmed by removing rows

A drop **merges** its files into the current batch, **deduplicated by
`source_path`** (re-dropping a file is a no-op — a `done` row stays `done`). There
is no replace gesture: the "new drop auto-clears" path of ADR-0024 is reversed.
The list empties only by removing rows down to zero (which routes through the
existing `reset()`).

Append applies in `idle` / `ready` / `done`. Dropping into a finished (`done`)
batch returns the phase to `ready` while existing `done` / `error` / `cancelled`
rows **keep their badges** until the next Convert. Drops stay **ignored** during
`reading` and `converting` (the existing `busy` guard) — mid-encode the batch is
locked for additions.

### Collision detection moves from pre-flight to a convert-time check

There is no reason to test the disk at drop time; whether a planned output exists
only matters at the moment we start to encode (and it goes stale the instant the
batch writes its outputs). So:

- `preflight` keeps computing each job's `output_path` but **drops the
  `output.exists()` check and the `collisions` field** from `PreflightResult`.
- A new, cheap command **`check_collisions(plan) -> Vec<Collision>`** runs the
  `output_path_for` + `output.exists()` loop with **no re-probe**. It is the
  single, authoritative collision source.
- `onConvert` becomes async: it calls `check_collisions`, and the modal is driven
  by that **fresh** result. This fixes the re-convert overwrite, and makes
  collisions correct for an appended batch for free (the whole current batch is
  re-checked; `job_index` aligns to row index).

The three-way resolution (Cancel / Override / Rename) of ADR-0014 is unchanged —
only *when* and *how often* collisions are detected changes.

### Per-job cancel, with cancelled rows marked in place

Row identity stays coupled to `file_index` (no stable-id refactor). Instead, a
cancelled job is **marked in place** rather than removed, so the array never
mutates mid-batch and event routing stays valid:

- A new `'cancelled'` `FileRowPhase`. Cancelled rows **persist into `done`** and
  re-run on the next Convert (`beginBatch` resets every row to `ready`), so a
  misclicked cancel is recoverable and an unwanted row is removed by hand.
- **Backend:** `EncoderState` keeps `CancelToken` as the **abort-all** flag and
  adds a shared **cancelled-jobs set** (`Arc<Mutex<HashSet<u32>>>`); both reset at
  `start_conversion`. `encode`'s cancel argument becomes a predicate
  (`abort.is_cancelled() || cancelled.contains(index)`) — the encoder still only
  asks "should I stop?", not why.
- New command **`cancel_job(file_index)`** inserts the index into the set. New
  event **`conversion:file_cancelled { file_index }`**. The runner skips a
  cancelled *queued* job without spawning FFmpeg, and on a per-job cancel of the
  *active* job emits `file_cancelled` and **continues** to the next job (the
  existing kill + temp-file delete of ADR-0013 means no partial output is left).
- **Footer Cancel is unchanged** = whole-batch abort. The interrupted active row
  now shows `'cancelled'` (truthful, and consistent with ✕-on-the-active-row);
  queued rows after an abort stay `'ready'`.
- **Frontend:** ✕ on a queued/active row calls `cancel_job(i)` and marks the row
  `'cancelled'` **optimistically** (a queued cancel shows instantly); the event is
  confirmation. `onProgress` ignores rows already `'cancelled'`. After the batch,
  ✕ on any row reverts to plain remove.

### The ✕ is always rendered and context-aware; FileRow cosmetics aligned

The ✕ is rendered in every phase (killing the end-of-batch layout shift) and its
meaning is driven by context, self-documented via the existing `Tooltip`:

| Context | Appearance | Tooltip |
|---|---|---|
| Not converting | normal, danger-on-hover | "Remove" |
| Converting, row `ready` / `converting` | normal, danger-on-hover | "Cancel this job" |
| Converting, row `done` / `error` / `cancelled` | inert/dimmed, `cursor-default` | "Can't remove while converting" |

The inert variant is *styled* inert, **not** the native `disabled` attribute, so
its hover tooltip still fires.

The Done / Error / **Cancelled** badges gain explicit `h-[18px]` + `leading-none`
+ `items-center`, matching the ✕'s 18px so they share a baseline. The
`'cancelled'` badge is neutral — `border-ink bg-surface text-muted` with the
muted square-dot motif, label "Cancelled", and **no secondary line** (so a
cancelled row reads as deliberately set aside).

### Transient feedback via the status bar

Append removes the surface the drop error relied on (the empty-state drop zone no
longer shows when a list is loaded). Rather than add toast infrastructure,
`StatusBar` gains an optional `notice?: string | null` prop that **overrides** the
status line for ~2.5s then auto-reverts (the derived `status` is the ground truth
underneath). It is fired only for **"No convertible video found"** when a drop
yields zero new files — a case that only occurs outside `converting`, so it never
fights the live progress readout. Mid-convert ignored drops stay silent.

## Consequences

- Adding files is additive and idempotent; the batch is a queue you build up, not
  a single shot.
- One authoritative, always-fresh collision path; re-converting a finished batch
  correctly routes through the modal instead of silently overwriting.
- A single job can be dropped from a running batch without aborting it, and
  finished rows no longer shift at end-of-batch; the encoder's `file_index`
  routing is preserved because cancelled rows are marked, never removed.
- The done-phase summary becomes "N done · N errors · N cancelled" (cancelled
  count derived frontend-side from row phases).
- **IPC contract change** (ADR-0016): `PreflightResult.collisions` is removed;
  `check_collisions` and `cancel_job` commands and the `conversion:file_cancelled`
  event are added. The ts-rs bindings regenerate accordingly.
- Implementation touches: `commands.rs` (`preflight` loses collision detection;
  new `check_collisions` / `cancel_job`; `run_batch` per-job skip + continue),
  `cancel.rs` / `EncoderState` (cancelled-jobs set), `runner.rs` (`encode` cancel
  predicate), the IPC event/command surface, `Shell.svelte` (append+dedup drop,
  async `onConvert` collision check, `cancelJob`, status-bar `notice`, cancelled
  counting), `FileRow.svelte` (always-rendered context-aware ✕, `'cancelled'`
  phase + badge, 18px alignment), and `StatusBar.svelte` (`notice` prop).
