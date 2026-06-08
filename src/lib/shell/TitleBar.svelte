<!--
  Custom title bar (ADR-0019 v2). Tauri runs with `decorations: false`, so the
  frame is drawn here as a solid-ink chrome block: a green brand square + RUCKEL
  wordmark (mono, uppercase, tracked), a drag region, and REAL close + minimize
  controls only — no maximize (meaningless at a fixed 860x560 size), and no
  decorative window-dots or connection meta (that theater belongs to the
  reference deploy-console screens, not a one-button converter).

  Window calls are guarded so the bar still renders in the browser-only gallery
  harness (no Tauri runtime there).
-->
<script lang="ts">
  const inTauri = '__TAURI_INTERNALS__' in window

  async function minimize() {
    if (!inTauri) return
    const { getCurrentWindow } = await import('@tauri-apps/api/window')
    await getCurrentWindow().minimize()
  }

  async function close() {
    if (!inTauri) return
    const { getCurrentWindow } = await import('@tauri-apps/api/window')
    await getCurrentWindow().close()
  }
</script>

<header
  data-tauri-drag-region
  class="flex h-9 shrink-0 items-center gap-[14px] bg-ink pr-0 pl-3 text-paper
         select-none"
>
  <!-- Brand: one accent square + wordmark. The square is the only green here. -->
  <span class="pointer-events-none flex items-center gap-[10px]">
    <span class="h-[11px] w-[11px] bg-accent"></span>
    <span class="font-mono text-xs font-medium tracking-[0.14em] uppercase">
      Ruckel
    </span>
  </span>

  <div class="ml-auto flex h-full">
    <button
      type="button"
      aria-label="Minimize"
      onclick={minimize}
      class="flex h-full w-11 cursor-default items-center justify-center
             text-paper outline-none hover:bg-paper hover:text-ink
             focus-visible:outline-2 focus-visible:-outline-offset-2
             focus-visible:outline-accent"
    >
      <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
        <line
          x1="0"
          y1="5"
          x2="10"
          y2="5"
          stroke="currentColor"
          stroke-width="1"
        />
      </svg>
    </button>
    <button
      type="button"
      aria-label="Close"
      onclick={close}
      class="flex h-full w-11 cursor-default items-center justify-center
             text-paper outline-none hover:bg-danger hover:text-paper
             focus-visible:outline-2 focus-visible:-outline-offset-2
             focus-visible:outline-accent"
    >
      <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
        <line
          x1="0"
          y1="0"
          x2="10"
          y2="10"
          stroke="currentColor"
          stroke-width="1"
        />
        <line
          x1="10"
          y1="0"
          x2="0"
          y2="10"
          stroke="currentColor"
          stroke-width="1"
        />
      </svg>
    </button>
  </div>
</header>
