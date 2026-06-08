# ADR-0021: Tailwind CSS for styling

- Status: Accepted
- Date: 2026-06-07

## Context

The frontend ([ADR-0002](0002-svelte-frontend.md)) needs a styling approach. The design
system ([ADR-0018](0018-frontend-design-system.md)) defines a fixed palette, two typefaces,
sharp (0-radius) corners, and a deliberately small set of tokens — a utility-class workflow
fits that disciplined, token-driven aesthetic well.

The scaffold ([repo bootstrap, 2026-06-07]) wires Tailwind in but does **not** yet apply the
design system: `src/app.css` is just `@import "tailwindcss";` and components use neutral
default colours. This ADR records the framework choice; ADR-0018's tokens are applied later.

## Decision

Use **Tailwind CSS v4** as the styling layer, integrated via the official
**`@tailwindcss/vite`** plugin (CSS-first config — no `tailwind.config.js`, no
`postcss.config`). The entry point is a single `@import "tailwindcss";` in `src/app.css`.

When the design system lands, ADR-0018's palette and type tokens are defined **in CSS** via
Tailwind v4's `@theme { ... }` block, so the ADR-0018 values become first-class utilities
(e.g. `bg-base`, `text-accent`, `font-mono`) rather than ad-hoc hex literals.

## Consequences

- Utility classes keep styling colocated with markup — appropriate for a small, fixed UI
  with no large component library.
- The ADR-0018 design tokens have a single home (`@theme` in `app.css`) and flow out as typed
  utilities, keeping the "two-colour discipline" enforceable.
- One more frontend build dependency, but it runs inside the existing Vite pipeline — no
  separate PostCSS step to maintain.
- v4's CSS-first model means there is no JS config file to drift; the theme is versioned
  alongside the stylesheet.

## Alternatives considered

- **Hand-rolled CSS + custom properties** (the implicit ADR-0018 default) — rejected: more
  boilerplate for layout/spacing/state variants that Tailwind gives for free; tokens would
  still need a home, which `@theme` provides anyway.
- **Tailwind v3 (`tailwind.config.js` + PostCSS)** — rejected: v4 is the current line, drops
  the JS config and PostCSS step, and integrates directly with Vite.
- **A component/UI kit (e.g. Skeleton, Flowbite)** — rejected: heavier than this fixed,
  custom-styled utility window needs; conflicts with the bespoke neo-brutalist look.
