<!--
  Application shell (ADR-0019 v2 layout) + the S2 tracer wiring. A 1.5px ink
  outer frame around a solid-ink title bar, a two-panel split (left: file drop +
  list / right: options), and a full-width ink status/action bar.

  The tracer (S2) drives the thinnest end-to-end path on the real left panel:
  drop one file → real `preflight` (ffprobe) → one-job plan → `start_conversion`
  (real ffmpeg, PowerPoint-safe / Presentation) → `conversion:progress` advances
  a real bordered progress bar → `done`. The right-panel options stay in their
  F1 DISABLED empty-state treatment — wiring presets/options is I2; the full
  multi-file phased reveal and error/collision/cancel UX arrive in later slices.
-->
<script lang="ts">
  import { onMount } from 'svelte'
  import TitleBar from './TitleBar.svelte'
  import Button from '../components/Button.svelte'
  import SegmentedControl from '../components/SegmentedControl.svelte'
  import {
    inTauri,
    listenConversion,
    listenFileDrop,
    preflight,
    startConversion,
  } from '../ipc-client'

  // Display-only values for the inert empty-state options (no binding in F1/S2).
  let preset = $state('PRESENTATION')
  let resolution = $state('ORIG')
  let quality = $state('CRF 23')
  let framerate = $state('ORIG')
  let audio = $state('128K')

  // Tracer state — one file through its phases (ADR-0020 phased reveal).
  type Phase = 'idle' | 'preflight' | 'converting' | 'done' | 'error'
  let phase = $state<Phase>('idle')
  let fileName = $state<string | null>(null)
  let percent = $state(0)
  let outputPath = $state<string | null>(null)
  let errorMessage = $state<string | null>(null)
  let dropActive = $state(false)

  const ext = $derived(
    fileName?.includes('.') ? fileName.split('.').pop()! : '',
  )
  const busy = $derived(phase === 'preflight' || phase === 'converting')

  function baseName(path: string): string {
    return path.split(/[\\/]/).pop() ?? path
  }

  function reset() {
    phase = 'idle'
    fileName = null
    percent = 0
    outputPath = null
    errorMessage = null
  }

  async function handleDrop(paths: string[]) {
    if (busy) return // ignore drops mid-batch (multi-file is I1)
    const path = paths[0]
    if (!path) return

    fileName = baseName(path)
    percent = 0
    outputPath = null
    errorMessage = null
    phase = 'preflight'

    try {
      const { plan } = await preflight([path])
      if (plan.length === 0) {
        phase = 'error'
        errorMessage = 'No convertible video found'
        return
      }
      phase = 'converting'
      await startConversion(plan)
    } catch (e) {
      phase = 'error'
      errorMessage = String(e)
    }
  }

  onMount(() => {
    // The gallery harness renders the shell in a plain browser tab — no Tauri
    // runtime, so skip all IPC wiring there.
    if (!inTauri) return
    const unlisteners: Array<() => void> = []
    listenFileDrop({
      onActive: (active) => (dropActive = active && !busy),
      onDrop: handleDrop,
    }).then((un) => unlisteners.push(un))
    listenConversion({
      onProgress: (e) => {
        if (phase === 'converting') percent = e.percent
      },
      onFileDone: (e) => {
        outputPath = e.output_path
      },
      onFileError: (e) => {
        phase = 'error'
        errorMessage = e.error_message
      },
      onDone: (e) => {
        // Tracer is single-file: a clean batch lands on done.
        if (phase !== 'error') {
          phase = e.failed > 0 ? 'error' : 'done'
          if (phase === 'done') percent = 100
        }
      },
      onCancelled: reset,
    }).then((un) => unlisteners.push(un))
    return () => unlisteners.forEach((un) => un())
  })
</script>

<div
  class="flex h-full w-full flex-col border-[1.5px] border-ink bg-paper text-ink"
>
  <TitleBar />

  <!-- Two-panel split. min-h-0 lets panels own their own overflow. -->
  <main class="flex min-h-0 flex-1">
    <!-- Left: persistent drop target. Empty = dashed marquee; a dropped file
         takes over with its phased-reveal row. -->
    <section class="flex min-w-0 flex-1 flex-col bg-surface p-3">
      {#if phase === 'idle'}
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
            <p
              class="font-mono text-[11px] font-medium tracking-[0.12em] text-ink
                     uppercase"
            >
              Drop video or folder
            </p>
            <p class="mt-1.5 text-[12px] text-muted">
              Converts to PowerPoint-ready MP4
            </p>
          </div>
        </div>
      {:else}
        <!-- File row (phased reveal). The tracer reveals name+ext → progress →
             done/error; duration/resolution + multi-row land in F2/I-slices. -->
        <div
          class="flex flex-col gap-2.5 border-[1.5px] p-3
                 {phase === 'error'
            ? 'border-l-[3px] border-danger'
            : 'border-ink'}"
        >
          <div class="flex items-center justify-between gap-3">
            <span class="min-w-0 truncate font-mono text-[12px] text-ink"
              >{fileName}</span
            >
            <span class="flex shrink-0 items-center gap-2">
              {#if ext}
                <span
                  class="border border-line px-1.5 py-px font-mono text-[10px]
                         tracking-[0.08em] text-muted uppercase">{ext}</span
                >
              {/if}
              {#if phase === 'done'}
                <span
                  class="inline-flex items-center gap-[6px] border-[1.5px] border-ink
                         bg-accent px-2 py-[3px] font-mono text-[10px]
                         tracking-[0.08em] text-ink uppercase"
                  ><span class="h-[7px] w-[7px] bg-ink"></span>Done</span
                >
              {:else if phase === 'error'}
                <span
                  class="inline-flex items-center gap-[6px] border-[1.5px] border-ink
                         bg-danger px-2 py-[3px] font-mono text-[10px]
                         tracking-[0.08em] text-paper uppercase"
                  ><span class="h-[7px] w-[7px] bg-paper"></span>Error</span
                >
              {/if}
            </span>
          </div>

          {#if phase === 'preflight'}
            <span class="font-mono text-[11px] text-muted">Reading…</span>
          {:else if phase === 'converting'}
            <div class="flex items-center gap-2.5">
              <div
                class="relative h-3 flex-1 border-[1.5px] border-ink bg-surface"
              >
                <div
                  class="absolute inset-0 border-r-[1.5px] border-ink bg-accent"
                  style="right: {100 - Math.min(100, Math.max(0, percent))}%"
                ></div>
              </div>
              <span class="w-9 text-right font-mono text-[11px] text-ink-2"
                >{Math.round(percent)}%</span
              >
            </div>
          {:else if phase === 'done'}
            <span class="truncate font-mono text-[11px] text-muted"
              >{outputPath ?? `${fileName} → _ppt.mp4`}</span
            >
          {:else if phase === 'error'}
            <span class="font-mono text-[11px] leading-snug text-danger"
              >{errorMessage}</span
            >
          {/if}
        </div>
      {/if}
    </section>

    <!-- Right: options. Empty state = disabled controls; no accent = "waiting". -->
    <aside
      class="flex w-[330px] shrink-0 flex-col gap-5 border-l-[1.5px] border-ink
             bg-surface p-4"
      aria-hidden="true"
    >
      <div class="flex flex-col gap-2">
        <span
          class="font-mono text-[10px] tracking-[0.14em] text-muted uppercase"
          >Preset</span
        >
        <SegmentedControl
          bind:value={preset}
          disabled
          options={[
            { label: 'PRESENTATION', value: 'PRESENTATION' },
            { label: 'HIGH', value: 'HIGH' },
            { label: 'COMPACT', value: 'COMPACT' },
          ]}
        />
      </div>

      <div class="flex flex-col gap-2">
        <span
          class="font-mono text-[10px] tracking-[0.14em] text-muted uppercase"
          >Resolution</span
        >
        <SegmentedControl
          bind:value={resolution}
          disabled
          options={[
            { label: '720', value: '720' },
            { label: '1080', value: '1080' },
            { label: 'ORIG', value: 'ORIG' },
          ]}
        />
      </div>

      <div class="flex flex-col gap-2">
        <span
          class="font-mono text-[10px] tracking-[0.14em] text-muted uppercase"
          >Quality</span
        >
        <SegmentedControl
          bind:value={quality}
          disabled
          options={[
            { label: 'CRF 18', value: 'CRF 18' },
            { label: 'CRF 23', value: 'CRF 23' },
            { label: 'CRF 28', value: 'CRF 28' },
          ]}
        />
      </div>

      <div class="flex flex-col gap-2">
        <span
          class="font-mono text-[10px] tracking-[0.14em] text-muted uppercase"
          >Framerate</span
        >
        <SegmentedControl
          bind:value={framerate}
          disabled
          options={[
            { label: '30', value: '30' },
            { label: '60', value: '60' },
            { label: 'ORIG', value: 'ORIG' },
          ]}
        />
      </div>

      <div class="flex flex-col gap-2">
        <span
          class="font-mono text-[10px] tracking-[0.14em] text-muted uppercase"
          >Audio</span
        >
        <SegmentedControl
          bind:value={audio}
          disabled
          options={[
            { label: '96K', value: '96K' },
            { label: '128K', value: '128K' },
            { label: '192K', value: '192K' },
          ]}
        />
      </div>
    </aside>
  </main>

  <!-- Full-width status / action bar (solid-ink block, ADR-0019/0020). -->
  <footer
    class="flex h-11 shrink-0 items-center justify-between bg-ink px-3 text-paper"
  >
    {#if phase === 'idle'}
      <span class="font-mono text-[11px] tracking-[0.04em] text-muted uppercase"
        >No files</span
      >
      <Button variant="primary" disabled>Convert</Button>
    {:else if busy}
      <span
        class="flex items-center gap-[10px] font-mono text-[11px]
               tracking-[0.04em] text-paper uppercase"
      >
        <span
          class="h-[8px] w-[8px] bg-accent"
          style="animation: blink 1.6s steps(1) infinite"
        ></span>
        {phase === 'preflight'
          ? 'Reading'
          : `Converting · ${Math.round(percent)}%`}
      </span>
      <Button variant="primary" disabled>Convert</Button>
    {:else}
      <span
        class="font-mono text-[11px] tracking-[0.04em] uppercase
               {phase === 'error' ? 'text-danger' : 'text-accent'}"
      >
        {phase === 'error' ? '0 done · 1 error' : '1 done · 0 errors'}
      </span>
      <Button variant="default" onclick={reset}>Clear</Button>
    {/if}
  </footer>
</div>
