# Sidecar retirement + docs close-out — the sidecar era ends atomically

Status: complete

## Parent

`.scratch/v2-single-binary/PRD.md` (steps 8 + 9, deliberately combined — the docs describe
the state the deletion creates). Decision records: ADR-0030 (artifact), ADR-0029
(supersedes the fetch machinery), ADR-0003 (license notices).

## What to build

### A. Retire the sidecar machinery

Nothing spawns sidecars any more (issues 03/04); now delete the corpse:

- `tauri.conf.json`: remove `externalBin`, the stray `bundle.android` key, and trim
  `bundle.targets` down from `"all"` (the bare exe is the artifact, ADR-0030)
- `src-tauri/binaries/` — the old `ffmpeg-manifest.json` and the gitignore entries for the
  fetched bytes
- `scripts/fetch-ffmpeg.mjs` and its `fetch:ffmpeg` npm script
- the `sidecar_command` helper in `lib.rs` and the old sidecar-spawning smoke test
- anything else a repo-wide grep for sidecar/ffprobe/ffmpeg-exe references turns up,
  except docs that are deliberately historical (superseded ADRs stay untouched)

### B. License notices

Ship dav1d's BSD-2 license text alongside the GPL notices (FFmpeg/x264 are covered by the
app's GPLv3 + the build manifest as the compliance record, per ADR-0029).

### C. CONTEXT.md close-out

- Stack line: "FFmpeg + ffprobe, bundled as sidecar processes" → linked in-process
  (ADR-0027)
- Glossary: **Sidecar** leaves; **Probe** redefined as the in-process libavformat read;
  **PowerPoint-safe flags** reworded from flag-vector phrasing to the encode contract
  (asserted on `EncoderConfig`, ADR-0006 values unchanged)

## Acceptance criteria

- [x] Fresh `cargo tauri build` on a checkout **without** `src-tauri/binaries/` produces a
      working `ruckel.exe`; the exe, copied alone to another directory, converts a video
      (build + standalone launch verified; the conversion run is the per-release human check)
- [x] Repo-wide grep finds no live sidecar/ffprobe spawn references (superseded ADRs and
      the PRD are the only mentions, as history)
- [x] dav1d BSD-2 notice ships with the app's license material (LICENSE +
      THIRD-PARTY-LICENSES.md created — the repo had no license files before)
- [x] CONTEXT.md glossary and stack section describe the in-process reality; no stale
      "sidecar" vocabulary outside superseded ADRs
- [x] Full suite green: unit, EncoderConfig tables, matrix integration tests

## Blocked by

- `03-probe-swap.md`
- `04-encoder-swap.md`
- `05-matrix-integration-tests.md`

## Notes

- The fixture-generation script (issue 01) stays committed but its sidecar dependency is
  now historical — mark it clearly as "ran once against v1.0-sidecar; corpus is the
  artifact" rather than deleting it.
- ADRs 0027–0031 were already written on 2026-06-09; this issue does not touch them.
