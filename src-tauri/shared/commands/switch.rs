use crate::errors::CommandError;
use crate::models::{ProfilePayload, SwitchHealthResponse, SwitchResponse};

#[cfg(target_os = "macos")]
use crate::macos as platform_runtime;

#[cfg(not(target_os = "macos"))]
use crate::windows as platform_runtime;

#[tauri::command]
pub async fn switch_profile(payload: ProfilePayload) -> Result<SwitchResponse, CommandError> {
    let profile = payload.profile;
    let restart_targets = payload.restart_targets.unwrap_or_default();
    tauri::async_runtime::spawn_blocking(move || {
        platform_runtime::switch::switch_profile_with_targets(&profile, &restart_targets)
    })
    .await
    .map_err(|error| CommandError::new("SWITCH_FAILED", format!("Switch task failed: {error}")))?
    .map_err(Into::into)
}

#[tauri::command]
pub async fn check_switch_health(
    payload: ProfilePayload,
) -> Result<SwitchHealthResponse, CommandError> {
    let profile = payload.profile;
    tauri::async_runtime::spawn_blocking(move || {
        let codex_home = platform_runtime::paths::get_codex_home();
        let cli_status = crate::shared::codex_cli_path::get_codex_cli_status(
            platform_runtime::codex_cli_resolver(),
            &codex_home,
        );
        crate::shared::switch_health::check_switch_health_with_home(
            &profile,
            Some(&codex_home),
            cli_status,
            platform_runtime::process::is_codex_app_running(),
            platform_runtime::process::is_vscode_running(),
        )
    })
    .await
    .map_err(|error| {
        CommandError::new(
            "SWITCH_HEALTH_FAILED",
            format!("Switch health check task failed: {error}"),
        )
    })?
    .map_err(Into::into)
}
