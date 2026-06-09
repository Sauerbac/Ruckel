<!--
  Right-panel options (ADR-0020 / ADR-0007). Three preset snaps over four
  segmented option groups, all bound to one `ConversionOptions` (the frozen S1
  contract type — no ad-hoc shape). Per the selection rule, the active
  preset/segment INVERTS to ink, never green. Deviating from a preset surfaces a
  muted CUSTOM indicator. The empty state is the whole panel `disabled`: fill /
  line / muted, no accent — absence of green is the "not ready" signal.

  F2 builds this from props; I2 wires it to the live plan. The progress fill and
  preset application mutate the bound options in place.
-->
<script lang="ts">
  import type { ConversionOptions } from '../ipc'
  import {
    AUDIO_OPTIONS,
    CRF_OPTIONS,
    FRAMERATE_OPTIONS,
    PRESET_LABELS,
    PRESET_ORDER,
    PRESETS,
    presetFor,
    RESOLUTION_OPTIONS,
    type PresetName,
  } from '../options'
  import SegmentedControl from './SegmentedControl.svelte'
  import Tooltip from './Tooltip.svelte'

  let {
    options = $bindable(),
    disabled = false,
  }: {
    options: ConversionOptions
    disabled?: boolean
  } = $props()

  const activePreset = $derived(presetFor(options))

  // Per-group tooltip content (ADR-0023). Structured — heading + one-line gloss
  // + an option→description table — so formatting carries the meaning and no dot
  // separators are needed. Row order mirrors the segments left→right.
  type Tip = {
    heading: string
    gloss: string
    rows: { opt: string; desc: string }[]
  }
  const TIPS: Record<string, Tip> = {
    preset: {
      heading: 'Preset',
      gloss: 'One-tap bundles — set all four knobs at once.',
      rows: [
        {
          opt: 'PRESENTATION',
          desc: 'balanced for slides (orig size, CRF 23)',
        },
        { opt: 'HIGH', desc: 'best quality, larger file (CRF 18)' },
        { opt: 'COMPACT', desc: 'smallest file (720p, CRF 28, 30 fps)' },
      ],
    },
    resolution: {
      heading: 'Resolution',
      gloss: 'Output frame height. Smaller = smaller file.',
      rows: [
        { opt: '480 / 720 / 1080', desc: 'downscale to that height' },
        { opt: 'ORIG', desc: 'keep the source size' },
      ],
    },
    quality: {
      heading: 'Quality',
      gloss: 'Compression level (CRF). Lower = sharper but larger.',
      rows: [
        { opt: '28', desc: 'small, softer' },
        { opt: '23', desc: 'balanced' },
        { opt: '18', desc: 'best quality, large file' },
      ],
    },
    framerate: {
      heading: 'Framerate',
      gloss: 'Frames per second. Lower = smaller file.',
      rows: [
        { opt: '24 / 30 / 60', desc: 'cap the rate' },
        { opt: 'ORIG', desc: 'keep the source' },
      ],
    },
    audio: {
      heading: 'Audio',
      gloss: 'Audio bitrate.',
      rows: [
        { opt: '96K', desc: 'smallest' },
        { opt: '128K', desc: 'standard' },
        { opt: '192K', desc: 'best' },
        { opt: 'NONE', desc: 'strip audio' },
      ],
    },
  }

  function applyPreset(name: PresetName) {
    const p = PRESETS[name]
    options.resolution = p.resolution
    options.crf = p.crf
    options.framerate = p.framerate
    options.audio = p.audio
  }
</script>

<!-- Group header: fixed-height row so the Custom badge / info icon appearing or
     disappearing never reflows the panel (ADR-0023). LABEL [ⓘ] on the left. -->
{#snippet header(label: string, tip: Tip)}
  <span class="flex items-center gap-1.5">
    <span class="font-mono text-[10px] tracking-[0.14em] text-muted uppercase"
      >{label}</span
    >
    <Tooltip heading={tip.heading} gloss={tip.gloss} rows={tip.rows} />
  </span>
{/snippet}

<div class="flex flex-col gap-5">
  <!-- Presets + Custom indicator. Hand-rolled (not SegmentedControl) because
       "Custom" means none selected — a state the segmented primitive can't hold. -->
  <div class="flex flex-col gap-2">
    <div class="flex h-5 items-center justify-between">
      {@render header('Preset', TIPS.preset)}
      {#if !disabled && activePreset === null}
        <span
          class="border border-line px-1.5 py-px font-mono text-[10px]
                 tracking-[0.1em] text-muted uppercase">Custom</span
        >
      {/if}
    </div>
    <div
      class="inline-flex {disabled
        ? 'border-[1.5px] border-line bg-fill'
        : 'border-[1.5px] border-ink bg-surface'}"
      role="group"
    >
      {#each PRESET_ORDER as name (name)}
        {@const selected = activePreset === name}
        <button
          type="button"
          {disabled}
          onclick={() => applyPreset(name)}
          class="h-[26px] flex-1 cursor-default border-line px-3 font-mono
                 text-[11px] tracking-[0.04em] outline-none not-first:border-l
                 focus-visible:outline-2 focus-visible:-outline-offset-2
                 focus-visible:outline-accent
                 {disabled
            ? selected
              ? 'bg-line text-ink-2'
              : 'text-muted'
            : selected
              ? 'bg-ink text-paper'
              : 'text-ink-2 hover:bg-fill'}"
        >
          {PRESET_LABELS[name]}
        </button>
      {/each}
    </div>
  </div>

  <div class="flex flex-col gap-2">
    <div class="flex h-5 items-center">
      {@render header('Resolution', TIPS.resolution)}
    </div>
    <SegmentedControl
      bind:value={options.resolution}
      {disabled}
      options={RESOLUTION_OPTIONS}
    />
  </div>

  <div class="flex flex-col gap-2">
    <div class="flex h-5 items-center">
      {@render header('Quality', TIPS.quality)}
    </div>
    <SegmentedControl
      bind:value={options.crf}
      {disabled}
      options={CRF_OPTIONS}
    />
  </div>

  <div class="flex flex-col gap-2">
    <div class="flex h-5 items-center">
      {@render header('Framerate', TIPS.framerate)}
    </div>
    <SegmentedControl
      bind:value={options.framerate}
      {disabled}
      options={FRAMERATE_OPTIONS}
    />
  </div>

  <div class="flex flex-col gap-2">
    <div class="flex h-5 items-center">
      {@render header('Audio', TIPS.audio)}
    </div>
    <SegmentedControl
      bind:value={options.audio}
      {disabled}
      options={AUDIO_OPTIONS}
    />
  </div>
</div>
