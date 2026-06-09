<!--
  Info tooltip primitive (ADR-0023, new primitive). A subordinate square info
  trigger that, on hover OR keyboard focus, reveals a flat popover decoding an
  option group's choices. Per ADR-0018 it is INSTANT — no fade — and flat (ink
  border, paper bg, mono). The popover is `position: fixed`, measured off the
  trigger's rect, so the right panel's `overflow-auto` cannot clip it; it opens
  LEFT (toward the panel interior, away from the window's right edge).

  Content is STRUCTURED, not a run-on sentence (ADR-0023 amendment): a heading,
  a one-line gloss of what the knob does, then an option→description table. The
  two-column grid (`auto 1fr`) lets the formatting carry the meaning, so no dot
  separators are needed.
-->
<script lang="ts">
  export type TooltipRow = { opt: string; desc: string }

  let {
    heading,
    gloss,
    rows,
  }: {
    heading: string
    gloss: string
    rows: TooltipRow[]
  } = $props()

  let triggerEl = $state<HTMLButtonElement | null>(null)
  let open = $state(false)
  let top = $state(0)
  let right = $state(0)

  function show() {
    const el = triggerEl
    if (!el) return
    const r = el.getBoundingClientRect()
    top = r.top
    // Anchor the popover's right edge 8px left of the trigger → opens left.
    right = window.innerWidth - r.left + 8
    open = true
  }

  function hide() {
    open = false
  }
</script>

<button
  bind:this={triggerEl}
  type="button"
  aria-label={`About ${heading}`}
  onmouseenter={show}
  onmouseleave={hide}
  onfocus={show}
  onblur={hide}
  class="flex h-[14px] w-[14px] shrink-0 cursor-default items-center justify-center
         border border-line font-mono text-[9px] leading-none text-muted
         outline-none hover:border-ink hover:text-ink focus-visible:outline-2
         focus-visible:outline-offset-1 focus-visible:outline-accent"
>
  i
</button>

{#if open}
  <div
    role="tooltip"
    style="position: fixed; top: {top}px; right: {right}px;"
    class="z-50 w-[244px] border-[1.5px] border-ink bg-paper px-2.5 py-2 font-mono"
  >
    <p class="text-[10px] tracking-[0.12em] text-ink uppercase">{heading}</p>
    <p class="mt-1 text-[10px] leading-[1.45] text-muted">{gloss}</p>
    <div
      class="mt-2 grid grid-cols-[auto_1fr] gap-x-3 gap-y-1 text-[10px] leading-[1.4]"
    >
      {#each rows as row (row.opt)}
        <span class="whitespace-nowrap text-ink">{row.opt}</span>
        <span class="text-muted">{row.desc}</span>
      {/each}
    </div>
  </div>
{/if}
