# Display nits — meta-line resolution guard + status-notice truncation

Status: complete

Two small display fixes surfaced by a read-through of the live components on
2026-06-09. The R1 live-eyeball items (title-bar chrome, phase transitions,
real-data states) were confirmed visually fine and are *not* part of this issue
(`v1-build/issues/14` closed). These are the only two static nits worth fixing;
deliberately small — obey ADR-0018 (flat, sharp, no shadows/transitions).

## Files in play

- `src/lib/format.ts` — `formatResolution`
- `src/lib/components/FileRow.svelte` — the meta line (`:188`)
- `src/lib/components/StatusBar.svelte` — the `notice` span (`:51`)

## What to build

### A. Resolution guard (`0×0`)

A valid video whose probe returns no coded dimensions falls back to `width`/
`height` = `0` (`probe.rs:85-86`), so the meta line renders a literal `0×0`.
**Omit the resolution segment entirely when `width` or `height` is `0`** — the
row then reads e.g. `MOV · 80 MB · 1:23` with no trailing `0×0`. Mirror the meta
line's existing pattern of conditionally revealing duration/resolution only once
probed (`FileRow.svelte:194-199`); this is the same idea one notch finer.

Implementation can live in `FileRow` (gate the resolution `<span>` on
`probe.width > 0 && probe.height > 0`) and/or `formatResolution`; keep
`formatResolution` pure and table-testable either way.

### B. Defensive notice truncation

The status-bar `notice` renders as a single un-truncated span
(`StatusBar.svelte:51`). Every notice we emit is short by construction, so this
is **insurance, not a bug fix**: add `max-w` + `truncate` (or equivalent) so a
hypothetically long notice can never push the 44px bar's layout. **Do not** clamp
the FileRow error message — that text is diagnostic and a tall error row is the
intended behaviour, not a layout fault.

## Acceptance criteria

- [ ] A probed video reporting `0` width or height shows no `0×0` (and no stray
      trailing `·`); a normal video still shows `1920×1080`
- [ ] `formatResolution` (or the guard) is unit-covered for the zero case
- [ ] The status-bar notice cannot overflow the bar; the FileRow error message is
      left full-length (unchanged)
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml` (if format tests live
      there — they're frontend here) and the frontend build/check both pass; the
      Gallery still renders every component state

## Notes

- Frontend-only; no contract change.
- If `formatResolution` keeps returning `WxH`, the empty-string/guard decision
  belongs to the caller (`FileRow`) so the formatter stays a dumb pure helper.
