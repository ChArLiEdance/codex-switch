use crate::models::{SwitchRestartTargets, TrayStatePayload};
#[cfg(target_os = "macos")]
use std::ffi::{CStr, CString};
#[cfg(target_os = "macos")]
use std::os::raw::c_char;
use std::sync::{Mutex, OnceLock};
#[cfg(not(target_os = "macos"))]
use tauri::menu::{MenuBuilder, SubmenuBuilder};
#[cfg(not(target_os = "macos"))]
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{App, AppHandle, Emitter, Manager};
#[cfg(not(target_os = "macos"))]
use tauri::{WebviewUrl, WebviewWindowBuilder, WindowEvent};

#[cfg(not(target_os = "macos"))]
const TRAY_ID: &str = "codex-switch-main";
#[cfg(not(target_os = "macos"))]
const TRAY_POPOVER_LABEL: &str = "tray-popover";
#[cfg(not(target_os = "macos"))]
const TRAY_POPOVER_WIDTH: f64 = 430.0;
#[cfg(not(target_os = "macos"))]
const TRAY_POPOVER_HEIGHT: f64 = 254.0;
#[cfg(target_os = "macos")]
const MACOS_TRAY_TEMPLATE_PNG: &[u8] = include_bytes!("../../icons/tray-template.png");
const ID_SHOW: &str = "tray_show_main";
const ID_SETTINGS: &str = "tray_settings";
const ID_ABOUT: &str = "tray_about";
const ID_QUIT: &str = "tray_quit";
#[cfg(not(target_os = "macos"))]
const ID_CURRENT: &str = "tray_current";
const ID_SWITCH_PREFIX: &str = "tray_switch_profile::";
static TRAY_RESTART_TARGETS: OnceLock<Mutex<SwitchRestartTargets>> = OnceLock::new();
static LAST_TRAY_STATE: OnceLock<Mutex<TrayStatePayload>> = OnceLock::new();
#[cfg(target_os = "macos")]
static TRAY_APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();

#[cfg(target_os = "macos")]
extern "C" {
    fn codex_switch_native_tray_install(
        icon_bytes: *const u8,
        icon_length: isize,
        callback: extern "C" fn(*const c_char, *const c_char),
    );
    fn codex_switch_native_tray_sync(json: *const c_char);
}

#[cfg(not(target_os = "macos"))]
#[derive(Debug, Clone, Copy)]
struct TrayLabels {
    show: &'static str,
    current: &'static str,
    switch_accounts: &'static str,
    settings: &'static str,
    about: &'static str,
    quit: &'static str,
    no_account: &'static str,
}

#[cfg(not(target_os = "macos"))]
fn labels(locale: &str) -> TrayLabels {
    if locale.starts_with("zh") {
        TrayLabels {
            show: "显示主界面",
            current: "当前账号",
            switch_accounts: "切换账号",
            settings: "设置",
            about: "关于",
            quit: "退出",
            no_account: "暂无当前账号",
        }
    } else {
        TrayLabels {
            show: "Show Main Window",
            current: "Current Account",
            switch_accounts: "Switch Account",
            settings: "Settings",
            about: "About",
            quit: "Quit",
            no_account: "No active account",
        }
    }
}

#[cfg(not(target_os = "macos"))]
fn tray_position_xy(position: tauri::Position) -> (f64, f64) {
    match position {
        tauri::Position::Physical(point) => (point.x as f64, point.y as f64),
        tauri::Position::Logical(point) => (point.x, point.y),
    }
}

pub fn install(app: &mut App) -> tauri::Result<()> {
    let _ = TRAY_RESTART_TARGETS.set(Mutex::new(SwitchRestartTargets::default()));
    let _ = LAST_TRAY_STATE.set(Mutex::new(TrayStatePayload::default()));
    let payload = TrayStatePayload {
        locale: "en".to_string(),
        ..TrayStatePayload::default()
    };

    #[cfg(target_os = "macos")]
    {
        let _ = TRAY_APP_HANDLE.set(app.handle().clone());
        unsafe {
            codex_switch_native_tray_install(
                MACOS_TRAY_TEMPLATE_PNG.as_ptr(),
                MACOS_TRAY_TEMPLATE_PNG.len() as isize,
                native_tray_callback,
            );
        }
        sync_state(app.handle(), payload)?;
        return Ok(());
    }

    #[cfg(not(target_os = "macos"))]
    {
        let menu = build_menu(app.handle(), &payload)?;
        let mut builder = TrayIconBuilder::with_id(TRAY_ID)
            .menu(&menu)
            .tooltip("Codex Switch")
            .show_menu_on_left_click(false)
            .on_menu_event(|app, event| handle_menu_event(app, event.id().as_ref()))
            .on_tray_icon_event(|tray, event| {
                if let TrayIconEvent::Click {
                    rect,
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } = event
                {
                    let (tray_x, tray_y) = tray_position_xy(rect.position);
                    let _ = toggle_windows_tray_popover(tray.app_handle(), tray_x, tray_y);
                }
            });

        if let Some(icon) = app.default_window_icon().cloned() {
            builder = builder.icon(icon);
        }

        let _tray = builder.build(app)?;
        Ok(())
    }
}

pub fn sync_state(app: &AppHandle, payload: TrayStatePayload) -> tauri::Result<()> {
    #[cfg(target_os = "macos")]
    let _ = app;

    if let Some(state) = LAST_TRAY_STATE.get() {
        if let Ok(mut guard) = state.lock() {
            *guard = payload.clone();
        }
    }

    if let Some(targets) = TRAY_RESTART_TARGETS.get() {
        if let Ok(mut guard) = targets.lock() {
            *guard = payload.restart_targets.clone();
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Ok(json) = serde_json::to_string(&payload) {
            if let Ok(json) = CString::new(json) {
                unsafe {
                    codex_switch_native_tray_sync(json.as_ptr());
                }
            }
        }
        return Ok(());
    }

    #[cfg(not(target_os = "macos"))]
    {
        let menu = build_menu(app, &payload)?;
        if let Some(tray) = app.tray_by_id(TRAY_ID) {
            tray.set_menu(Some(menu))?;
            let title = payload
                .current_title
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or("Codex Switch");
            tray.set_tooltip(Some(format!("Codex Switch - {title}")))?;
        }
        if let Some(window) = app.get_webview_window(TRAY_POPOVER_LABEL) {
            let _ = window.emit("codex-switch://tray-state-updated", payload);
        }
        Ok(())
    }
}

pub fn current_state() -> TrayStatePayload {
    LAST_TRAY_STATE
        .get()
        .and_then(|state| state.lock().ok().map(|guard| guard.clone()))
        .unwrap_or_default()
}

pub fn open_route(app: &AppHandle, route: &str) -> tauri::Result<()> {
    hide_windows_tray_popover(app)?;
    show_main_window(app)?;
    let _ = app.emit("codex-switch://tray-route", route);
    Ok(())
}

pub fn hide_windows_tray_popover(app: &AppHandle) -> tauri::Result<()> {
    #[cfg(not(target_os = "macos"))]
    if let Some(window) = app.get_webview_window(TRAY_POPOVER_LABEL) {
        window.hide()?;
    }
    let _ = app;
    Ok(())
}

pub fn show_main_window(app: &AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
    Ok(())
}

pub fn hide_main_window(app: &AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn build_menu(
    app: &AppHandle,
    payload: &TrayStatePayload,
) -> tauri::Result<tauri::menu::Menu<tauri::Wry>> {
    let label = labels(&payload.locale);

    let current = payload
        .current_title
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(label.no_account);

    let mut switch_menu = SubmenuBuilder::with_id(app, "tray_switch_menu", label.switch_accounts);
    for profile in &payload.profiles {
        let profile_label = if profile.nickname.trim().is_empty() {
            profile.display_title.as_str()
        } else {
            profile.nickname.as_str()
        };
        switch_menu = switch_menu.text(
            format!("{ID_SWITCH_PREFIX}{}", profile.folder_name),
            profile_label,
        );
    }
    let switch_menu = switch_menu.build()?;

    MenuBuilder::new(app)
        .text(ID_SHOW, label.show)
        .separator()
        .text(ID_CURRENT, format!("{}: {current}", label.current))
        .separator()
        .item(&switch_menu)
        .separator()
        .text(ID_SETTINGS, label.settings)
        .text(ID_ABOUT, label.about)
        .separator()
        .text(ID_QUIT, label.quit)
        .build()
}

#[cfg(not(target_os = "macos"))]
fn windows_tray_popover_position(app: &AppHandle) -> tauri::Result<tauri::PhysicalPosition<i32>> {
    const MARGIN: i32 = 18;
    let monitor = app.primary_monitor()?.or_else(|| {
        app.available_monitors()
            .ok()
            .and_then(|mut monitors| monitors.pop())
    });

    let Some(monitor) = monitor else {
        return Ok(tauri::PhysicalPosition::new(MARGIN, MARGIN));
    };

    let position = monitor.position();
    let size = monitor.size();
    let width = TRAY_POPOVER_WIDTH.round() as i32;
    let height = TRAY_POPOVER_HEIGHT.round() as i32;
    let min_x = position.x + MARGIN;
    let min_y = position.y + MARGIN;
    let x = position.x + size.width as i32 - width - MARGIN;
    let y = position.y + size.height as i32 - height - MARGIN;

    Ok(tauri::PhysicalPosition::new(x.max(min_x), y.max(min_y)))
}

#[cfg(not(target_os = "macos"))]
fn toggle_windows_tray_popover(app: &AppHandle, _tray_x: f64, _tray_y: f64) -> tauri::Result<()> {
    let position = windows_tray_popover_position(app)?;

    if let Some(window) = app.get_webview_window(TRAY_POPOVER_LABEL) {
        if window.is_visible().unwrap_or(false) {
            window.hide()?;
        } else {
            window.set_position(position)?;
            window.show()?;
            window.set_focus()?;
            let _ = window.emit("codex-switch://tray-state-updated", current_state());
        }
        return Ok(());
    }

    let window =
        WebviewWindowBuilder::new(app, TRAY_POPOVER_LABEL, WebviewUrl::App("tray.html".into()))
            .title("Codex Switch")
            .inner_size(TRAY_POPOVER_WIDTH, TRAY_POPOVER_HEIGHT)
            .min_inner_size(TRAY_POPOVER_WIDTH, TRAY_POPOVER_HEIGHT)
            .position(position.x as f64, position.y as f64)
            .resizable(false)
            .decorations(false)
            .transparent(true)
            .shadow(true)
            .always_on_top(true)
            .skip_taskbar(true)
            .focused(true)
            .build()?;

    let hide_window = window.clone();
    window.on_window_event(move |event| {
        if matches!(event, WindowEvent::Focused(false)) {
            let _ = hide_window.hide();
        }
    });

    Ok(())
}

#[cfg(target_os = "macos")]
extern "C" fn native_tray_callback(event: *const c_char, payload: *const c_char) {
    if event.is_null() {
        return;
    }
    let Some(app) = TRAY_APP_HANDLE.get() else {
        return;
    };

    let event = unsafe { CStr::from_ptr(event) }
        .to_string_lossy()
        .to_string();

    if event == "tray_switch_profile" {
        if payload.is_null() {
            return;
        }
        let profile = unsafe { CStr::from_ptr(payload) }.to_string_lossy();
        handle_menu_event(app, &format!("{ID_SWITCH_PREFIX}{profile}"));
        return;
    }

    handle_menu_event(app, &event);
}

fn handle_menu_event(app: &AppHandle, id: &str) {
    match id {
        ID_SHOW => {
            let _ = show_main_window(app);
        }
        ID_SETTINGS => {
            let _ = show_main_window(app);
            let _ = app.emit("codex-switch://tray-route", "settings");
        }
        ID_ABOUT => {
            let _ = show_main_window(app);
            let _ = app.emit("codex-switch://tray-route", "about");
        }
        ID_QUIT => {
            app.exit(0);
        }
        _ if id.starts_with(ID_SWITCH_PREFIX) => {
            let profile = id.trim_start_matches(ID_SWITCH_PREFIX).to_string();
            let app = app.clone();
            let restart_targets = TRAY_RESTART_TARGETS
                .get()
                .and_then(|targets| targets.lock().ok().map(|guard| guard.clone()))
                .unwrap_or_default();
            tauri::async_runtime::spawn_blocking(move || {
                #[cfg(target_os = "macos")]
                let response =
                    crate::macos::switch::switch_profile_with_targets(&profile, &restart_targets);
                #[cfg(not(target_os = "macos"))]
                let response =
                    crate::windows::switch::switch_profile_with_targets(&profile, &restart_targets);
                let _ = app.emit(
                    "codex-switch://tray-switch-finished",
                    response
                        .map(|value| value.message)
                        .unwrap_or_else(|error| error.message),
                );
            });
        }
        _ => {}
    }
}
