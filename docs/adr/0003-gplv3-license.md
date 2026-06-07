# ADR-0003: License Ruckel under GPLv3

- Status: Accepted
- Date: 2026-06-07

## Context

Ruckel converts video using FFmpeg built with libx264. **libx264 is GPL**, and H.264
*encoding* requires `--enable-gpl`, so any FFmpeg binary capable of producing the `_ppt.mp4`
output is a GPL binary. This constrains how Ruckel may be distributed:

- **Sidecar architecture** (FFmpeg as a separate process — see
  [ADR-0004](0004-ffmpeg-sidecar-not-ffi.md)): Ruckel is "mere aggregation," not a derivative
  work, so Ruckel's own license could in principle be anything.
- **FFI architecture** (linking `libav*`/`libx264` into the process): GPL code is linked into
  Ruckel itself, so the whole app must be GPL.

The project owner's intent is that Ruckel be **open-source and freely available to everyone**.

## Decision

License **Ruckel itself under GPLv3**.

## Consequences

- All three FFmpeg integration architectures — sidecar, from-source, and FFI — remain
  permanently available; there is no contamination concern because the app is GPL anyway.
- Compliance obligation for the bundled FFmpeg binary is minimal: ship the GPL license text
  and a pointer to the exact upstream FFmpeg source used (a `THIRD-PARTY-LICENSES.md` plus an
  about-box entry). See [ADR-0005](0005-prebuilt-ffmpeg-binaries.md) for which source to pin.
- Anyone may use, modify, and redistribute Ruckel under GPLv3 terms.
