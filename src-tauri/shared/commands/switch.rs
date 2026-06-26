use crate::errors::CommandError;
use crate::models::{ProfilePayload, SwitchResponse};

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
