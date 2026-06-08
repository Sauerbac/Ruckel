<!--
  Dev-only gallery harness (?gallery). Mounted by main.ts when
  `import.meta.env.DEV && location.search` includes `gallery`. It shows the
  v2 design-system primitives in their states plus the empty shell so the locked
  look (ADR-0018/0019/0020) can be eyeballed and screenshot-checked.
-->
<script lang="ts">
  import Button from './lib/components/Button.svelte'
  import Panel from './lib/components/Panel.svelte'
  import SegmentedControl from './lib/components/SegmentedControl.svelte'
  import Shell from './lib/shell/Shell.svelte'

  let quality = $state('PRESENTATION')
  let resolution = $state(1080)

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
      <Button variant="default">Clear</Button>
      <Button variant="danger">Cancel</Button>
      <Button variant="primary" disabled>Convert</Button>
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
