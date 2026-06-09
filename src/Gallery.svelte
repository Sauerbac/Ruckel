<!--
  Dev-only gallery harness (?gallery). Mounted by main.ts when
  `import.meta.env.DEV && location.search` includes `gallery`. It shows the
  v2 design-system primitives in their states plus the empty shell so the locked
  look (ADR-0018/0019/0020) can be eyeballed and screenshot-checked.
-->
<script lang="ts">
  import Button from './lib/components/Button.svelte'
  import CollisionModal from './lib/components/CollisionModal.svelte'
  import FileRow from './lib/components/FileRow.svelte'
  import OptionsPanel from './lib/components/OptionsPanel.svelte'
  import Panel from './lib/components/Panel.svelte'
  import SegmentedControl from './lib/components/SegmentedControl.svelte'
  import StatusBar from './lib/components/StatusBar.svelte'
  import Shell from './lib/shell/Shell.svelte'
  import type { Collision, ConversionOptions, FileProbe } from './lib/ipc'
  import { PRESETS } from './lib/options'

  let quality = $state('PRESENTATION')
  let resolution = $state(1080)

  // --- F2 mock props (no live IPC; every state is driven from these). ---
  const probe: FileProbe = {
    duration_secs: 754,
    width: 1920,
    height: 1080,
    size_bytes: 260_046_848,
  }

  // OptionsPanel: one interactive instance on a preset, one on a Custom mix,
  // and one rendered in its disabled empty state.
  let presetOpts = $state<ConversionOptions>({ ...PRESETS.PRESENTATION })
  let customOpts = $state<ConversionOptions>({
    resolution: 'P1080',
    crf: 23,
    framerate: 'Fps60',
    audio: 'Kbps128',
  })
  let emptyOpts = $state<ConversionOptions>({ ...PRESETS.PRESENTATION })

  const collisions: Collision[] = [
    {
      job_index: 0,
      source_path: 'C:\\Users\\Simon\\Videos\\lecture_03.mp4',
      output_path: 'C:\\Users\\Simon\\Videos\\lecture_03_ppt.mp4',
    },
    {
      job_index: 1,
      source_path: 'C:\\Users\\Simon\\Videos\\keynote_intro.mov',
      output_path: 'C:\\Users\\Simon\\Videos\\keynote_intro_ppt.mp4',
    },
  ]

  const swatches = [
    { name: 'paper', hex: '#EDEBE6' },
    { name: 'surface', hex: '#FBFAF8' },
    { name: 'fill', hex: '#E4E1DA' },
    { name: 'ink', hex: '#16150F' },
    { name: 'ink-2', hex: '#3A382F' },
    { name: 'muted', hex: '#76736A' },
    { name: 'line', hex: '#C8C4B9' },
    { name: 'accent', hex: '#1FB866' },
    { name: 'accent-d', hex: '#159152' },
    { name: 'warning', hex: '#D9821A' },
    { name: 'danger', hex: '#C8341D' },
  ]
</script>

<div class="min-h-screen overflow-auto bg-paper px-8 py-8 text-ink">
  <header class="mb-8">
    <h1 class="text-2xl font-semibold tracking-tight">Ruckel — Gallery</h1>
    <p class="mt-1 font-mono text-[12px] text-muted">
      v2 utilitarian neo-brutalism · primitives + empty shell (ADR-0018 / 0019 /
      0020)
    </p>
  </header>

  <!-- Typography proof: Space Grotesk for chrome, IBM Plex Mono for data. -->
  <section class="mb-10">
    <h2
      class="mb-3 font-mono text-[11px] tracking-[0.14em] text-muted uppercase"
    >
      Typography
    </h2>
    <div class="grid max-w-2xl grid-cols-2 gap-4">
      <Panel class="p-4">
        <p class="text-xs text-muted">Chrome — Space Grotesk</p>
        <p class="mt-1 text-lg">Drop video or folder</p>
      </Panel>
      <Panel class="p-4">
        <p class="text-xs text-muted">Data — IBM Plex Mono</p>
        <p class="mt-1 font-mono text-lg">lecture_03_ppt.mp4 · 248 MB</p>
      </Panel>
    </div>
  </section>

  <!-- Palette swatches (ADR-0018 v2). -->
  <section class="mb-10">
    <h2
      class="mb-3 font-mono text-[11px] tracking-[0.14em] text-muted uppercase"
    >
      Palette
    </h2>
    <div class="flex flex-wrap gap-3 font-mono text-xs">
      {#each swatches as s (s.name)}
        <div class="w-28">
          <div
            class="h-14 border-[1.5px] border-ink"
            style="background-color: {s.hex}"
          ></div>
          <p class="mt-1 text-ink">{s.name}</p>
          <p class="text-muted">{s.hex}</p>
        </div>
      {/each}
    </div>
  </section>

  <!-- Status signals: the reserved-green triad + the live pulse. -->
  <section class="mb-10">
    <h2
      class="mb-3 font-mono text-[11px] tracking-[0.14em] text-muted uppercase"
    >
      Status signals
    </h2>
    <div class="flex flex-wrap items-center gap-3">
      <span
        class="inline-flex items-center gap-[6px] border-[1.5px] border-ink
               bg-accent px-2 py-[3px] font-mono text-[10px] tracking-[0.08em]
               text-ink uppercase"
      >
        <span class="h-[7px] w-[7px] bg-ink"></span>Done
      </span>
      <span
        class="inline-flex items-center gap-[6px] border-[1.5px] border-ink
               bg-warning px-2 py-[3px] font-mono text-[10px] tracking-[0.08em]
               text-ink uppercase"
      >
        <span class="h-[7px] w-[7px] bg-ink"></span>Skipped
      </span>
      <span
        class="inline-flex items-center gap-[6px] border-[1.5px] border-ink
               bg-danger px-2 py-[3px] font-mono text-[10px] tracking-[0.08em]
               text-paper uppercase"
      >
        <span class="h-[7px] w-[7px] bg-paper"></span>Error
      </span>
      <span
        class="inline-flex items-center gap-[8px] border-[1.5px] border-ink
               bg-ink px-2 py-[3px] font-mono text-[10px] tracking-[0.08em]
               text-accent uppercase"
      >
        <span
          class="h-[8px] w-[8px] bg-accent"
          style="animation: blink 1.6s steps(1) infinite"
        ></span>Converting
      </span>
    </div>
  </section>

  <!-- Flat bordered progress bar (signature element). -->
  <section class="mb-10">
    <h2
      class="mb-3 font-mono text-[11px] tracking-[0.14em] text-muted uppercase"
    >
      Progress
    </h2>
    <div class="flex max-w-md flex-col gap-3">
      <div class="relative h-3 border-[1.5px] border-ink bg-surface">
        <div
          class="absolute inset-0 border-r-[1.5px] border-ink bg-accent"
          style="right: 32%"
        ></div>
      </div>
      <div class="relative h-3 border-[1.5px] border-ink bg-surface">
        <div class="absolute inset-0 bg-ink"></div>
      </div>
    </div>
  </section>

  <!-- Button primitive — every variant + disabled. -->
  <section class="mb-10">
    <h2
      class="mb-3 font-mono text-[11px] tracking-[0.14em] text-muted uppercase"
    >
      Buttons
    </h2>
    <div class="flex flex-wrap items-center gap-3">
      <Button variant="primary">Convert</Button>
      <Button variant="default">Default</Button>
      <Button variant="danger">Cancel</Button>
      <Button variant="primary" disabled>Convert</Button>
    </div>
    <!-- Large size — the pinned rail-footer action (ADR-0024). -->
    <div class="mt-3 flex max-w-[300px] flex-col gap-3">
      <Button variant="primary" size="lg" class="w-full justify-center"
        >Convert</Button
      >
      <Button variant="danger" size="lg" class="w-full justify-center"
        >Cancel</Button
      >
      <Button variant="primary" size="lg" disabled class="w-full justify-center"
        >Convert</Button
      >
    </div>
  </section>

  <!-- Panel primitive. -->
  <section class="mb-10">
    <h2
      class="mb-3 font-mono text-[11px] tracking-[0.14em] text-muted uppercase"
    >
      Panel
    </h2>
    <Panel class="max-w-md p-4">
      <p class="text-sm">
        A flat surface with a heavy 1.5px ink border and sharp corners — the
        container everything sits inside.
      </p>
    </Panel>
  </section>

  <!-- Segmented-control primitive (interactive). Selected inverts to ink. -->
  <section class="mb-10">
    <h2
      class="mb-3 font-mono text-[11px] tracking-[0.14em] text-muted uppercase"
    >
      Segmented control
    </h2>
    <div class="flex flex-col items-start gap-3">
      <SegmentedControl
        bind:value={quality}
        options={[
          { label: 'PRESENTATION', value: 'PRESENTATION' },
          { label: 'HIGH', value: 'HIGH' },
          { label: 'COMPACT', value: 'COMPACT' },
        ]}
      />
      <SegmentedControl
        bind:value={resolution}
        options={[
          { label: '720', value: 720 },
          { label: '1080', value: 1080 },
          { label: 'ORIG', value: 0 },
        ]}
      />
      <SegmentedControl
        value={1080}
        disabled
        options={[
          { label: '720', value: 720 },
          { label: '1080', value: 1080 },
          { label: 'ORIG', value: 0 },
        ]}
      />
    </div>
  </section>

  <!-- ============================================================== -->
  <!-- F2 — feature components, every locked state from mock props.    -->
  <!-- ============================================================== -->

  <!-- File row — the ADR-0020 phased reveal, one row per phase. The ✕ (ADR-0025)
       is always rendered and context-aware: Remove when idle, Cancel-this-job
       mid-batch, styled-inert for a settled row mid-batch. -->
  <section class="mb-10">
    <h2
      class="mb-3 font-mono text-[11px] tracking-[0.14em] text-muted uppercase"
    >
      File row — phased reveal · context-aware ✕
    </h2>
    <div class="flex max-w-md flex-col gap-3">
      <FileRow
        fileName="lecture_03.mp4"
        sizeBytes={260_046_848}
        phase="dropped"
        onRemove={() => {}}
      />
      <FileRow
        fileName="lecture_03.mp4"
        sizeBytes={260_046_848}
        {probe}
        phase="ready"
        onRemove={() => {}}
      />
      <FileRow
        fileName="lecture_03.mp4"
        sizeBytes={260_046_848}
        {probe}
        phase="converting"
        percent={40}
        onRemove={() => {}}
      />
      <FileRow
        fileName="lecture_03.mp4"
        sizeBytes={260_046_848}
        {probe}
        phase="done"
        outputPath="C:\Users\Simon\Videos\lecture_03_ppt.mp4"
        onRemove={() => {}}
      />
      <FileRow
        fileName="keynote_intro.mov"
        sizeBytes={88_080_384}
        {probe}
        phase="error"
        errorMessage="ffmpeg exited 1: Unsupported codec (hevc) in stream 0"
        onRemove={() => {}}
      />
      <FileRow
        fileName="webinar_q2.mkv"
        sizeBytes={141_557_760}
        {probe}
        phase="cancelled"
        onRemove={() => {}}
      />
    </div>
    <!-- Mid-batch ✕ contexts (ADR-0025): Cancel-this-job vs styled-inert. -->
    <p
      class="mt-4 mb-2 font-mono text-[10px] tracking-[0.1em] text-muted uppercase"
    >
      Mid-batch (converting) — ✕ becomes Cancel / inert
    </p>
    <div class="flex max-w-md flex-col gap-3">
      <FileRow
        fileName="lecture_03.mp4"
        sizeBytes={260_046_848}
        {probe}
        phase="converting"
        percent={40}
        converting
        onRemove={() => {}}
        onCancelJob={() => {}}
      />
      <FileRow
        fileName="lecture_03.mp4"
        sizeBytes={260_046_848}
        {probe}
        phase="done"
        outputPath="C:\Users\Simon\Videos\lecture_03_ppt.mp4"
        converting
        onRemove={() => {}}
        onCancelJob={() => {}}
      />
    </div>
  </section>

  <!-- Right-panel options — empty (disabled), a preset, and a Custom mix. -->
  <section class="mb-10">
    <h2
      class="mb-3 font-mono text-[11px] tracking-[0.14em] text-muted uppercase"
    >
      Options panel — empty · preset · custom
    </h2>
    <div class="flex flex-wrap gap-6">
      <div class="w-[300px]">
        <p class="mb-2 font-mono text-[10px] text-muted uppercase">
          Empty (disabled)
        </p>
        <OptionsPanel bind:options={emptyOpts} disabled />
      </div>
      <div class="w-[300px]">
        <p class="mb-2 font-mono text-[10px] text-muted uppercase">
          Preset selected
        </p>
        <OptionsPanel bind:options={presetOpts} />
      </div>
      <div class="w-[300px]">
        <p class="mb-2 font-mono text-[10px] text-muted uppercase">
          Custom (no preset match)
        </p>
        <OptionsPanel bind:options={customOpts} />
      </div>
    </div>
  </section>

  <!-- Status-only bar (ADR-0024) — all four states; the action moved to the
       rail footer, so the bar carries state + the converting progress fill only. -->
  <section class="mb-10">
    <h2
      class="mb-3 font-mono text-[11px] tracking-[0.14em] text-muted uppercase"
    >
      Status bar — idle · ready · converting · done · notice
    </h2>
    <div class="flex max-w-2xl flex-col gap-3">
      <StatusBar status={{ state: 'idle' }} />
      <StatusBar
        status={{ state: 'ready', fileCount: 3, totalBytes: 580_911_104 }}
      />
      <StatusBar
        status={{ state: 'converting', percent: 62, elapsedSecs: 95 }}
      />
      <StatusBar
        status={{ state: 'done', succeeded: 2, failed: 1, cancelled: 1 }}
      />
      <StatusBar
        status={{ state: 'idle' }}
        notice="No convertible video found"
      />
    </div>
  </section>

  <!-- Collision modal — framed in a bounded box so the scrim is contained. -->
  <section class="mb-10">
    <h2
      class="mb-3 font-mono text-[11px] tracking-[0.14em] text-muted uppercase"
    >
      Collision modal
    </h2>
    <div
      class="relative h-[420px] w-[680px] overflow-hidden border-[1.5px] border-line bg-paper"
    >
      <CollisionModal {collisions} />
    </div>
  </section>

  <!-- Empty shell, framed at its true 860x560 fixed size (ADR-0019). -->
  <section class="mb-10">
    <h2
      class="mb-3 font-mono text-[11px] tracking-[0.14em] text-muted uppercase"
    >
      Empty shell — 860 × 560
    </h2>
    <div class="h-[560px] w-[860px] overflow-hidden">
      <Shell />
    </div>
  </section>
</div>
