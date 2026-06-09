# Finishing touches

A small, grilled-and-scoped spree closing the last functional gap and a couple of
display nits before R2 (release). Came out of a grilling pass on 2026-06-09 over
the deferred R1 polish issue (`v1-build/issues/14`) plus a read-through of the
live components.

## What's in scope

1. **Pre-flight skips invalid candidates instead of aborting** — the one genuine
   functional gap. One stray non-video / unreadable file in a folder drop
   currently kills the whole drop. See **ADR-0026**. (Issue 01.)
2. **Display nits** — the `0×0` resolution render for a dimensionless video, and
   defensive truncation of the status-bar notice. (Issue 02.)

## What's explicitly out

- **Recursive folder walk** (ADR-0009 rejected it for v1).
- **Advanced mode** — raw CRF stepper / custom FFmpeg flags (ADR-0007/0020,
  deferred to a future advanced surface).
- The R1 live-eyeball items (title-bar chrome, phase-transition layout, real-data
  states) — **confirmed visually fine** on 2026-06-09; `v1-build/issues/14`
  closed accordingly.
- R2 release plumbing (CI, bundling, GPL, LFS-vs-commit) — separate, see
  `v1-build/issues/15`.

## Decision record

- **ADR-0026** — pre-flight skip-and-count + the drop-notice matrix.
- Conforms to **ADR-0015** (skip-and-surface), extends the **ADR-0025** status-bar
  notice, amends **ADR-0016** (`PreflightResult.skipped`).
