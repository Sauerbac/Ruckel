# ADR-0028: Curated decode matrix as the input-support contract

- Status: Accepted
- Date: 2026-06-09
- Amends: [ADR-0005](0005-prebuilt-ffmpeg-binaries.md), [ADR-0011](0011-file-type-filtering.md)

## Context

ADR-0005/0011 committed Ruckel to an "any video file" goal, delivered by the BtbN sidecar
compiling in hundreds of decoders, every demuxer, network protocols, devices, and the full
filter zoo. That long tail *is* the ~135 MB per binary. A `--disable-everything` build
([ADR-0029](0029-self-built-minimal-static-ffmpeg.md)) forces an explicit, written answer to
"which files can Ruckel open?" — and any file outside the list regresses versus v1.

## Decision

Input support becomes an **explicit, curated decode matrix**, aimed at "everything a normal
person drops on a PowerPoint converter." The matrix below is the contract; it is expressed
mechanically as the configure flags in the build manifest, and enforced by the fixture corpus
(one fixture per real-world cell, with a manifest↔corpus drift test).

### Output side (fixed, trivial)

| Component | Enabled |
|---|---|
| Encoders | `libx264`, `aac` (native) |
| Muxer | `mp4` |
| Protocol | `file` only |
| Filters | `buffer`, `buffersink`, `scale`, `format`, `transpose`, `hflip`, `vflip`, `fps`, `null` (see ADR-0027) |

No network, no devices, no other muxers/encoders/protocols.

### Input side (the matrix)

| Category | Enabled |
|---|---|
| **Demuxers** | `mov` (mp4/m4v/m4a/3gp), `matroska` (mkv/webm), `avi`, `asf` (wmv/wma), `flv`, `mpegts`, `mpegps` (vob), `mpegvideo`, `mxf` |
| **Video decoders** | `h264`, `hevc`, `mpeg4` (incl. Xvid/DivX), `mpeg2video`, `mpeg1video`, `msmpeg4v2`, `msmpeg4v3`, `wmv1`, `wmv2`, `vc1` (covers WMV3/WMV9), `vp8`, `vp9`, `av1` via **`libdav1d`**, `mjpeg`, `h263`, `prores`, `dnxhd`, `theora` |
| **Audio decoders** | `aac`, `mp3`, `mp2`, `ac3`, `eac3`, `wmav2`, `wmapro`, `vorbis`, `opus`, `flac`, `alac`, and the common PCM family (`pcm_s16le`, `pcm_s16be`, `pcm_s24le`, `pcm_s32le`, `pcm_u8`, `pcm_f32le`, `pcm_alaw`, `pcm_mulaw`) |
| **Parsers** | the set matching the decoders above (`h264`, `hevc`, `mpeg4video`, `mpegvideo`, `vp8`, `vp9`, `av1`, `mjpeg`, `aac`, `ac3`, `mpegaudio`, `opus`, `vorbis`, `flac`) |
| **Bitstream filters** | none expected (audio and video are always re-encoded, never copied). The matrix tests are the arbiter — a BSF is added only when a fixture proves it necessary. |

### AV1 is in

AV1 decode ships via external **`dav1d`** (BSD-2, small, fast, well-maintained). AV1 is a
*growing* format (screen recorders, newer phones, YouTube rips); shipping without it would be a
silent regression on the one format trending up, unlike the shrinking legacy tail.

### What is deliberately dropped

RealMedia, ancient QuickTime codecs (Sorenson, Cinepak), GoPro CineForm, raw camera formats,
3GPP's weirder corners, and the rest of the long tail. A file outside the matrix fails at
pre-flight probe and is **skipped with a notice** — the ADR-0026 skip-and-count machinery
already handles exactly this. That is the complete user-facing story for unsupported input.

## Consequences

- ADR-0011's consequence "the allowlist must stay broad to match the any-video-file goal" is
  superseded: the goal itself is now "the curated matrix," not "any video file." The
  extension-filtering mechanics of ADR-0011 are unchanged.
- Some file that converted in v1 may be skipped in v2. This is deliberate, accepted scope.
- Growing the matrix later is cheap and mechanical: add configure flags + a fixture, rebuild
  the libs, bump the manifest.
- Estimated cost: roughly 10–25 MB added to the exe, versus ~270 MB of sidecars today.
- Decoded inputs arrive in many pixel formats (10-bit HEVC, 4:2:2 ProRes, …); the mandatory
  `format=yuv420p` stage of the filter graph normalizes all of them, same as the sidecar did.
