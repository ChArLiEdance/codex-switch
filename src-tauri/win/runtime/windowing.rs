#![cfg_attr(target_os = "macos", allow(dead_code))]

use tauri::{utils::config::Color, App, Emitter, Manager, WindowEvent};

const WINDOW_BG: Color = Color(244, 241, 236, 255);

pub fn install(app: &mut App) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window("main") else {
        return Ok(());
    };

    let _ = window.set_background_color(Some(WINDOW_BG));

    let close_window = window.clone();
    window.on_window_event(move |event| {
        if let WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let _ = crate::platform::sync_on_window_close();
            let _ = close_window.emit("codex-switch://close-requested", ());
        }
    });

    Ok(())
}
