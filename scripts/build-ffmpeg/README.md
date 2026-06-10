# build-ffmpeg — Ruckel's self-built minimal static FFmpeg (ADR-0029)

Builds **dav1d → x264 → FFmpeg** as MSVC-native static libraries containing exactly the
ADR-0028 decode matrix, and installs them into the **gitignored** `src-tauri/ffmpeg-libs/`
(static libs + headers + pkg-config files + manifest-hash stamp).

Everything is pinned by the committed **`src-tauri/ffmpeg-build-manifest.json`**:
source tarballs (version + SHA-256), the full configure flag sets, and the NASM build
tool. The manifest doubles as the GPL-compliance record (ADR-0003).

## Usage

```powershell
pwsh scripts/build-ffmpeg/build.ps1          # incremental-ish (cached downloads, fresh extract)
pwsh scripts/build-ffmpeg/build.ps1 -Clean   # from scratch, wipes .work/ and ffmpeg-libs/
```

Takes ~10–20 minutes. Runs rarely — only when the manifest changes.

## Prerequisites (one-time)

| Tool | Why | Install |
|---|---|---|
| Visual Studio 2022, C++ x64 toolset | `cl.exe` — one CRT family end to end, no MinGW objects | VS installer |
| MSYS2 (default `C:\msys64`) | bash + make for the x264/FFmpeg configure builds | `winget install MSYS2.MSYS2` |
| MSYS2 pkgs: `make`, `pkgconf`, `diffutils` | build drivers inside the MSYS2 shell | `C:\msys64\usr\bin\bash.exe -lc 'pacman -S --needed --noconfirm make pkgconf diffutils'` |
| meson + ninja | dav1d's build system | `pip install meson ninja` |
| libclang (`LIBCLANG_PATH`) | bindgen, run by rusty_ffmpeg at **cargo build** time | `pip install libclang`, then set `LIBCLANG_PATH` (user env var) to the wheel's `clang/native` dir |

NASM is **not** a prerequisite — it is pinned in the manifest and fetched/verified by the
script. MSYS2 location can be overridden with `-Msys2Root` or `RUCKEL_MSYS2_ROOT`.

## How the pieces fit (ADR-0029)

- `build.ps1` verifies prerequisites, downloads + SHA-256-verifies all pins, imports the
  vcvars64 environment, then drives the three builds inside an MSYS2 shell with that
  environment inherited (`build-dav1d.sh`, `build-x264.sh`, `build-ffmpeg.sh`).
- Inside the MSYS2 shell the MSVC bin dir is prepended to PATH so `link`/`cl`/`lib`
  resolve to the toolchain, not MSYS2's coreutils `/usr/bin/link`; the scripts also clear
  any inherited `ORIGINAL_PATH` so MSYS2's `inherit` mode sees the vcvars environment.
- Static libs are normalized to `<name>.lib` (rusty_ffmpeg emits
  `cargo:rustc-link-lib=static=<name>`, which the MSVC linker resolves as `<name>.lib`).
- The script stamps `ffmpeg-libs/manifest-hash.txt` with the manifest's SHA-256;
  `src-tauri/build.rs` **fails the cargo build loudly** if the stamp is missing or stale,
  so a stale local build can never silently link. Any manifest edit ⇒ re-run this script.
- rsmpeg/rusty_ffmpeg find the libs via `FFMPEG_LIBS_DIR`/`FFMPEG_INCLUDE_DIR`, set
  repo-relatively in the committed `.cargo/config.toml`. Extra libs (x264, dav1d,
  Windows system libs) are emitted by `src-tauri/build.rs` — see the manifest's
  `linking` section.
