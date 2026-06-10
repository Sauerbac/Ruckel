use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

fn main() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    ffmpeg_libs_stale_guard(&manifest_dir);
    link_ffmpeg_extras(&manifest_dir);
    tauri_build::build()
}

/// Stale-build guard (ADR-0029): `ffmpeg-libs/` must carry the SHA-256 stamp of the
/// committed build manifest, so a stale or missing local FFmpeg build can never
/// silently link.
fn ffmpeg_libs_stale_guard(manifest_dir: &Path) {
    let manifest = manifest_dir.join("ffmpeg-build-manifest.json");
    let stamp = manifest_dir.join("ffmpeg-libs").join("manifest-hash.txt");
    println!("cargo:rerun-if-changed={}", manifest.display());
    println!("cargo:rerun-if-changed={}", stamp.display());

    let manifest_bytes = std::fs::read(&manifest)
        .expect("src-tauri/ffmpeg-build-manifest.json is missing — it is a committed file");
    let expected = hex(&Sha256::digest(&manifest_bytes));

    let actual = std::fs::read_to_string(&stamp)
        .map(|s| s.trim().to_ascii_lowercase())
        .unwrap_or_default();

    if actual != expected {
        panic!(
            "\n\
             ============================================================================\n\
             src-tauri/ffmpeg-libs/ is {} (stale-build guard, ADR-0029).\n\
             \n\
             committed manifest hash: {expected}\n\
             ffmpeg-libs stamp:       {}\n\
             \n\
             The static FFmpeg libs on disk were not built from the committed\n\
             ffmpeg-build-manifest.json. Rebuild them:\n\
             \n\
                 pwsh scripts/build-ffmpeg/build.ps1\n\
             ============================================================================\n",
            if actual.is_empty() { "missing" } else { "STALE" },
            if actual.is_empty() { "<none>".into() } else { actual },
        );
    }
}

/// rusty_ffmpeg links the seven libav libs; everything else FFmpeg's link needs is
/// emitted here (see ffmpeg-build-manifest.json "linking").
fn link_ffmpeg_extras(manifest_dir: &Path) {
    let libs_dir = manifest_dir.join("ffmpeg-libs").join("lib");
    println!("cargo:rustc-link-search=native={}", libs_dir.display());
    // External codec libs, statically linked.
    // `libx264` (not `x264`): FFmpeg's msvc configure hardcodes -lx264 -> libx264.lib,
    // so the build pipeline leaves x264's cl-built archive under that exact name.
    println!("cargo:rustc-link-lib=static=libx264");
    println!("cargo:rustc-link-lib=static=dav1d");
    // Windows system libs, resolved empirically by the tracer slice: avutil's
    // av_get_random_seed uses BCrypt* (a plain system check in FFmpeg's configure,
    // not governed by --disable-autodetect). Nothing else came up at link time.
    println!("cargo:rustc-link-lib=bcrypt");
}

fn hex(digest: &[u8]) -> String {
    digest.iter().map(|b| format!("{b:02x}")).collect()
}
