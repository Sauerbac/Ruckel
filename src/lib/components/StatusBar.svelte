<!--
  Status bar (ADR-0019 / ADR-0024): a full-width solid-ink block carrying live
  batch state. Status-only — the primary action moved to the pinned rail footer
  (ADR-0024); this bar's reason to exist as a solid-ink block is the global
  progress fill it alone can give. Four states, driven by a discriminated
  `status` prop —

    idle        "NO FILES"               (mono, muted)
    ready       "N FILES · ~X MB"
    converting  green ● pulse + N% + elapsed, with an accent fill along the base
                tracking overall batch progress
    done        "N DONE · N ERRORS"
-->
<script lang="ts">
  import { formatBytes, formatDuration } from '../format'

  export type StatusBarStatus =
    | { state: 'idle' }
    | { state: 'ready'; fileCount: number; totalBytes: number }
    | { state: 'converting'; percent: number; elapsedSecs: number }
    | { state: 'done'; succeeded: number; failed: number }

  let { status }: { status: StatusBarStatus } = $props()

  const fill = $derived(
    status.state === 'converting'
      ? Math.min(100, Math.max(0, status.percent))
      : 0,
  )
</script>

<div class="relative flex h-11 shrink-0 items-center bg-ink px-3 text-paper">
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
  {:else if status.state === 'ready'}
    <span class="font-mono text-[11px] tracking-[0.04em] text-paper uppercase">
      {status.fileCount}
      {status.fileCount === 1 ? 'File' : 'Files'} · ~{formatBytes(
        status.totalBytes,
      )}
    </span>
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
  {:else}
    <span class="font-mono text-[11px] tracking-[0.04em] uppercase">
      <span class="text-accent">{status.succeeded} done</span>
      <span class="text-muted"> · </span>
      <span class={status.failed > 0 ? 'text-danger' : 'text-muted'}
        >{status.failed} {status.failed === 1 ? 'error' : 'errors'}</span
      >
    </span>
  {/if}
</div>
