//! Ruckel backend. Module layout mirrors the pre-flight / encode split
//! (ADR-0017): `preflight` resolves a Conversion Plan, `encoder` runs it,
//! `probe` is the shared ffprobe wrapper, `commands` exposes thin IPC handlers.
//!
//! Modules are `pub` so the S1 codegen integration test can reach the frozen
//! IPC contract types (ADR-0016).

pub mod commands;
pub mod encoder;
pub mod preflight;
pub mod probe;

use std::path::PathBuf;
use std::process::Command;

/// The bundled sidecar target triple (ADR-0001: Windows-only). The fetched
/// binaries carry this suffix in `src-tauri/binaries/` (ADR-0022).
const SIDECAR_TRIPLE: &str = "x86_64-pc-windows-msvc";

/// Resolve a bundled sidecar (`ffmpeg` / `ffprobe`) to an on-disk path
/// (ADR-0004). In a bundled app Tauri places the sidecar next to the main
/// executable with the triple stripped (`ffmpeg.exe`); in dev and tests the
/// fetched binary lives in `src-tauri/binaries/<name>-<triple>.exe`.
pub fn sidecar_path(name: &str) -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let bundled = dir.join(format!("{name}.exe"));
            if bundled.exists() {
                return bundled;
            }
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("binaries")
        .join(format!("{name}-{SIDECAR_TRIPLE}.exe"))
}

/// A `Command` for a bundled sidecar, with the Windows console window
/// suppressed so spawning FFmpeg/ffprobe never flashes a black box.
pub fn sidecar_command(name: &str) -> Command {
    let mut cmd = Command::new(sidecar_path(name));
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(commands::EncoderState::default())
        .invoke_handler(tauri::generate_handler![
            commands::preflight,
            commands::check_collisions,
            commands::start_conversion,
            commands::cancel_conversion,
        ])
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
