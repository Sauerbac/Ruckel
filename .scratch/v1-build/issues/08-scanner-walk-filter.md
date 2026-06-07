# B3 — Scanner: flat walk + extension filter

Status: ready-for-agent

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
