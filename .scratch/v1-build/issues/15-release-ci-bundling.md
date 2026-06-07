# R2 — CI + bundling + GPL + git-storage decision

Status: ready-for-human

## What to build

Release readiness. This slice is **HITL** — it carries the two genuine human decisions that were
deferred during planning, plus the release plumbing.

- **CI:** verify sidecar checksums against the manifest, build the app, run the full test suite
  (Rust units + the real-FFmpeg smoke tests + frontend checks).
- **Bundling/packaging:** produce the Windows installer/bundle with the sidecars included.
- **GPLv3 source-offer compliance:** record and ship the exact upstream FFmpeg source + build
  config for the bundled binaries (ADR-0003 / ADR-0005).
- **Git-storage decision (deferred from S0):** decide LFS vs commit for the binaries and apply it,
  right before the first push.

## Acceptance criteria

- [ ] CI verifies checksums, builds, and runs the full test suite green
- [ ] A Windows bundle/installer is produced with the sidecars included and launches correctly
- [ ] GPLv3 source-offer satisfied: exact upstream source + build config recorded and shipped
- [ ] **Human decision made and applied:** binaries git-storage (LFS vs commit), before first push

## Blocked by

- 14 (feature-complete app)
