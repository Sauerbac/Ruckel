# ADR-0018: Frontend design system

- Status: Accepted
- Date: 2026-06-08
- Revises: the initial 2026-06-07 draft (which named this aesthetic but undershot
  it — dark GitHub-style surface, soft borders, 2px radius). This revision lands
  the aesthetic faithfully, using an external design handoff (utilitarian
  neo-brutalism) as the reference, adapted to Ruckel's single-task window.

## Context

Ruckel should read as a trustworthy desktop utility, not a flashy web app — "utility
done right," old Windows chrome with modern discipline. A single coherent visual
language keeps the small UI honest. The original ADR-0018 named this aesthetic
("utilitarian neo-brutalism") but the implementation drifted into a soft dark theme.
This revision commits to the literal execution: warm light surfaces, sharp corners,
heavy visible borders, flat chrome blocks, one strong accent.

## Decision

**Aesthetic:** Utilitarian neo-brutalism — visible intentional borders instead of
shadows, flat solid-color surfaces (no gradients, no blur, no glass), monospace for
all data and a geometric sans for chrome, structured grid layout with hard dividers.
**Light, warm-neutral** (not dark). **Zero `box-shadow` anywhere.**

**Palette (warm light):**

| Role | Hex |
|---|---|
| Paper — app background | `#EDEBE6` |
| Surface — panels, lists | `#FBFAF8` |
| Fill — recessed (toolbars, headers) | `#E4E1DA` |
| Ink — text + all borders + chrome blocks | `#16150F` |
| Ink-2 — secondary text | `#3A382F` |
| Muted — tertiary / meta text | `#76736A` |
| Line — hairline interior borders | `#C8C4B9` |

**Status colors (the one deliberate divergence from the handoff's single-hue purity
— Ruckel reports real per-file outcomes, so it needs honest signals):**

| Signal | Hex | Hue | Role |
|---|---|---|---|
| Success / active / live | `#1FB866` | 152° | the primary accent; done, converting pulse, drop-target, active |
| Warning | `#D9821A` | 35° | collisions and skipped/filtered files only |
| Danger | `#C8341D` | 12° | real encode failures only |
| Accent pressed | `#159152` | — | green hover/pressed, "done" text |

All three signals are warm-biased to sit on warm paper and matched in
saturation/value so they read as one family. Green carries weight by being reserved:
it never marks a *selected* control (see ADR-0020).

**Typography:** **Space Grotesk** for all chrome/UI (labels, buttons, body);
**IBM Plex Mono** for all data (filenames, paths, sizes, durations, progress numbers,
SHAs, status segments, error messages). The mono/sans split carries the aesthetic and
is non-negotiable. Both are OFL and **bundled locally as subset woff2** — no CDN
(offline desktop app, per the font-hosting rationale in app.css / src/fonts).

**Shape:** `border-radius: 0` everywhere — no exceptions. Shell and major dividers
`1.5px solid` ink; interior hairlines `1px` line. The only sanctioned dashed border is
the drop-target marquee (ADR-0020); the only "depth" cue allowed is an `inset` accent
left-edge marker (reads as a flat bar, not a shadow).

**Motion:** none except a single `blink` keyframe — `steps(1)` hard on/off, ~1.6s
(`0–60%` opaque, `61–100%` at `.3`), used only for the live "converting" pulse and a
text cursor. No smooth fades, no ease, no shadow lifts; the aesthetic is instant/flat.

**Component signatures (the load-bearing generic primitives — recorded here so the
system stands without the original handoff):**

- **Button** — mono ~11px UPPERCASE, `1.5px` ink border, `--surface` bg, 26px tall,
  sharp. **Hover inverts** (bg→ink, text→paper). The accent (primary) variant = green
  bg + ink text + 600 weight, hover→`--accent-d`. Disabled = `--fill` bg, `--muted`
  text, `--line` border (no accent). Focus uses `outline`, never a ring (rings are
  `box-shadow`, which is banned).
- **Segmented control** — segments share one `1.5px` ink frame divided by `1px`
  hairlines; the **selected** segment inverts to ink + paper text (never green — see
  ADR-0020).
- **Flat progress bar** — `1.5px` ink frame, `--surface` track, `--accent` fill ending
  in a `1.5px` ink right edge (a hard tick); the completed state fills `--ink` instead.
  No animation, no rounding, no gradient — the signature "bordered, not glossy" element.
- **Chip / status pill** — mono ~10px UPPERCASE, `1.5px` ink border, sharp, with a
  leading 7px **square** dot. Variant carried by fill: success = `--accent` bg,
  warning = `--warning` bg, error = `--ink`/`--danger` bg with paper text.
- **Pulse** — an 8px `--accent` square running the `blink` keyframe; drive it from real
  live state (e.g. converting), never decoratively.

## Consequences

- Monospace data text makes filenames and numbers scannable and reinforces the
  utility feel.
- A reserved green plus two warm status hues gives an unmistakable three-way outcome
  signal (done / caution / failed) without diluting the accent.
- Light/warm-only in v1. Dark mode and theming remain out of scope (ADR-0019's fixed
  single-task window argues against per-user theming).
- Swapping to Space Grotesk + IBM Plex Mono requires subsetting and hosting two new
  woff2 files and updating `src/fonts/LICENSES.md`; Inter and JetBrains Mono are
  removed.
