# ADR-0026: Pre-flight skips invalid candidates instead of aborting

- Status: Accepted
- Date: 2026-06-09
- Clarifies: [ADR-0015](0015-error-handling.md) (pre-flight errors are
  skip-and-surface, never abort — the implementation now conforms)
- Revises: [ADR-0016](0016-tauri-command-and-event-contract.md)
  (`PreflightResult` gains a `skipped: u32` count)

## Context

ADR-0015 already decided that a pre-flight error (audio-only, unreadable file)
should be "collected before encoding starts and surfaced in the plan" — one bad
file must never derail a batch. The implementation diverged: `commands.rs::preflight`
returns `Err` on the **first** candidate that fails to probe or carries no video
stream, so a single stray file in a folder drop throws away every good file
alongside it. (This is the loose end flagged "out of scope" in the issue-08
scanner comment.)

`probe::probe` can fail in four ways, which collapse into two user-facing meanings:

- **No video stream** (`Ok { has_video: false }`) — a valid media file we read
  fine, it just has no video track (an audio-only / data-only file, e.g. a
  renamed `.mp3` or an extensionless audio file the scanner kept for the ffprobe
  fallback).
- **Unreadable** — ffprobe failed to *spawn*, exited non-zero (truncated /
  corrupt / non-media file that slipped past the extension filter), or emitted
  malformed JSON.

To the user both mean the same thing: "this isn't a usable video, skip it."
Neither is something they can act on differently, so the distinction is not worth
surfacing. The one case a "skipped" message would *misrepresent* — ffprobe
failing to spawn for **every** file (a broken install) — is already guarded by
the `fetch:ffmpeg` checksum + the smoke test, is rare, and is non-destructive even
when it misfires; special-casing it is gold-plating for v1.

## Decision

**Pre-flight skips every invalid candidate and continues.** All four probe
outcomes above are skipped per-candidate rather than aborting the drop; the good
files become jobs. This makes the implementation conform to ADR-0015.

**The skipped count is surfaced on the contract.** `PreflightResult` gains
`skipped: u32` (= scanned candidates − jobs produced). The frontend cannot derive
this itself — it drops *paths*, but the scanner expands a folder into N
candidates the frontend never sees — so the backend must report it. The ts-rs
bindings regenerate.

**The frontend reports drop outcomes through the status-bar notice** (the
transient ~2.5s override of ADR-0025), with one aggregate count — the
no-video/unreadable split is *not* itemised:

| Drop outcome | Surface | Copy |
|---|---|---|
| Some valid + some skipped (empty-start or append) | status notice | `N SKIPPED (NOT VIDEO)` |
| Append, nothing new, some skipped | status notice | `NO CONVERTIBLE VIDEO` |
| Append, nothing new, all were duplicates | status notice | `ALREADY ADDED` |
| Empty-start, nothing valid at all | drop-zone text (existing `dropError`) | `No convertible video found` |

The `ALREADY ADDED` case corrects a pre-existing lie: today a re-drop of
already-loaded files reports "No convertible video found", which is false — the
files *are* convertible, just already present. The `skipped` count lets the
frontend tell "skipped because not video" apart from "skipped because duplicate"
(the latter is the frontend's own dedup, ADR-0025).

## Consequences

- One stray file no longer discards a whole folder drop; pre-flight is genuinely
  batch-tolerant, matching the encode phase (ADR-0015) and the scanner's union/
  dedup behaviour (ADR-0009/0011).
- **IPC contract change** (ADR-0016): `PreflightResult` gains `skipped: u32`;
  the ts-rs bindings regenerate and the codegen drift gate must stay green.
- After this change `preflight` no longer returns `Err` for file-level reasons,
  so the frontend's `catch` on `preflight()` becomes a near-dead safety net and
  every emitted notice is short by construction (no raw-error-string overflow of
  the status bar).
- Supersedes the issue-08 scanner "out of scope" note about pre-flight aborting on
  the first non-video candidate.
