//! Ruckel backend. Module layout mirrors the pre-flight / encode split
//! (ADR-0017): `preflight` resolves a Conversion Plan, `encoder` runs it,
//! `probe` is the shared in-process libavformat read (ADR-0027), `commands`
//! exposes thin IPC handlers.
//!
//! Modules are `pub` so the S1 codegen integration test can reach the frozen
//! IPC contract types (ADR-0016).

pub mod commands;
pub mod encoder;
pub mod preflight;
pub mod probe;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(commands::EncoderState::default())
        .invoke_handler(tauri::generate_handler![
            commands::preflight,
            commands::check_collisions,
            commands::start_conversion,
            commands::cancel_conversion,
            commands::cancel_job,
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
