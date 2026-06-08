<!--
  Status / action bar (ADR-0019 / ADR-0020): a full-width solid-ink block
  carrying live batch state on the left and the primary action on the right.
  Four states, driven by a discriminated `status` prop —

    idle        "NO FILES"               → inert (disabled) CONVERT
    ready       "N FILES · ~X MB"        → green CONVERT
    converting  green ● pulse + elapsed   → CANCEL, with an accent fill tracking
                                            overall batch progress
    done        "N DONE · N ERRORS"       → CLEAR / CONVERT AGAIN

  Actions are optional callbacks so the gallery can render every state inertly.
-->
<script lang="ts">
  import { formatBytes, formatDuration } from '../format'
  import Button from './Button.svelte'

  export type StatusBarStatus =
    | { state: 'idle' }
    | { state: 'ready'; fileCount: number; totalBytes: number }
    | { state: 'converting'; percent: number; elapsedSecs: number }
    | { state: 'done'; succeeded: number; failed: number }

  let {
    status,
    onConvert,
    onCancel,
    onClear,
  }: {
    status: StatusBarStatus
    onConvert?: () => void
    onCancel?: () => void
    onClear?: () => void
  } = $props()

  const fill = $derived(
    status.state === 'converting'
      ? Math.min(100, Math.max(0, status.percent))
      : 0,
  )
</script>

<div class="relative flex h-11 shrink-0 items-center justify-between bg-ink px-3 text-paper">
  {#if status.state === 'converting'}
    <!-- Overall batch progress: a flat accent fill along the bar's base. -->
    <div
      class="absolute bottom-0 left-0 h-[3px] bg-accent"
      style="width: {fill}%"
    ></div>
  {/if}

  {#if status.state === 'idle'}
    <span class="font-mono text-[11px] tracking-[0.04em] text-muted uppercase"
      >No files</span
    >
    <Button variant="primary" disabled>Convert</Button>
  {:else if status.state === 'ready'}
    <span class="font-mono text-[11px] tracking-[0.04em] text-paper uppercase">
      {status.fileCount}
      {status.fileCount === 1 ? 'File' : 'Files'} · ~{formatBytes(status.totalBytes)}
    </span>
    <Button variant="primary" onclick={onConvert}>Convert</Button>
  {:else if status.state === 'converting'}
    <span
      class="flex items-center gap-[10px] font-mono text-[11px] tracking-[0.04em]
             text-paper uppercase"
    >
      <span
        class="h-[8px] w-[8px] bg-accent"
        style="animation: blink 1.6s steps(1) infinite"
      ></span>
      Converting · {Math.round(fill)}% · {formatDuration(status.elapsedSecs)}
    </span>
    <Button variant="danger" onclick={onCancel}>Cancel</Button>
  {:else}
    <span class="font-mono text-[11px] tracking-[0.04em] uppercase">
      <span class="text-accent">{status.succeeded} done</span>
      <span class="text-muted"> · </span>
      <span class={status.failed > 0 ? 'text-danger' : 'text-muted'}
        >{status.failed} {status.failed === 1 ? 'error' : 'errors'}</span
      >
    </span>
    <span class="flex items-center gap-2">
      <Button variant="default" onclick={onClear}>Clear</Button>
      <Button variant="primary" onclick={onConvert}>Convert Again</Button>
    </span>
  {/if}
</div>
