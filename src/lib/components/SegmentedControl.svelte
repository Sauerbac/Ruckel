<!--
  Segmented-control primitive (ADR-0018 v2 / 0020). A row of mutually-exclusive
  segments sharing one 1.5px ink frame, divided by hairlines. The selected
  segment INVERTS to ink + paper text — never green (green is reserved for
  live/success status). Disabled = the empty-state treatment: fill/line/muted,
  no accent. Used for the four option groups (ADR-0007). Generic over the value.
-->
<script lang="ts" generics="T extends string | number">
  type Option = { label: string; value: T }

  let {
    options,
    value = $bindable(),
    disabled = false,
  }: {
    options: Option[]
    value: T
    disabled?: boolean
  } = $props()
</script>

<div
  class="flex w-full border-[1.5px]
         {disabled ? 'border-line bg-fill' : 'border-ink bg-surface'}"
  role="group"
>
  {#each options as option (option.value)}
    {@const selected = option.value === value}
    <button
      type="button"
      {disabled}
      onclick={() => (value = option.value)}
      class="h-[26px] flex-1 cursor-default border-line px-3 font-mono text-[12px]
             outline-none not-first:border-l focus-visible:outline-2
             focus-visible:-outline-offset-2 focus-visible:outline-accent
             {disabled
        ? selected
          ? 'bg-line text-ink-2'
          : 'text-muted'
        : selected
          ? 'bg-ink text-paper'
          : 'text-ink-2 hover:bg-fill'}"
    >
      {option.label}
    </button>
  {/each}
</div>
