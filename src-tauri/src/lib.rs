use tauri::Manager;

#[path = "../shared/runtime/cli.rs"]
pub mod cli;
#[path = "../shared/commands/mod.rs"]
pub mod commands;
#[path = "../shared/runtime/errors.rs"]
pub mod errors;
#[cfg(target_os = "macos")]
#[path = "../mac/runtime/mod.rs"]
pub mod macos;
#[path = "../shared/runtime/models.rs"]
pub mod models;
#[path = "../shared/platform/mod.rs"]
pub mod platform;
#[path = "../shared/runtime/mod.rs"]
pub mod shared;
#[cfg(not(target_os = "macos"))]
#[path = "../win/runtime/windowing.rs"]
pub mod windowing;
#[cfg(not(target_os = "macos"))]
#[path = "../win/runtime/mod.rs"]
pub mod windows;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            platform::setup_runtime()?;
            platform::install_windowing(app)?;
            Ok(shared::tray::install(app)?)
        })
        .invoke_handler(tauri::generate_handler![
            commands::dashboard::get_profiles_snapshot,
            commands::dashboard::get_current_live_quota,
            commands::dashboard::get_usage_stats,
            commands::dashboard::get_usage_query_settings,
            commands::dashboard::save_usage_query_settings,
            commands::dashboard::list_codex_sessions,
            commands::dashboard::get_codex_session_messages,
            commands::dashboard::refresh_active_profile_quota_silent,
            commands::dashboard::refresh_all_oauth_profile_plans_silent,
            commands::actions::open_codex,
            commands::actions::login_current_profile,
            commands::actions::login_profile,
            commands::actions::refresh_profile,
            commands::actions::rename_profile,
            commands::actions::delete_profile,
            commands::actions::clear_profile_account,
            commands::actions::update_profile_base_url,
            commands::actions::open_profile_folder,
            commands::actions::add_profile,
            commands::actions::open_contact,
            commands::actions::open_releases,
            commands::actions::open_url,
            commands::actions::check_update,
            commands::actions::install_update,
            commands::actions::export_profiles_backup,
            commands::actions::import_profiles_backup,
            commands::actions::open_xiaohongshu,
            commands::actions::get_codex_cli_status,
            commands::actions::set_codex_cli_path,
            commands::actions::clear_codex_cli_path,
            commands::actions::redetect_codex_cli_path,
            commands::actions::cancel_codex_login,
            commands::actions::sync_tray_state,
            commands::actions::get_tray_state,
            commands::actions::open_tray_route,
            commands::actions::hide_tray_popover,
            commands::actions::show_main_window,
            commands::actions::hide_main_window,
            commands::actions::quit_app,
            commands::actions::restart_app,
            commands::switch::check_switch_health,
            commands::switch::switch_profile,
            commands::tools::list_codex_skills,
            commands::tools::save_codex_skill,
            commands::tools::delete_codex_skill,
            commands::tools::open_codex_skills_folder,
            commands::tools::list_codex_prompts,
            commands::tools::save_codex_prompt,
            commands::tools::enable_codex_prompt,
            commands::tools::delete_codex_prompt,
            commands::tools::import_codex_prompt_from_agents,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

pub fn run_cli(args: &[String]) -> i32 {
    cli::run(args, None)
}
