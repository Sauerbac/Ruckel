# CONTEXT — Ruckel

Ruckel is a Windows-only desktop utility that converts video files into PowerPoint-friendly
MP4s. Drop a single file or a folder, and each video is transcoded to H.264/AAC MP4 with
PowerPoint-safe flags, written next to the original with a `_ppt` suffix.

This file is the **source of truth for domain vocabulary**. Decisions live in `docs/adr/`.

## Stack at a glance

- **App framework:** Tauri v2 (Windows-only) — see [ADR-0001](docs/adr/0001-tauri-v2-windows-desktop.md)
- **Frontend:** Svelte — see [ADR-0002](docs/adr/0002-svelte-frontend.md)
- **Video backend:** FFmpeg + ffprobe, bundled as sidecar processes — see [ADR-0004](docs/adr/0004-ffmpeg-sidecar-not-ffi.md)
- **License:** GPLv3 — see [ADR-0003](docs/adr/0003-gplv3-license.md)

## Glossary

Use these terms exactly. Don't drift to synonyms.

| Term | Definition |
|---|---|
| **`_ppt` output** | The converted MP4. Always named `<source-stem>_ppt.mp4`, written in the source file's own folder. The naming contract is fixed — see [ADR-0008](docs/adr/0008-output-location-and-naming.md). |
| **PowerPoint-safe flags** | The non-negotiable encode flags every output carries: `-c:v libx264 -profile:v high -pix_fmt yuv420p -movflags +faststart -c:a aac`. No hardcoded `-level`. See [ADR-0006](docs/adr/0006-powerpoint-safe-output-encoding.md). |
| **Sidecar** | A standalone executable (`ffmpeg.exe`, `ffprobe.exe`) bundled with the app and spawned as a child process via `std::process::Command`. Not linked into the app. See [ADR-0004](docs/adr/0004-ffmpeg-sidecar-not-ffi.md). |
| **Pre-flight** | The phase that runs all fast checks (scan, filter, probe, collision detection) *before* any encoding, producing a fully-resolved Conversion Plan. See [ADR-0010](docs/adr/0010-preflight-encode-separation.md). |
| **Probe** | An invocation of `ffprobe` to read a file's duration and confirm a video stream is present. |
| **Conversion Plan** | The fully-resolved batch the encoder executes blindly: an ordered list of Conversion Jobs. Output of pre-flight. |
| **Conversion Job** | One resolved unit of work: source path, output path, and the four user options applied. |
| **Encoder** | The component that takes a Conversion Plan and runs FFmpeg per job, emitting progress events. Does no decision-making. |
| **Flat walk** | Folder scanning that includes only the immediate files in the dropped folder — no subdirectory recursion. See [ADR-0009](docs/adr/0009-processing-model.md). |
| **Collision** | A planned `_ppt` output path that already exists on disk. Resolved in pre-flight only. See [ADR-0014](docs/adr/0014-collision-resolution.md). |
| **Override / Rename / Cancel** | The three per-file collision resolutions: overwrite the existing file / write to a user-supplied name / skip this file and continue the batch. |
| **Faststart** | The `+faststart` muxer flag that moves MP4 metadata (moov atom) to the front, so PowerPoint loads the video instantly instead of after a full scan. |
| **The four options** | The only user-facing encode knobs: Resolution, Quality (CRF), Framerate cap, Audio bitrate. See [ADR-0007](docs/adr/0007-user-options-and-presets.md). |
| **Preset** | A named bundle of all four option values: **Presentation**, **High Quality**, **Compact**. Selecting one snaps all four controls at once. |
| **Custom** | A display-only state shown when the four controls don't match any preset. Not itself selectable. |
| **Phased reveal** | The frontend pattern where a file row gains data in stages (on-drop → after pre-flight → during conversion → done/error) mirroring the backend phases. See [ADR-0020](docs/adr/0020-frontend-interaction-model.md). |

## ADR index

| # | Decision |
|---|---|
| [0001](docs/adr/0001-tauri-v2-windows-desktop.md) | Tauri v2 as the app framework (Windows-only) |
| [0002](docs/adr/0002-svelte-frontend.md) | Svelte for the frontend |
| [0003](docs/adr/0003-gplv3-license.md) | License Ruckel under GPLv3 |
| [0004](docs/adr/0004-ffmpeg-sidecar-not-ffi.md) | FFmpeg/ffprobe as sidecar processes, not FFI |
| [0005](docs/adr/0005-prebuilt-ffmpeg-binaries.md) | Ship prebuilt static FFmpeg binaries for v1 |
| [0006](docs/adr/0006-powerpoint-safe-output-encoding.md) | Fixed PowerPoint-safe output encoding flags |
| [0007](docs/adr/0007-user-options-and-presets.md) | Four user-facing options and three presets |
| [0008](docs/adr/0008-output-location-and-naming.md) | Output location and `_ppt` naming contract |
| [0009](docs/adr/0009-processing-model.md) | Sequential processing, flat folder walk |
| [0010](docs/adr/0010-preflight-encode-separation.md) | Pre-flight / encode separation |
| [0011](docs/adr/0011-file-type-filtering.md) | File-type filtering |
| [0012](docs/adr/0012-output-safety-temp-and-rename.md) | Output safety: temp file + atomic rename |
| [0013](docs/adr/0013-encoder-process-control.md) | Encoder process control: progress + cancellation |
| [0014](docs/adr/0014-collision-resolution.md) | Collision resolution in pre-flight |
| [0015](docs/adr/0015-error-handling.md) | Error handling: skip-and-continue + summary |
| [0016](docs/adr/0016-tauri-command-and-event-contract.md) | Tauri command surface and event contract |
| [0017](docs/adr/0017-rust-module-structure.md) | Rust module structure |
| [0018](docs/adr/0018-frontend-design-system.md) | Frontend design system |
| [0019](docs/adr/0019-frontend-window-and-layout.md) | Frontend window and layout |
| [0020](docs/adr/0020-frontend-interaction-model.md) | Frontend interaction model |
