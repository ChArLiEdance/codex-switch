#[tauri::command]
fn backend_health() -> &'static str {
    "codex_switch_backend_ready"
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![backend_health])
        .run(tauri::generate_context!())
        .expect("failed to run Codex Switch");
}

