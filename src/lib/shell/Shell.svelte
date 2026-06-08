<!--
  Application shell (ADR-0019 v2 layout) wired to the live IPC contract — the
  Phase 3 feature integration (I1 multi-file · I2 options/presets · I3 collision
  · I4 errors · I5 cancel). A 1.5px ink outer frame around a solid-ink title
  bar, a two-panel split (left: drop target / file list, right: options), and a
  full-width StatusBar action bar.

  The whole app is a small state machine over `phase`:

    idle      empty drop target; options dimmed; "No files"
    reading   pre-flight in flight (scan → probe → collision check)
    ready     N file rows + live options; CONVERT (opens the collision modal
              first when pre-flight found existing outputs)
    converting sequential encode (ADR-0009); the active row shows progress, the
              rest queued/done; CANCEL kills the in-flight job (ADR-0013)
    done      per-file outcomes retained; "N done · N errors"; CLEAR / again

  The frontend owns no encode logic: it sends a resolved Conversion Plan to
  `start_conversion` and reflects the `conversion:*` event stream back onto the
  rows. `file_index` indexes the plan we sent, so `rows` is kept 1:1 with that
  plan (collision-cancelled files are dropped from both together).
-->
<script lang="ts">
  import { onDestroy, onMount } from 'svelte'
  import TitleBar from './TitleBar.svelte'
  import FileRow, { type FileRowPhase } from '../components/FileRow.svelte'
  import OptionsPanel from '../components/OptionsPanel.svelte'
  import StatusBar, { type StatusBarStatus } from '../components/StatusBar.svelte'
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
    inTauri,
    listenConversion,
    listenFileDrop,
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

  // The batch's shared options (ADR-0007); applied to every job at convert.
  let options = $state<ConversionOptions>({ ...PRESETS.PRESENTATION })

  // Collisions detected in pre-flight, surfaced through the modal on Convert.
  let collisions = $state<Collision[]>([])
  let showCollisions = $state(false)

  // Converting/done bookkeeping.
  let activeIndex = $state(0)
  let succeeded = $state(0)
  let failed = $state(0)
  let elapsedSecs = $state(0)
  let startedAt = 0
  let timer: ReturnType<typeof setInterval> | null = null

  const busy = $derived(phase === 'reading' || phase === 'converting')
  const totalBytes = $derived(rows.reduce((sum, r) => sum + r.sizeBytes, 0))

  /** Overall batch progress (ADR-0020): finished files count as full, the
   *  active one by its live percent, queued as zero. */
  const overallPercent = $derived(
    rows.length === 0
      ? 0
      : rows.reduce(
          (sum, r) =>
            sum +
            (r.rowPhase === 'done' || r.rowPhase === 'error'
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
        ? { state: 'done', succeeded, failed }
        : phase === 'ready'
          ? { state: 'ready', fileCount: rows.length, totalBytes }
          : { state: 'idle' },
  )

  // --- Pre-flight ---------------------------------------------------------

  async function handleDrop(paths: string[]) {
    if (busy || showCollisions) return // ignore drops mid-batch / mid-modal
    dropError = null
    rows = []
    collisions = []
    phase = 'reading'
    try {
      const result = await preflight(paths)
      if (result.plan.length === 0) {
        phase = 'idle'
        dropError = 'No convertible video found'
        return
      }
      rows = result.plan.map((job, i) => ({
        job,
        fileName: baseName(job.source_path),
        sizeBytes: result.files[i]?.size_bytes ?? 0,
        probe: result.files[i],
        rowPhase: 'ready' as FileRowPhase,
        percent: 0,
        outputPath: null,
        errorMessage: null,
      }))
      collisions = result.collisions
      phase = 'ready'
    } catch (e) {
      phase = 'idle'
      dropError = String(e)
    }
  }

  // --- Convert / collisions ----------------------------------------------

  function onConvert() {
    // Collisions block the encode until resolved in pre-flight (ADR-0014).
    if (collisions.length > 0) {
      showCollisions = true
      return
    }
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

  function convertAgain() {
    beginBatch(rows)
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
        if (r) {
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
      onDone: (e) => {
        succeeded = e.succeeded
        failed = e.failed
        phase = 'done'
        stopTimer()
      },
      onCancelled: () => {
        // The killed file never finished — return it to the queued look; keep
        // every already-finished output (ADR-0013).
        const r = rows[activeIndex]
        if (r && r.rowPhase === 'converting') {
          r.rowPhase = 'ready'
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

  onDestroy(stopTimer)
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
        <div
          class="flex flex-1 flex-col items-center justify-center gap-4 border-[1.5px]
                 text-center {dropActive
            ? 'border-solid border-accent bg-accent/5'
            : 'border-dashed border-accent'}"
        >
          <span
            class="flex h-11 w-11 items-center justify-center border-[1.5px]
                   border-accent font-mono text-xl leading-none text-accent"
            aria-hidden="true">↑</span
          >
          <div class="px-4">
            {#if phase === 'reading'}
              <p
                class="font-mono text-[11px] font-medium tracking-[0.12em] text-ink
                       uppercase"
              >
                Reading…
              </p>
              <p class="mt-1.5 text-[12px] text-muted">Probing dropped files</p>
            {:else}
              <p
                class="font-mono text-[11px] font-medium tracking-[0.12em] text-ink
                       uppercase"
              >
                Drop video or folder
              </p>
              <p class="mt-1.5 text-[12px] {dropError ? 'text-danger' : 'text-muted'}">
                {dropError ?? 'Converts to PowerPoint-ready MP4'}
              </p>
            {/if}
          </div>
        </div>
      {:else}
        <!-- File list (ADR-0020). One row per candidate; the row owns its own
             phased reveal driven by the live event stream. -->
        <div class="flex min-h-0 flex-1 flex-col gap-2.5 overflow-auto">
          {#each rows as row (row.job.source_path)}
            <FileRow
              fileName={row.fileName}
              sizeBytes={row.sizeBytes}
              probe={row.probe}
              phase={row.rowPhase}
              percent={row.percent}
              outputPath={row.outputPath}
              errorMessage={row.errorMessage}
            />
          {/each}
        </div>
      {/if}
    </section>

    <!-- Right: options. Dimmed/inert until at least one file is loaded
         (ADR-0020); locked during an in-flight batch so the live plan can't
         shift under the encoder. -->
    <aside
      class="flex w-[330px] shrink-0 flex-col overflow-auto border-l-[1.5px]
             border-ink bg-surface p-4"
    >
      <OptionsPanel
        bind:options
        disabled={rows.length === 0 || phase === 'converting'}
      />
    </aside>
  </main>

  <!-- Full-width status / action bar (ADR-0019 / ADR-0020). -->
  <StatusBar
    {status}
    onConvert={phase === 'done' ? convertAgain : onConvert}
    onCancel={onCancel}
    onClear={reset}
  />

  <!-- Collision resolution (ADR-0014): blocks the encode until resolved. -->
  {#if showCollisions}
    <CollisionModal
      {collisions}
      onConfirm={resolveCollisions}
      onDismiss={dismissCollisions}
    />
  {/if}
</div>
