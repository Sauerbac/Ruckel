<!--
  Button primitive (ADR-0018 v2). Utilitarian neo-brutalism: mono uppercase,
  1.5px ink border, sharp corners, flat. `default` hover-inverts to ink/paper;
  `primary` is the single green action (ink text); `danger` fills red on hover.
  No shadows, no gradients, no transitions (the aesthetic is instant/flat).
  Focus uses outline, not a ring, to keep `box-shadow` at zero.
-->
<script lang="ts">
  import type { Snippet } from 'svelte'
  import type { HTMLButtonAttributes } from 'svelte/elements'

  type Variant = 'default' | 'primary' | 'danger'
  type Size = 'sm' | 'lg'

  let {
    variant = 'default',
    size = 'sm',
    children,
    class: extra = '',
    ...rest
  }: HTMLButtonAttributes & {
    variant?: Variant
    size?: Size
    children: Snippet
  } = $props()

  const variants: Record<Variant, string> = {
    default: 'bg-surface text-ink hover:bg-ink hover:text-paper',
    primary:
      'bg-accent text-ink font-semibold border-ink hover:bg-accent-d hover:text-ink',
    danger: 'bg-surface text-danger hover:bg-danger hover:text-paper',
  }

  // `sm` is the chrome default; `lg` is the pinned rail-footer action (ADR-0024).
  const sizes: Record<Size, string> = {
    sm: 'h-[26px] px-3 text-[11px]',
    lg: 'h-11 px-4 text-[12px]',
  }
</script>

<button
  class="inline-flex cursor-default items-center gap-[7px] border-[1.5px]
         border-ink font-mono tracking-[0.06em] uppercase
         outline-none focus-visible:outline-2 focus-visible:outline-offset-0
         focus-visible:outline-accent
         disabled:cursor-default disabled:border-line disabled:bg-fill
         disabled:text-muted disabled:hover:bg-fill disabled:hover:text-muted
         {sizes[size]} {variants[variant]} {extra}"
  {...rest}
>
  {@render children()}
</button>
