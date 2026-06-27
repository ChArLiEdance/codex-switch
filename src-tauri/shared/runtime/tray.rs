use crate::models::{QuotaWindow, SwitchRestartTargets, TrayStatePayload};
use std::sync::{Mutex, OnceLock};
use tauri::image::Image;
use tauri::menu::{MenuBuilder, SubmenuBuilder};
use tauri::tray::TrayIconBuilder;
use tauri::{App, AppHandle, Emitter, Manager};

const TRAY_ID: &str = "codex-switch-main";
const MACOS_TRAY_TEMPLATE_RGBA: &[u8] = include_bytes!("../../icons/tray-template.rgba");
const ID_SHOW: &str = "tray_show_main";
const ID_SETTINGS: &str = "tray_settings";
const ID_ABOUT: &str = "tray_about";
const ID_QUIT: &str = "tray_quit";
const ID_CURRENT: &str = "tray_current";
const ID_FIVE_HOUR: &str = "tray_quota_five_hour";
const ID_WEEKLY: &str = "tray_quota_weekly";
const ID_REFRESH: &str = "tray_quota_refresh";
const ID_SWITCH_PREFIX: &str = "tray_switch_profile::";
static TRAY_RESTART_TARGETS: OnceLock<Mutex<SwitchRestartTargets>> = OnceLock::new();

#[derive(Debug, Clone, Copy)]
struct TrayLabels {
    show: &'static str,
    current: &'static str,
    switch_accounts: &'static str,
    settings: &'static str,
    about: &'static str,
    quit: &'static str,
    no_account: &'static str,
    five_hour: &'static str,
    weekly: &'static str,
    refresh: &'static str,
    used: &'static str,
    left: &'static str,
    resets: &'static str,
}

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
            five_hour: "5h",
            weekly: "7d",
            refresh: "下次刷新",
            used: "已用",
            left: "剩余",
            resets: "重置",
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
            five_hour: "5h",
            weekly: "7d",
            refresh: "Next refresh",
            used: "Used",
            left: "Left",
            resets: "Resets",
        }
    }
}

pub fn install(app: &mut App) -> tauri::Result<()> {
    let _ = TRAY_RESTART_TARGETS.set(Mutex::new(SwitchRestartTargets::default()));
    let payload = TrayStatePayload {
        locale: "en".to_string(),
        ..TrayStatePayload::default()
    };
    let menu = build_menu(app.handle(), &payload)?;
    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .tooltip("Codex Switch")
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| handle_menu_event(app, event.id().as_ref()));

    #[cfg(target_os = "macos")]
    {
        let icon = Image::new(MACOS_TRAY_TEMPLATE_RGBA, 32, 32);
        builder = builder.icon(icon).icon_as_template(true);
    }

    #[cfg(not(target_os = "macos"))]
    {
        if let Some(icon) = app.default_window_icon().cloned() {
            builder = builder.icon(icon);
        }
    }

    let _tray = builder.build(app)?;
    Ok(())
}

pub fn sync_state(app: &AppHandle, payload: TrayStatePayload) -> tauri::Result<()> {
    if let Some(targets) = TRAY_RESTART_TARGETS.get() {
        if let Ok(mut guard) = targets.lock() {
            *guard = payload.restart_targets.clone();
        }
    }
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
    let quota = payload.current_quota.as_ref();
    let five_hour = quota
        .map(|summary| quota_line(label.five_hour, &summary.five_hour, &label))
        .unwrap_or_else(|| format!("{}: --", label.five_hour));
    let weekly = quota
        .map(|summary| quota_line(label.weekly, &summary.weekly, &label))
        .unwrap_or_else(|| format!("{}: --", label.weekly));
    let refresh = quota
        .and_then(|summary| {
            summary
                .five_hour
                .refresh_at
                .clone()
                .or_else(|| summary.weekly.refresh_at.clone())
        })
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "--".to_string());

    let mut switch_menu = SubmenuBuilder::with_id(app, "tray_switch_menu", label.switch_accounts);
    for profile in &payload.profiles {
        let profile_label = if profile.nickname.trim().is_empty() {
            profile.display_title.as_str()
        } else {
            profile.nickname.as_str()
        };
        let five = profile
            .quota
            .five_hour
            .remaining_percent
            .map(|value| format!("{value}%"))
            .unwrap_or_else(|| "--".to_string());
        let weekly = profile
            .quota
            .weekly
            .remaining_percent
            .map(|value| format!("{value}%"))
            .unwrap_or_else(|| "--".to_string());
        switch_menu = switch_menu.text(
            format!("{ID_SWITCH_PREFIX}{}", profile.folder_name),
            format!("{profile_label}  5h {five} / 7d {weekly}"),
        );
    }
    let switch_menu = switch_menu.build()?;

    MenuBuilder::new(app)
        .text(ID_SHOW, label.show)
        .separator()
        .text(ID_CURRENT, format!("{}: {current}", label.current))
        .text(ID_FIVE_HOUR, five_hour)
        .text(ID_WEEKLY, weekly)
        .text(ID_REFRESH, format!("{}: {refresh}", label.refresh))
        .separator()
        .item(&switch_menu)
        .separator()
        .text(ID_SETTINGS, label.settings)
        .text(ID_ABOUT, label.about)
        .separator()
        .text(ID_QUIT, label.quit)
        .build()
}

fn quota_line(label: &str, window: &QuotaWindow, labels: &TrayLabels) -> String {
    let Some(percent) = window.remaining_percent.map(|value| value.min(100)) else {
        return format!("{label}  ▱▱▱▱▱▱▱▱▱▱  --");
    };
    let used = 100u8.saturating_sub(percent);
    let bar = quota_bar(percent);
    let reset = window
        .refresh_at
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("--");
    format!(
        "{label}  {bar}  {} {used}%  {} {percent}%  {} {reset}",
        labels.used, labels.left, labels.resets
    )
}

fn quota_bar(percent: u8) -> String {
    let filled = ((usize::from(percent) + 5) / 10).clamp(0, 10);
    let empty = 10usize.saturating_sub(filled);
    let block = if percent > 60 {
        "🟩"
    } else if percent >= 20 {
        "🟧"
    } else {
        "🟥"
    };
    format!("{}{}", block.repeat(filled), "▱".repeat(empty))
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
