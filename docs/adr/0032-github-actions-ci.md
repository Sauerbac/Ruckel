# ADR-0032: CI/CD on GitHub Actions — manifest-cached FFmpeg, release-on-tag

- Status: Accepted
- Date: 2026-06-11

## Context

[ADR-0029](0029-self-built-minimal-static-ffmpeg.md) deliberately deferred CI: the FFmpeg
libs were built on the development machine, but the contract was kept CI-shaped — one
committed manifest pins everything, one script (`scripts/build-ffmpeg/build.ps1`) consumes
it, and the stale-build guard plus the `runtime_lib_matches_manifest_configuration` test
verify the result. With v2 complete and the bare `ruckel.exe` as the release artifact
([ADR-0030](0030-single-file-artifact.md)), releases should not depend on one machine.

GitHub's `windows-latest` runners already satisfy almost every `build.ps1` prerequisite:
VS 2022 with the C++ x64 toolset, MSYS2 at `C:\msys64`, system `curl`/`tar`, Node, Rust.
Only the MSYS2 packages (`make`, `pkgconf`, `diffutils`) and the pip tools
(`meson`, `ninja`, `libclang`) need installing per run.

## Decision

One workflow, `.github/workflows/build.yml`, on pushes to `main`/`v2`, PRs to `main`, and
`v*` tags:

- **Build job (windows-latest):** install the missing prerequisites, then run the existing
  local pipeline unchanged — `build.ps1` → `cargo test` → `npm run tauri build` — and
  upload `ruckel.exe` as the build artifact. `npm run check` gates the frontend.
- **FFmpeg libs are cached** (`actions/cache`) keyed on the SHA of
  `ffmpeg-build-manifest.json` + the build scripts. The ~20–30 min lib build runs only
  when those change; ordinary pushes pay only the Rust/frontend build.
- **The cargo cache key also embeds the manifest hash**: rustc bundles the static libav
  objects into the `rusty_ffmpeg` rlib, so a manifest change must invalidate the cargo
  cache too, or a cached rlib would keep linking the old libs (the rebuild trap in
  `scripts/build-ffmpeg/README.md`). The runtime-configuration test remains the backstop.
- **Release on `v*` tags:** a second job creates a GitHub release carrying exactly
  `ruckel.exe` (the ADR-0030 artifact) plus `LICENSE` and `THIRD-PARTY-LICENSES.md`
  (GPL compliance, [ADR-0003](0003-gplv3-license.md); the tagged source's build manifest
  is the FFmpeg/x264 source pin).

## Consequences

- A release is `git tag vX.Y.Z && git push --tags` — no machine-specific state involved;
  any contributor's tag produces the same pinned build.
- The full suite (unit, EncoderConfig tables, fixture corpus, matrix drift gates,
  per-fixture transcodes) runs on every push; the committed fixtures make this possible
  without network access.
- CI builds trust the cache: a poisoned/corrupt `ffmpeg-libs` cache entry would be caught
  by the stale-build guard (hash mismatch) or the runtime-configuration test, not slip
  into a release.
- The local workflow is unchanged; CI runs the same script with the same manifest.
