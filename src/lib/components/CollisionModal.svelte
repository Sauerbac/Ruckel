<!--
  Collision modal (ADR-0020 / ADR-0014). A blocking flat panel — 1.5px ink
  border over a HARD ink scrim (no blur, no shadow, no rounding) — listing the
  planned `_ppt` outputs that already exist. Each conflict is marked amber
  (`--warning`) and carries a per-file choice (Cancel / Override / Rename), with
  a bulk "Apply to all" that snaps every row at once. Dismissing cancels the
  whole conversion (ADR-0014).

  Driven by the frozen S1 `Collision[]` (F2, mock props). The per-row choice is
  local visual state; I3 lifts these decisions back into the plan.
-->
<script lang="ts">
  import type { Collision } from '../ipc'
  import { baseName } from '../format'
  import Button from './Button.svelte'

  export type CollisionChoice = 'cancel' | 'override' | 'rename'

  let {
    collisions,
    onConfirm,
    onDismiss,
  }: {
    collisions: Collision[]
    onConfirm?: (choices: Record<number, CollisionChoice>) => void
    onDismiss?: () => void
  } = $props()

  // Per-file choice keyed by the contract's job_index. Unset rows default to the
  // safe non-destructive option (Rename) — see `choiceFor`.
  const DEFAULT_CHOICE: CollisionChoice = 'rename'
  let choices = $state<Record<number, CollisionChoice>>({})

  const CHOICES: CollisionChoice[] = ['cancel', 'override', 'rename']
  const LABELS: Record<CollisionChoice, string> = {
    cancel: 'Cancel',
    override: 'Override',
    rename: 'Rename',
  }

  const choiceFor = (jobIndex: number) => choices[jobIndex] ?? DEFAULT_CHOICE

  function applyAll(choice: CollisionChoice) {
    for (const c of collisions) choices[c.job_index] = choice
  }

  function confirm() {
    onConfirm?.(
      Object.fromEntries(collisions.map((c) => [c.job_index, choiceFor(c.job_index)])),
    )
  }
</script>

<!-- Hard ink scrim — opaque, no blur. Clicking it dismisses (= cancel batch). -->
<div
  class="absolute inset-0 z-20 flex items-center justify-center bg-ink/70 p-6"
>
  <div
    class="flex max-h-full w-[460px] flex-col border-[1.5px] border-ink bg-surface"
    role="dialog"
    aria-modal="true"
    aria-label="Resolve output collisions"
  >
    <header class="flex items-center justify-between bg-ink px-3 py-2 text-paper">
      <span class="font-mono text-[11px] tracking-[0.1em] uppercase"
        >Files already exist</span
      >
      <span class="font-mono text-[11px] text-warning"
        >{collisions.length}
        {collisions.length === 1 ? 'conflict' : 'conflicts'}</span
      >
    </header>

    <!-- Bulk "Apply to all". -->
    <div class="flex items-center gap-2 border-b-[1.5px] border-line px-3 py-2">
      <span class="font-mono text-[10px] tracking-[0.1em] text-muted uppercase"
        >Apply to all</span
      >
      <span class="ml-auto flex gap-2">
        {#each CHOICES as choice (choice)}
          <button
            type="button"
            onclick={() => applyAll(choice)}
            class="h-[22px] cursor-default border border-line px-2 font-mono
                   text-[10px] tracking-[0.04em] text-ink-2 uppercase
                   outline-none hover:bg-fill focus-visible:outline-2
                   focus-visible:-outline-offset-2 focus-visible:outline-accent"
            >{LABELS[choice]}</button
          >
        {/each}
      </span>
    </div>

    <ul class="flex flex-col gap-2 overflow-auto p-3">
      {#each collisions as c (c.job_index)}
        <li class="flex flex-col gap-2 border-l-[3px] border-l-warning bg-paper p-2.5">
          <div class="flex flex-col gap-0.5">
            <span class="truncate font-mono text-[12px] text-ink"
              >{baseName(c.source_path)}</span
            >
            <span class="truncate font-mono text-[10px] text-warning"
              >{baseName(c.output_path)} exists</span
            >
          </div>
          <div class="flex gap-2">
            {#each CHOICES as choice (choice)}
              {@const selected = choiceFor(c.job_index) === choice}
              <button
                type="button"
                onclick={() => (choices[c.job_index] = choice)}
                class="h-[24px] flex-1 cursor-default border-[1.5px] px-2 font-mono
                       text-[10px] tracking-[0.04em] uppercase outline-none
                       focus-visible:outline-2 focus-visible:-outline-offset-2
                       focus-visible:outline-accent
                       {selected
                  ? 'border-ink bg-ink text-paper'
                  : 'border-line text-ink-2 hover:bg-fill'}">{LABELS[choice]}</button
              >
            {/each}
          </div>
        </li>
      {/each}
    </ul>

    <footer class="flex justify-end gap-2 border-t-[1.5px] border-line p-3">
      <Button variant="default" onclick={onDismiss}>Cancel All</Button>
      <Button variant="primary" onclick={confirm}>Continue</Button>
    </footer>
  </div>
</div>
