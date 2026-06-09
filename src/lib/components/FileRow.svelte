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
    | 'cancelled'

  let {
    fileName,
    sizeBytes,
    probe = null,
    phase,
    percent = 0,
    outputPath = null,
    errorMessage = null,
    converting = false,
    onRemove,
    onCancelJob,
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
    /** Whether the batch as a whole is converting — drives the ✕'s meaning. */
    converting?: boolean
    /** Remove this row from the batch (ADR-0023). */
    onRemove?: () => void
    /** Cancel just this job mid-batch (ADR-0025). */
    onCancelJob?: () => void
  } = $props()

  const ext = $derived(fileName.includes('.') ? fileName.split('.').pop()! : '')
  const fill = $derived(Math.min(100, Math.max(0, percent)))

  // The ✕ is always rendered and context-aware (ADR-0025): plain Remove when
  // idle; Cancel-this-job for a queued/active row mid-batch; styled-inert (not
  // native `disabled`, so the title tooltip still fires) for an already-settled
  // row mid-batch.
  const xCancellable = $derived(
    converting && (phase === 'ready' || phase === 'converting'),
  )
  const xActive = $derived(!converting || xCancellable)
  const xTitle = $derived(
    !converting
      ? 'Remove'
      : xCancellable
        ? 'Cancel this job'
        : "Can't remove while converting",
  )

  function onX() {
    if (!converting) onRemove?.()
    else if (xCancellable) onCancelJob?.()
    // else: styled-inert, no action.
  }

  // Styled hint tooltip for the ✕ (ADR-0025): the flat popover look of the info
  // Tooltip (ink border, paper bg, mono, INSTANT — no fade) instead of the OS
  // `title`. It is `position: fixed` and portaled to <body> so the file list's
  // overflow + mask-image (which establishes a stacking context) can't clip it;
  // it opens LEFT off the ✕, like the info tooltip.
  let xEl = $state<HTMLButtonElement | null>(null)
  let hintOpen = $state(false)
  let hintTop = $state(0)
  let hintRight = $state(0)

  function showHint() {
    // Only mid-batch, where the ✕'s meaning is non-obvious (Cancel vs inert). A
    // bare ✕ outside a batch already reads as Remove — no tooltip needed.
    if (!converting) return
    const el = xEl
    if (!el) return
    const r = el.getBoundingClientRect()
    hintTop = r.top
    hintRight = window.innerWidth - r.left + 8
    hintOpen = true
  }

  function hideHint() {
    hintOpen = false
  }

  /** Re-parent a node to <body> so a masked/overflow ancestor can't clip it. */
  function portal(node: HTMLElement) {
    document.body.appendChild(node)
    return {
      destroy() {
        node.remove()
      },
    }
  }
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
      <!-- Outcome badges share the ✕'s 18px box height + leading-none so they
           sit on one baseline (ADR-0025). -->
      {#if phase === 'done'}
        <span
          class="inline-flex h-[18px] items-center gap-[5px] border border-ink
                 bg-accent px-1.5 font-mono text-[10px] leading-none
                 tracking-[0.08em] text-ink uppercase"
          ><span class="h-[6px] w-[6px] bg-ink"></span>Done</span
        >
      {:else if phase === 'error'}
        <span
          class="inline-flex h-[18px] items-center gap-[5px] border border-ink
                 bg-danger px-1.5 font-mono text-[10px] leading-none
                 tracking-[0.08em] text-paper uppercase"
          ><span class="h-[6px] w-[6px] bg-paper"></span>Error</span
        >
      {:else if phase === 'cancelled'}
        <!-- Neutral, set-aside look — no secondary line follows (ADR-0025). -->
        <span
          class="inline-flex h-[18px] items-center gap-[5px] border border-ink
                 bg-surface px-1.5 font-mono text-[10px] leading-none
                 tracking-[0.08em] text-muted uppercase"
          ><span class="h-[6px] w-[6px] bg-muted"></span>Cancelled</span
        >
      {/if}
      <!-- Always-rendered context-aware ✕ (ADR-0025): Remove when idle, Cancel
           this job mid-batch, styled-inert for a settled row mid-batch. Styled
           inert (not native `disabled`) so the hint tooltip still fires. -->
      <button
        bind:this={xEl}
        type="button"
        aria-label={`${xTitle} ${fileName}`}
        aria-disabled={!xActive}
        onclick={onX}
        onmouseenter={showHint}
        onmouseleave={hideHint}
        onfocus={showHint}
        onblur={hideHint}
        class="flex h-[18px] w-[18px] cursor-default items-center justify-center
               border font-mono text-[10px] leading-none outline-none
               focus-visible:outline-2 focus-visible:-outline-offset-2
               focus-visible:outline-accent {xActive
          ? 'border-line text-muted hover:border-danger hover:bg-danger hover:text-paper'
          : 'border-line text-line'}">✕</button
      >
    </span>
  </div>

  <!-- Meta line: plain dot-separated data — ext + size always; duration +
       resolution once probed. De-chipped per ADR-0024 (no box on plain data). -->
  <div class="flex items-center gap-2 font-mono text-[11px] text-muted">
    {#if ext}
      <span class="uppercase">{ext}</span>
      <span aria-hidden="true">·</span>
    {/if}
    <span>{formatBytes(sizeBytes)}</span>
    {#if probe}
      <span aria-hidden="true">·</span>
      <span>{formatDuration(probe.duration_secs)}</span>
      <!-- A probe can report 0×0 when a video carries no coded dimensions
           (probe.rs falls back to 0); omit the resolution segment entirely then,
           separator and all, so the row never shows a literal 0×0 (ADR-0026 nit). -->
      {#if probe.width > 0 && probe.height > 0}
        <span aria-hidden="true">·</span>
        <span>{formatResolution(probe.width, probe.height)}</span>
      {/if}
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

<!-- ✕ hint, styled like the info Tooltip and portaled to <body> (ADR-0025). -->
{#if hintOpen}
  <div
    use:portal
    role="tooltip"
    style="position: fixed; top: {hintTop}px; right: {hintRight}px;"
    class="pointer-events-none z-50 max-w-[220px] border-[1.5px] border-ink
           bg-paper px-2 py-1 font-mono text-[10px] leading-[1.4] text-ink"
  >
    {xTitle}
  </div>
{/if}
