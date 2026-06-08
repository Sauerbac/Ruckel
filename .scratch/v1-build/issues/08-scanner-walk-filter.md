# B3 — Scanner: flat walk + extension filter

Status: complete

## What to build

Pre-flight scanning in `preflight/scanner.rs`: given dropped paths (files and/or folders), produce
the set of candidate video files. A dropped folder is walked **flat** — immediate files only, no
subdirectory recursion (ADR-0009). Candidates are filtered by the supported video extension set
(ADR-0011). The result feeds plan construction, yielding **N** Conversion Jobs.

## Acceptance criteria

- [ ] A dropped folder contributes only its immediate files (no recursion into subfolders)
- [ ] Non-video extensions are filtered out; supported video extensions are kept
- [ ] A mix of dropped files and folders resolves to the correct candidate set
- [ ] Table tests over a temp directory tree cover flat-walk and filtering; no FFmpeg needed

## Blocked by

- 04 (plan/job types and the preflight command established by the tracer)

## Comments

**Implemented** in `preflight/scanner.rs` as `scan(dropped) -> Vec<PathBuf>`:

- Each dropped **folder** is walked flat — immediate files only, `read_dir` with
  no recursion (ADR-0009); its files are sorted for deterministic output.
- Each dropped **file** is taken as-is. Both paths run through `is_video_candidate`
  (ADR-0011): a known video extension (broad lowercased allowlist) is kept, an
  extensionless file is kept and left for the ffprobe fallback, and any other
  extension is skipped. Probing each candidate for a real video stream stays in
  `commands.rs::preflight` (the ADR-0010 separation), so the scanner spawns no
  FFmpeg and is fully table-testable.
- Results are deduplicated (a file dropped directly *and* via its folder appears
  once) with top-level drop order preserved; nonexistent paths are ignored.
- `commands.rs::preflight` now builds the plan from `scanner::scan(paths)` instead
  of the tracer's single-file loop — so a folder or multi-file drop yields N jobs.

**Verification**

- `cargo test` — 32 green (29 unit incl. 6 new `scanner` tests over a self-cleaning
  temp tree, 1 codegen, 2 smoke). Tests cover flat-walk (no recursion into a
  subfolder), extension filtering, the file/folder mix union, dedup, and the pure
  `is_video_candidate` classifier.
- `cargo clippy` clean for the new modules; codegen drift gate clean
  (`git diff --exit-code src/lib/ipc`).

**Note (out of scope):** `preflight` still aborts on the first candidate lacking a
video stream (tracer behaviour). For a folder drop this means one stray
extensionless non-video file would fail the whole pre-flight; batch-tolerant
pre-flight error handling belongs to issue 12 (errors).
