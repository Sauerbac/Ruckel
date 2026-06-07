# ADR-0018: Frontend design system

- Status: Accepted
- Date: 2026-06-07

## Context

Ruckel should read as a trustworthy desktop utility, not a flashy web app. A single coherent
visual language keeps the small UI disciplined.

## Decision

**Aesthetic:** Retro-modern utility ("utilitarian neo-brutalism") — strong visible borders,
flat surfaces, monospace data text, one teal accent. No gradients, shadows, or decoration.
**Dark mode only** in v1.

**Palette (70-20-10):**

| Role | Hex |
|---|---|
| Base (70%) | `#0D1117` |
| Surface (20%) | `#161B22` |
| Accent (10%) | `#14B8A6` |
| Text primary | `#E6EDF3` |
| Text muted | `#7D8590` |
| Border | `#30363D` |
| Danger | `#F85149` |

**Typography:** Inter for all UI text; **JetBrains Mono** for all data (filenames, paths,
sizes, durations, progress numbers, error messages).

**Shape:** 2px border radius everywhere.

## Consequences

- Monospace data text makes filenames and numbers scannable and reinforces the utility feel.
- The accent is used sparingly (buttons, progress, active/selected, focus rings) so it carries
  weight.
- Light theme is explicitly out of scope for v1.
