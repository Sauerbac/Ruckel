<!--
  File row (ADR-0020 phased reveal). A display-only status row — not a browser
  entry, so no row-selection invert. Driven purely by props (F2): the phase
  discriminator plus the frozen-contract data each phase reveals —

    dropped     filename + size + extension
    ready       + duration + resolution (from FileProbe, post pre-flight)
    converting  inline bordered progress bar (accent fill, hard ink right-tick)
    done        green check badge + output path
    error       red badge, row auto-expands with the message in mono

  Live IPC wiring lands in the Phase 3 slices; here the parent supplies values.
-->
<script lang="ts">
  import type { FileProbe } from '../ipc'
  import {
    baseName,
    formatBytes,
    formatDuration,
    formatResolution,
  } from '../format'

  export type FileRowPhase =
    | 'dropped'
    | 'ready'
    | 'converting'
    | 'done'
    | 'error'

  let {
    fileName,
    sizeBytes,
    probe = null,
    phase,
    percent = 0,
    outputPath = null,
    errorMessage = null,
    onRemove,
  }: {
    /** Source filename (basename) — shown in mono. */
    fileName: string
    /** Dropped-file size in bytes (a frontend display value, not a probe field). */
    sizeBytes: number
    /** Per-file probe metadata; present from `ready` onward (ADR-0020). */
    probe?: FileProbe | null
    phase: FileRowPhase
    /** Live percent (0–100) for the `converting` phase. */
    percent?: number
    /** The written `<stem>_ppt.mp4` path for the `done` phase. */
    outputPath?: string | null
    /** Failure reason for the `error` phase (ADR-0015). */
    errorMessage?: string | null
    /** Remove this row from the batch (ADR-0023). Absent ⇒ no ✕ rendered. */
    onRemove?: () => void
  } = $props()

  const ext = $derived(fileName.includes('.') ? fileName.split('.').pop()! : '')
  const fill = $derived(Math.min(100, Math.max(0, percent)))
</script>

<div
  class="flex flex-col gap-2 border-[1.5px] p-3
         {phase === 'error'
    ? 'border-ink border-l-[3px] border-l-danger'
    : 'border-ink'}"
>
  <!-- Name + outcome badge + remove. (Format chip lives on the meta line.) -->
  <div class="flex items-center justify-between gap-3">
    <span class="min-w-0 truncate font-mono text-[12px] text-ink"
      >{fileName}</span
    >
    <span class="flex shrink-0 items-center gap-2">
      {#if phase === 'done'}
        <span
          class="inline-flex items-center gap-[5px] border border-ink bg-accent
                 px-1.5 py-px font-mono text-[10px] tracking-[0.08em] text-ink
                 uppercase"
          ><span class="h-[6px] w-[6px] bg-ink"></span>Done</span
        >
      {:else if phase === 'error'}
        <span
          class="inline-flex items-center gap-[5px] border border-ink bg-danger
                 px-1.5 py-px font-mono text-[10px] tracking-[0.08em] text-paper
                 uppercase"
          ><span class="h-[6px] w-[6px] bg-paper"></span>Error</span
        >
      {/if}
      <!-- Remove (ADR-0023): persistent, subordinate; inverts to danger on
           hover. Phase-gated — hidden mid-encode to keep rows 1:1 with the
           plan's file_index. -->
      {#if onRemove && phase !== 'converting'}
        <button
          type="button"
          aria-label={`Remove ${fileName}`}
          onclick={onRemove}
          class="flex h-[18px] w-[18px] cursor-default items-center
                 justify-center border border-line font-mono text-[10px]
                 leading-none text-muted outline-none hover:border-danger
                 hover:bg-danger hover:text-paper focus-visible:outline-2
                 focus-visible:-outline-offset-2 focus-visible:outline-accent"
          >✕</button
        >
      {/if}
    </span>
  </div>

  <!-- Meta line: format chip + size always; duration + resolution once probed. -->
  <div class="flex items-center gap-2 font-mono text-[11px] text-muted">
    {#if ext}
      <span
        class="border border-line px-1.5 py-px text-[10px] tracking-[0.08em]
               uppercase">{ext}</span
      >
    {/if}
    <span>{formatBytes(sizeBytes)}</span>
    {#if probe}
      <span aria-hidden="true">·</span>
      <span>{formatDuration(probe.duration_secs)}</span>
      <span aria-hidden="true">·</span>
      <span>{formatResolution(probe.width, probe.height)}</span>
    {/if}
  </div>

  {#if phase === 'converting'}
    <div class="flex items-center gap-2.5">
      <div class="relative h-3 flex-1 border-[1.5px] border-ink bg-surface">
        <div
          class="absolute inset-0 border-r-[1.5px] border-ink bg-accent"
          style="right: {100 - fill}%"
        ></div>
      </div>
      <span class="w-9 text-right font-mono text-[11px] text-ink-2"
        >{Math.round(fill)}%</span
      >
    </div>
  {:else if phase === 'done'}
    <span class="truncate font-mono text-[11px] text-muted"
      >{outputPath ? baseName(outputPath) : `${fileName} → _ppt.mp4`}</span
    >
  {:else if phase === 'error'}
    <span class="font-mono text-[11px] leading-snug text-danger"
      >{errorMessage}</span
    >
  {/if}
</div>
