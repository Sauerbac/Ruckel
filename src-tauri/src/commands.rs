//! `#[tauri::command]` handlers. Kept thin — each handler delegates
//! immediately into `preflight` or `encoder` (ADR-0016, ADR-0017).
