<!--
  Application shell (ADR-0019 v2 layout) wired to the live IPC contract — the
  Phase 3 feature integration (I1 multi-file · I2 options/presets · I3 collision
  · I4 errors · I5 cancel). A 1.5px ink outer frame around a solid-ink title
  bar, a two-panel split (left: drop target / file list, right: options with a
  pinned rail-footer action), and a full-width status-only StatusBar.

  The primary action lives in the right panel's pinned footer (ADR-0024): a
  single morphing slot — disabled Convert (no files) → green Convert (ready) →
  danger Cancel (converting) → green Convert (done/re-run). It sits beside the
  OptionsPanel rather than inside it so it stays live while the panel is
  `disabled` mid-encode.

  The whole app is a small state machine over `phase`:

    idle      empty drop target; options dimmed; "No files"
    reading   pre-flight in flight (scan → probe → collision check)
    ready     N file rows + live options; CONVERT (opens the collision modal
              first when pre-flight found existing outputs)
    converting sequential encode (ADR-0009); the active row shows progress, the
              rest queued/done; the footer CANCEL kills the in-flight job (ADR-0013)
    done      per-file outcomes retained; "N done · N errors"; the footer Convert
              re-runs the batch (routing through collisions like any convert)

  The frontend owns no encode logic: it sends a resolved Conversion Plan to
  `start_conversion` and reflects the `conversion:*` event stream back onto the
  rows. `file_index` indexes the plan we sent, so `rows` is kept 1:1 with that
  plan (collision-cancelled files are dropped from both together).
-->
<script lang="ts">
  import { onDestroy, onMount } from 'svelte'
  import TitleBar from './TitleBar.svelte'
  import Button from '../components/Button.svelte'
  import FileRow, { type FileRowPhase } from '../components/FileRow.svelte'
  import OptionsPanel from '../components/OptionsPanel.svelte'
  import StatusBar, {
    type StatusBarStatus,
  } from '../components/StatusBar.svelte'
  import CollisionModal, {
    type CollisionChoice,
  } from '../components/CollisionModal.svelte'
  import { baseName } from '../format'
  import { PRESETS } from '../options'
  import type {
    Collision,
    ConversionJob,
    ConversionOptions,
    FileProbe,
  } from '../ipc'
  import {
    cancelConversion,
    cancelJob as cancelJobCmd,
    checkCollisions,
    inTauri,
    listenConversion,
    listenFileDrop,
    openVideoDialog,
    preflight,
    startConversion,
  } from '../ipc-client'

  type AppPhase = 'idle' | 'reading' | 'ready' | 'converting' | 'done'

  /** One file's view model: its job (mutated by options/rename), the probe
   *  metadata the row reveals, and its live encode state. */
  interface Row {
    job: ConversionJob
    fileName: string
    sizeBytes: number
    probe: FileProbe
    rowPhase: FileRowPhase
    percent: number
    outputPath: string | null
    errorMessage: string | null
  }

  let phase = $state<AppPhase>('idle')
  let rows = $state<Row[]>([])
  let dropActive = $state(false)
  let dropError = $state<string | null>(null)

  // A drop is probing (pre-flight). Tracked apart from `phase` because an append
  // probe leaves the loaded list (and its phase) on screen (ADR-0025); only an
  // empty-state drop flips `phase` to 'reading' for the drop-zone spinner.
  let reading = $state(false)

  // Transient status-bar override (ADR-0025): a ~2.5s notice that supplants the
  // derived status line, then auto-reverts. Fired when a drop yields no new file.
  let notice = $state<string | null>(null)
  let noticeTimer: ReturnType<typeof setTimeout> | null = null

  // The batch's shared options (ADR-0007); applied to every job at convert.
  let options = $state<ConversionOptions>({ ...PRESETS.PRESENTATION })

  // Collisions detected fresh at convert time (ADR-0025), surfaced through the
  // modal before any encode runs.
  let collisions = $state<Collision[]>([])
  let showCollisions = $state(false)

  // Converting/done bookkeeping.
  let activeIndex = $state(0)
  let succeeded = $state(0)
  let failed = $state(0)
  let elapsedSecs = $state(0)
  let startedAt = 0
  let timer: ReturnType<typeof setInterval> | null = null

  const busy = $derived(reading || phase === 'converting')
  const totalBytes = $derived(rows.reduce((sum, r) => sum + r.sizeBytes, 0))

  // Cancelled rows, derived from row phases (ADR-0025) — the frontend's truth,
  // since the backend's done summary counts only succeeded/failed.
  const cancelledCount = $derived(
    rows.filter((r) => r.rowPhase === 'cancelled').length,
  )

  /** Overall batch progress (ADR-0020): settled files (done / error /
   *  cancelled) count as full, the active one by its live percent, queued as
   *  zero. */
  const overallPercent = $derived(
    rows.length === 0
      ? 0
      : rows.reduce(
          (sum, r) =>
            sum +
            (r.rowPhase === 'done' ||
            r.rowPhase === 'error' ||
            r.rowPhase === 'cancelled'
              ? 100
              : r.rowPhase === 'converting'
                ? r.percent
                : 0),
          0,
        ) / rows.length,
  )

  const status = $derived<StatusBarStatus>(
    phase === 'converting'
      ? { state: 'converting', percent: overallPercent, elapsedSecs }
      : phase === 'done'
        ? { state: 'done', succeeded, failed, cancelled: cancelledCount }
        : phase === 'ready'
          ? { state: 'ready', fileCount: rows.length, totalBytes }
          : { state: 'idle' },
  )

  // --- File-list fade (ADR-0023) ------------------------------------------
  // The list hides its scrollbar; a mask-image gradient fades the top/bottom
  // edges. Each edge's gradient stop drops at its extreme — no top fade at the
  // top, no bottom fade at the bottom, neither when the list isn't scrollable.
  let listEl = $state<HTMLDivElement | null>(null)
  let fadeTop = $state(false)
  let fadeBottom = $state(false)

  function updateFade() {
    const el = listEl
    if (!el) return
    fadeTop = el.scrollTop > 1
    fadeBottom = el.scrollTop + el.clientHeight < el.scrollHeight - 1
  }

  // Re-measure whenever the list's layout-affecting state changes — row count,
  // and each row's phase/error (both alter row height). Runs post-DOM.
  $effect(() => {
    void rows.length
    for (const r of rows) {
      void r.rowPhase
      void r.errorMessage
    }
    if (listEl) updateFade()
  })

  const maskStyle = $derived.by(() => {
    const top = fadeTop ? '24px' : '0'
    const bottom = fadeBottom ? 'calc(100% - 24px)' : '100%'
    const g = `linear-gradient(to bottom, transparent 0, #000 ${top}, #000 ${bottom}, transparent 100%)`
    return `-webkit-mask-image: ${g}; mask-image: ${g};`
  })

  // --- Pre-flight ---------------------------------------------------------

  async function handleDrop(paths: string[]) {
    if (busy || showCollisions) return // ignore drops mid-batch / mid-modal
    // Append-on-drop (ADR-0025): a drop merges into the current batch; the list
    // only ever grows here and is trimmed elsewhere by removing rows. An empty
    // start flips `phase` to 'reading' for the drop-zone spinner; an append
    // leaves the loaded list on screen and uses `reading` for the busy guard.
    const startingEmpty = rows.length === 0
    if (startingEmpty) {
      dropError = null
      phase = 'reading'
    }
    reading = true
    try {
      const result = await preflight(paths)
      // Deduplicate by source_path — re-dropping a file is a no-op, so a done
      // row stays done with its badge.
      const known = rows.map((r) => r.job.source_path)
      const additions: Row[] = []
      result.plan.forEach((job, i) => {
        if (known.includes(job.source_path)) return
        known.push(job.source_path)
        additions.push({
          job,
          fileName: baseName(job.source_path),
          sizeBytes: result.files[i]?.size_bytes ?? 0,
          probe: result.files[i],
          rowPhase: 'ready' as FileRowPhase,
          percent: 0,
          outputPath: null,
          errorMessage: null,
        })
      })
      if (additions.length === 0) {
        // No convertible video, or every file is already loaded.
        if (startingEmpty) {
          phase = 'idle'
          dropError = 'No convertible video found'
        } else {
          showNotice('No convertible video found')
        }
        return
      }
      // Dropping into a finished batch returns the phase to ready; existing
      // done/error/cancelled rows keep their badges until the next Convert.
      rows = [...rows, ...additions]
      phase = 'ready'
    } catch (e) {
      if (startingEmpty) {
        phase = 'idle'
        dropError = String(e)
      } else {
        showNotice(String(e))
      }
    } finally {
      reading = false
    }
  }

  /** Show a transient status-bar notice (~2.5s) that overrides the derived
   *  status line, then auto-reverts (ADR-0025). */
  function showNotice(message: string) {
    notice = message
    if (noticeTimer) clearTimeout(noticeTimer)
    noticeTimer = setTimeout(() => {
      notice = null
      noticeTimer = null
    }, 2500)
  }

  /** Click-to-browse (ADR-0023): open the file dialog and feed any selection
   *  through the same pre-flight path as a drop. No-op outside Tauri / mid-batch. */
  async function browse() {
    if (busy || showCollisions) return
    const paths = await openVideoDialog()
    if (paths.length > 0) handleDrop(paths)
  }

  // --- Convert / collisions ----------------------------------------------

  async function onConvert() {
    // Collisions are checked fresh against disk at convert time (ADR-0025), so a
    // re-convert of a finished batch routes through the modal instead of
    // silently overwriting. The whole current batch is re-checked, so an
    // appended batch is covered for free (job_index aligns to row index).
    const found = await checkCollisions(rows.map((r) => r.job))
    if (found.length > 0) {
      collisions = found
      showCollisions = true
      return
    }
    collisions = []
    beginBatch(rows)
  }

  /** Resolve modal choices into the final plan: Cancel drops the job, Rename
   *  retargets the output to a free `_N` name, Override keeps it (the encoder's
   *  atomic rename replaces the existing file). `choices` is keyed by
   *  `job_index`, which equals the row's position in the current order. */
  function resolveCollisions(choices: Record<number, CollisionChoice>) {
    showCollisions = false
    const taken = new Set(rows.map((r) => r.job.output_path))
    const kept: Row[] = []
    rows.forEach((row, i) => {
      const choice = choices[i] // undefined for rows that had no collision
      if (choice === 'cancel') return // drop from the batch (skip, continue)
      if (choice === 'rename') {
        const renamed = renameTarget(row.job.output_path, taken)
        taken.add(renamed)
        row.job = { ...row.job, output_path: renamed }
      }
      kept.push(row)
    })
    collisions = []
    if (kept.length === 0) {
      reset() // every file was cancelled — nothing to do
      return
    }
    beginBatch(kept)
  }

  /** First `<stem>_ppt_N.mp4` (N≥2) not already spoken for. Avoids the known
   *  existing outputs and every other job's output; an unrelated pre-existing
   *  `_N` file on disk is the one residual edge (v1). */
  function renameTarget(outputPath: string, taken: Set<string>): string {
    const at = outputPath.toLowerCase().lastIndexOf('.mp4')
    const base = at >= 0 ? outputPath.slice(0, at) : outputPath
    const ext = at >= 0 ? outputPath.slice(at) : '.mp4'
    let n = 2
    let candidate = `${base}_${n}${ext}`
    while (taken.has(candidate)) candidate = `${base}_${++n}${ext}`
    return candidate
  }

  function dismissCollisions() {
    // Cancel the conversion attempt but keep the loaded files (ADR-0014): no
    // encoding begins; the user can adjust and retry or clear.
    showCollisions = false
  }

  // --- Batch lifecycle ----------------------------------------------------

  function beginBatch(batch: Row[]) {
    for (const r of batch) {
      r.job = { ...r.job, options: { ...options } } // apply chosen options (I2)
      r.rowPhase = 'ready'
      r.percent = 0
      r.outputPath = null
      r.errorMessage = null
    }
    rows = batch // keep rows 1:1 with the plan we send (file_index alignment)
    succeeded = 0
    failed = 0
    activeIndex = 0
    phase = 'converting'
    startTimer()
    startConversion(batch.map((r) => r.job)).catch((e) => {
      // The whole batch failed to start — surface it on every row.
      stopTimer()
      for (const r of rows) {
        r.rowPhase = 'error'
        r.errorMessage = String(e)
      }
      failed = rows.length
      succeeded = 0
      phase = 'done'
    })
  }

  function onCancel() {
    cancelConversion() // the encoder kills FFmpeg; we transition on `cancelled`
  }

  /** Cancel a single job mid-batch (ADR-0025). Marks the row 'cancelled'
   *  optimistically — a queued cancel shows instantly — then asks the backend;
   *  the `file_cancelled` event is confirmation. Cancelled rows are kept in
   *  place (never removed) so the encoder's file_index routing stays valid, and
   *  re-run on the next Convert (beginBatch resets every row to ready). */
  function cancelJob(index: number) {
    const r = rows[index]
    if (!r || (r.rowPhase !== 'ready' && r.rowPhase !== 'converting')) return
    r.rowPhase = 'cancelled'
    r.percent = 0
    cancelJobCmd(index)
  }

  function reset() {
    stopTimer()
    phase = 'idle'
    rows = []
    collisions = []
    showCollisions = false
    dropError = null
    succeeded = 0
    failed = 0
    activeIndex = 0
  }

  /** Remove a row from the batch (ADR-0023). Phase-gated to non-converting so
   *  the encoder's `file_index` keeps pointing at the plan we sent. The last
   *  row returning to the empty drop zone; removing while `done` recomputes the
   *  status summary from the survivors. */
  function removeRow(index: number) {
    if (phase === 'converting') return
    rows = rows.filter((_, i) => i !== index)
    if (rows.length === 0) {
      reset()
      return
    }
    if (phase === 'done') {
      succeeded = rows.filter((r) => r.rowPhase === 'done').length
      failed = rows.filter((r) => r.rowPhase === 'error').length
    }
  }

  function startTimer() {
    startedAt = Date.now()
    elapsedSecs = 0
    timer = setInterval(() => {
      elapsedSecs = (Date.now() - startedAt) / 1000
    }, 500)
  }

  function stopTimer() {
    if (timer) {
      clearInterval(timer)
      timer = null
    }
  }

  // --- IPC wiring ---------------------------------------------------------

  onMount(() => {
    // The gallery harness renders the shell in a plain browser tab — no Tauri
    // runtime, so skip all IPC wiring there.
    if (!inTauri) return
    const unlisteners: Array<() => void> = []
    listenFileDrop({
      onActive: (active) => (dropActive = active && !busy && !showCollisions),
      onDrop: handleDrop,
    }).then((un) => unlisteners.push(un))
    listenConversion({
      onProgress: (e) => {
        if (phase !== 'converting') return
        activeIndex = e.file_index
        const r = rows[e.file_index]
        // A row cancelled mid-flight stays cancelled — ignore late progress for
        // it (ADR-0025).
        if (r && r.rowPhase !== 'cancelled') {
          r.rowPhase = 'converting'
          r.percent = e.percent
        }
      },
      onFileDone: (e) => {
        const r = rows[e.file_index]
        if (r) {
          r.rowPhase = 'done'
          r.percent = 100
          r.outputPath = e.output_path
        }
      },
      onFileError: (e) => {
        const r = rows[e.file_index]
        if (r) {
          r.rowPhase = 'error'
          r.errorMessage = e.error_message
        }
      },
      onFileCancelled: (e) => {
        // Confirmation of a per-job cancel (ADR-0025); idempotent with the
        // optimistic mark in cancelJob.
        const r = rows[e.file_index]
        if (r) {
          r.rowPhase = 'cancelled'
          r.percent = 0
        }
      },
      onDone: (e) => {
        succeeded = e.succeeded
        failed = e.failed
        phase = 'done'
        stopTimer()
      },
      onCancelled: () => {
        // Whole-batch abort (ADR-0025): the interrupted active row reads as
        // 'cancelled' (truthful), queued rows stay 'ready'; every finished
        // output is kept (ADR-0013).
        const r = rows[activeIndex]
        if (r && r.rowPhase === 'converting') {
          r.rowPhase = 'cancelled'
          r.percent = 0
        }
        succeeded = rows.filter((x) => x.rowPhase === 'done').length
        failed = rows.filter((x) => x.rowPhase === 'error').length
        phase = 'done'
        stopTimer()
      },
    }).then((un) => unlisteners.push(un))
    return () => unlisteners.forEach((un) => un())
  })

  onDestroy(() => {
    stopTimer()
    if (noticeTimer) clearTimeout(noticeTimer)
  })
</script>

<div
  class="relative flex h-full w-full flex-col border-[1.5px] border-ink bg-paper text-ink"
>
  <TitleBar />

  <!-- Two-panel split. min-h-0 lets panels own their own overflow. -->
  <main class="flex min-h-0 flex-1">
    <!-- Left: drop target when empty, file list once files are loaded. -->
    <section class="flex min-w-0 flex-1 flex-col bg-surface p-3">
      {#if rows.length === 0}
        <!-- Empty state doubles as a click-to-browse button (ADR-0023): drag-drop
             still works at the window level; folders stay drag-only. Block
             children are spans so the markup stays valid inside <button>. -->
        <button
          type="button"
          onclick={browse}
          disabled={phase === 'reading'}
          class="flex flex-1 flex-col items-center justify-center gap-4 border-[1.5px]
                 text-center outline-none focus-visible:outline-2
                 focus-visible:-outline-offset-2 focus-visible:outline-accent
                 disabled:cursor-default {dropActive
            ? 'border-solid border-accent bg-accent/5'
            : 'cursor-pointer border-dashed border-accent hover:bg-accent/5'}"
        >
          <span
            class="flex h-11 w-11 items-center justify-center border-[1.5px]
                   border-accent font-mono text-xl leading-none text-accent"
            aria-hidden="true">↑</span
          >
          <span class="block px-4">
            {#if phase === 'reading'}
              <span
                class="block font-mono text-[11px] font-medium tracking-[0.12em]
                       text-ink uppercase"
              >
                Reading…
              </span>
              <span class="mt-1.5 block text-[12px] text-muted"
                >Probing dropped files</span
              >
            {:else}
              <span
                class="block font-mono text-[11px] font-medium tracking-[0.12em]
                       text-ink uppercase"
              >
                Drop video or folder
              </span>
              <span
                class="mt-1.5 block text-[12px] {dropError
                  ? 'text-danger'
                  : 'text-muted'}"
              >
                {dropError ?? 'Click to browse, or drop a video or folder'}
              </span>
            {/if}
          </span>
        </button>
      {:else}
        <!-- File list (ADR-0020). One row per candidate; the row owns its own
             phased reveal driven by the live event stream. -->
        <div
          bind:this={listEl}
          onscroll={updateFade}
          style={maskStyle}
          class="scrollbar-none flex min-h-0 flex-1 flex-col gap-2.5 overflow-auto"
        >
          {#each rows as row, i (row.job.source_path)}
            <FileRow
              fileName={row.fileName}
              sizeBytes={row.sizeBytes}
              probe={row.probe}
              phase={row.rowPhase}
              percent={row.percent}
              outputPath={row.outputPath}
              errorMessage={row.errorMessage}
              converting={phase === 'converting'}
              onRemove={() => removeRow(i)}
              onCancelJob={() => cancelJob(i)}
            />
          {/each}
        </div>
      {/if}
    </section>

    <!-- Right: options + the pinned rail-footer action (ADR-0024). The options
         are dimmed/inert until a file is loaded (ADR-0020) and locked during an
         in-flight batch; the footer button is a sibling (not inside the panel)
         so it stays live as Cancel while the panel is disabled mid-encode. -->
    <aside
      class="flex w-[330px] shrink-0 flex-col border-l-[1.5px] border-ink
             bg-surface"
    >
      <div class="min-h-0 flex-1 overflow-auto p-4">
        <OptionsPanel
          bind:options
          disabled={rows.length === 0 || phase === 'converting'}
        />
      </div>
      <!-- Single morphing action slot, pinned below the knobs it acts on. -->
      <div class="shrink-0 p-4">
        {#if phase === 'converting'}
          <Button
            variant="danger"
            size="lg"
            class="w-full justify-center"
            onclick={onCancel}>Cancel</Button
          >
        {:else}
          <Button
            variant="primary"
            size="lg"
            class="w-full justify-center"
            disabled={rows.length === 0}
            onclick={onConvert}>Convert</Button
          >
        {/if}
      </div>
    </aside>
  </main>

  <!-- Full-width status-only bar (ADR-0019 / ADR-0024): batch state + the
       global progress fill; the action lives in the rail footer above. -->
  <StatusBar {status} {notice} />

  <!-- Collision resolution (ADR-0014): blocks the encode until resolved. -->
  {#if showCollisions}
    <CollisionModal
      {collisions}
      onConfirm={resolveCollisions}
      onDismiss={dismissCollisions}
    />
  {/if}
</div>
