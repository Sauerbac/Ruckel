<!--
  Static application shell (ADR-0019 v2 layout). A 1.5px ink outer frame around a
  solid-ink title bar, a two-panel split (left: file drop + list / right:
  options), and a full-width ink status/action bar spanning both panels. F1
  renders empty states only (ADR-0020): the left panel shows the dashed-green
  drop marquee; the right panel shows the option controls in their DISABLED
  treatment (fill/line/muted, no accent — the absence of green is the "not ready"
  signal). Data-bound rows, live counts, and the converting pulse arrive in F2.
-->
<script lang="ts">
  import TitleBar from './TitleBar.svelte'
  import Button from '../components/Button.svelte'
  import SegmentedControl from '../components/SegmentedControl.svelte'

  // Display-only values for the inert empty-state controls (no binding in F1).
  let preset = $state('PRESENTATION')
  let resolution = $state('1080')
  let quality = $state('CRF 23')
  let framerate = $state('ORIG')
  let audio = $state('128K')
</script>

<div
  class="flex h-full w-full flex-col border-[1.5px] border-ink bg-paper text-ink"
>
  <TitleBar />

  <!-- Two-panel split. min-h-0 lets panels own their own overflow. -->
  <main class="flex min-h-0 flex-1">
    <!-- Left: persistent drop target. Empty state = dashed-green marquee. -->
    <section class="flex min-w-0 flex-1 flex-col bg-surface p-3">
      <div
        class="flex flex-1 flex-col items-center justify-center gap-4
               border-[1.5px] border-dashed border-accent text-center"
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

  <!-- Full-width status / action bar. Empty/ready frame (solid-ink block). -->
  <footer
    class="flex h-11 shrink-0 items-center justify-between bg-ink px-3 text-paper"
  >
    <span class="font-mono text-[11px] tracking-[0.04em] text-muted uppercase">
      No files
    </span>
    <Button variant="primary" disabled>Convert</Button>
  </footer>
</div>
