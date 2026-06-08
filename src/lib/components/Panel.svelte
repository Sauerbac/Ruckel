<!--
  Panel / surface primitive (ADR-0018 v2). A flat surface with a heavy 1.5px ink
  border and sharp corners — the basic container everything sits inside. No
  radius, no shadow.
-->
<script lang="ts">
  import type { Snippet } from 'svelte'
  import type { HTMLAttributes } from 'svelte/elements'

  let {
    children,
    class: extra = '',
    ...rest
  }: HTMLAttributes<HTMLDivElement> & { children: Snippet } = $props()
</script>

<div class="border-[1.5px] border-ink bg-surface {extra}" {...rest}>
  {@render children()}
</div>
