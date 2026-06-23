use crate::{
    account_hint::redacted_account_hint_from_path,
    app_state::{AppStateRepository, SwitchHistoryEntry, SwitchHistoryStatus, VscodeReloadMode},
    cli_app::{CliRuntime, SystemCliRuntime},
    desktop_app::{DesktopProcessController, MacDesktopProcessController},
    importer::EnvironmentSnapshot,
    profile::{ProfileAuthStatus, ProfileMetadata, TargetEnvironment},
    profile_store::{ProfileRepository, ProfileStoreError},
    secret_store::{SecretStore, SecretStoreError, SecretVault},
    switch_transaction::{
        RestoreArtifact, RestorePlan, SwitchTransaction, TransactionError, TransactionPhase,
        TransactionRunner,
    },
    vscode_app::{MacVscodeProcessController, VscodeProcessController},
};
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    thread,
    time::{Duration, Instant},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileSwitchRequest {
    pub profile_id: String,
    pub environments: Vec<TargetEnvironment>,
    pub auto_restart_apps: bool,
    pub vscode_reload_mode: VscodeReloadMode,
    pub confirm_process_close: bool,
    pub desktop_app_path: Option<String>,
    pub vscode_app_path: Option<String>,
    pub quit_timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileSwitchResult {
    pub profile: ProfileMetadata,
    pub transaction: SwitchTransaction,
    pub identity_verification: SwitchIdentityVerification,
    pub switched_environments: Vec<TargetEnvironment>,
    pub manual_actions: Vec<String>,
    pub warnings: Vec<String>,
    pub closed_processes: Vec<String>,
    pub restarted_apps: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreDefaultOnExitResult {
    pub attempted: bool,
    pub reason: String,
    pub switch_result: Option<ProfileSwitchResult>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestartAppRequest {
    pub app_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestartAppResult {
    pub target: String,
    pub restarted: bool,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SwitchIdentityStatus {
    Verified,
    Incomplete,
    Mismatch,
    NotChecked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentIdentityObservation {
    pub environment: TargetEnvironment,
    pub account_hint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchIdentityVerification {
    pub status: SwitchIdentityStatus,
    pub target_account_hint: String,
    pub observed: Vec<EnvironmentIdentityObservation>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileSwitchError {
    ProfileNotFound(String),
    NoEnvironmentSelected,
    EnvironmentUnavailable(TargetEnvironment),
    SecretMissing(TargetEnvironment),
    SecretStore(String),
    ProfileStore(String),
    Snapshot(String),
    NoRestorableArtifacts,
    Transaction(String),
    AppState(String),
    CliTaskRunning(Vec<String>),
    ProcessCloseConfirmationRequired(Vec<String>),
    Process(String),
}

impl From<SecretStoreError> for ProfileSwitchError {
    fn from(value: SecretStoreError) -> Self {
        ProfileSwitchError::SecretStore(format!("{value:?}"))
    }
}

impl From<ProfileStoreError> for ProfileSwitchError {
    fn from(value: ProfileStoreError) -> Self {
        ProfileSwitchError::ProfileStore(format!("{value:?}"))
    }
}

pub fn switch_saved_profile<S: SecretStore>(
    request: ProfileSwitchRequest,
    profile_repository: &ProfileRepository,
    app_state_repository: &AppStateRepository,
    vault: &SecretVault<S>,
    timestamp: String,
) -> Result<ProfileSwitchResult, ProfileSwitchError> {
    let runtime = SystemProfileSwitchRuntime::default();
    switch_saved_profile_with_runtime(
        request,
        profile_repository,
        app_state_repository,
        vault,
        timestamp,
        &runtime,
    )
}

pub fn switch_saved_profile_with_runtime<S: SecretStore, R: ProfileSwitchRuntime>(
    request: ProfileSwitchRequest,
    profile_repository: &ProfileRepository,
    app_state_repository: &AppStateRepository,
    vault: &SecretVault<S>,
    timestamp: String,
    runtime: &R,
) -> Result<ProfileSwitchResult, ProfileSwitchError> {
    if request.environments.is_empty() {
        return Err(ProfileSwitchError::NoEnvironmentSelected);
    }

    let profiles = profile_repository.list_profiles()?;
    let mut profile = profiles
        .iter()
        .cloned()
        .find(|profile| profile.id == request.profile_id)
        .ok_or_else(|| ProfileSwitchError::ProfileNotFound(request.profile_id.clone()))?;
    let from_profile = latest_used_profile_name(&profiles, &profile.id);

    let plan = restore_plan_from_profile(&profile, &request.environments, vault, &timestamp)?;
    let mut closed_processes = close_running_processes(&request, runtime)?;
    let mut restarted_apps = Vec::new();
    let backup_root = app_state_repository.root().join("backups");
    let runner = TransactionRunner::new(backup_root);
    let planned_transaction =
        SwitchTransaction::new(plan.transaction_id.clone(), plan.target_profile_id.clone());
    app_state_repository
        .save_current_transaction(&planned_transaction)
        .map_err(|error| ProfileSwitchError::AppState(format!("{error:?}")))?;
    let transaction = runner
        .run_with_post_restore(&plan, || {
            restart_after_restore(&request, runtime, &mut restarted_apps)
                .map_err(|error| TransactionError::PostRestore(format!("{error:?}")))
        })
        .map_err(|error| ProfileSwitchError::Transaction(format!("{error:?}")))?;
    app_state_repository
        .save_current_transaction(&transaction)
        .map_err(|error| ProfileSwitchError::AppState(format!("{error:?}")))?;
    let identity_verification = verify_switched_identity(&profile, &plan, &transaction);
    let status = if transaction.phase == TransactionPhase::Completed {
        if identity_verification.status == SwitchIdentityStatus::Verified {
            SwitchHistoryStatus::Success
        } else {
            SwitchHistoryStatus::Incomplete
        }
    } else if transaction
        .events
        .iter()
        .any(|event| event.phase == TransactionPhase::RolledBack)
    {
        SwitchHistoryStatus::RolledBack
    } else {
        SwitchHistoryStatus::Failed
    };
    if transaction.phase == TransactionPhase::Completed {
        profile = profile_repository
            .mark_profile_used(&profile.id, timestamp.clone())
            .map_err(ProfileSwitchError::from)?;
    }

    app_state_repository
        .append_history(SwitchHistoryEntry {
            id: format!("history-{timestamp}-{}", profile.id),
            switched_at: timestamp.clone(),
            from_profile,
            to_profile: profile.name.clone(),
            environments: request.environments.clone(),
            status,
            error_type: if transaction.phase == TransactionPhase::Completed {
                match identity_verification.status {
                    SwitchIdentityStatus::Verified => None,
                    SwitchIdentityStatus::Incomplete => Some("IdentityIncomplete".to_string()),
                    SwitchIdentityStatus::Mismatch => Some("IdentityMismatch".to_string()),
                    SwitchIdentityStatus::NotChecked => Some("IdentityNotChecked".to_string()),
                }
            } else {
                Some(format!("{:?}", transaction.phase))
            },
        })
        .map_err(|error| ProfileSwitchError::AppState(format!("{error:?}")))?;

    let manual_actions = manual_actions_for(&request);
    let mut warnings = Vec::new();
    if transaction.phase != TransactionPhase::Completed {
        warnings.push(
            "Switch restore failed; rollback was attempted and post-switch actions were skipped"
                .to_string(),
        );
    } else if identity_verification.status == SwitchIdentityStatus::Incomplete {
        warnings.push(format!(
            "Configuration switched, but account identity verification is incomplete: {}",
            identity_verification.message
        ));
    } else if identity_verification.status == SwitchIdentityStatus::Mismatch {
        warnings.push(format!(
            "Configuration switched, but the detected account hint did not match the target profile: {}",
            identity_verification.message
        ));
    }

    Ok(ProfileSwitchResult {
        profile,
        transaction,
        identity_verification,
        switched_environments: request.environments,
        manual_actions,
        warnings,
        closed_processes: std::mem::take(&mut closed_processes),
        restarted_apps,
    })
}

pub fn restore_default_on_exit_with_runtime<S: SecretStore, R: ProfileSwitchRuntime>(
    profile_repository: &ProfileRepository,
    app_state_repository: &AppStateRepository,
    vault: &SecretVault<S>,
    timestamp: String,
    runtime: &R,
) -> Result<RestoreDefaultOnExitResult, ProfileSwitchError> {
    let settings = app_state_repository
        .load_settings()
        .map_err(|error| ProfileSwitchError::AppState(format!("{error:?}")))?;
    if !settings.restore_default_on_exit {
        return Ok(RestoreDefaultOnExitResult {
            attempted: false,
            reason: "Restore default on exit is disabled".to_string(),
            switch_result: None,
        });
    }

    let profiles = profile_repository.list_profiles()?;
    let Some(default_profile) = profiles.iter().find(|profile| profile.default_profile) else {
        return Ok(RestoreDefaultOnExitResult {
            attempted: false,
            reason: "No default profile is configured".to_string(),
            switch_result: None,
        });
    };

    let current_profile_id = profiles
        .iter()
        .filter_map(|profile| {
            profile
                .last_used_at
                .as_ref()
                .map(|used_at| (used_at.parse::<u64>().unwrap_or(0), profile.id.as_str()))
        })
        .max_by_key(|(used_at, _)| *used_at)
        .map(|(_, id)| id);
    if current_profile_id == Some(default_profile.id.as_str()) {
        return Ok(RestoreDefaultOnExitResult {
            attempted: false,
            reason: "Default profile is already the latest used profile".to_string(),
            switch_result: None,
        });
    }

    let environments: Vec<TargetEnvironment> = settings
        .default_scope
        .iter()
        .copied()
        .filter(|environment| default_profile.supports(*environment))
        .collect();
    if environments.is_empty() {
        return Ok(RestoreDefaultOnExitResult {
            attempted: false,
            reason: "Default profile has no available environments in the default switch scope"
                .to_string(),
            switch_result: None,
        });
    }

    let switch_result = switch_saved_profile_with_runtime(
        ProfileSwitchRequest {
            profile_id: default_profile.id.clone(),
            environments,
            auto_restart_apps: false,
            vscode_reload_mode: VscodeReloadMode::None,
            confirm_process_close: !settings.confirm_before_closing_apps,
            desktop_app_path: None,
            vscode_app_path: None,
            quit_timeout_ms: 8000,
        },
        profile_repository,
        app_state_repository,
        vault,
        timestamp,
        runtime,
    )?;

    Ok(RestoreDefaultOnExitResult {
        attempted: true,
        reason: "Default profile restore transaction completed for app exit".to_string(),
        switch_result: Some(switch_result),
    })
}

pub fn retry_restart_desktop<R: ProfileSwitchRuntime>(
    runtime: &R,
    app_path: Option<&str>,
) -> Result<RestartAppResult, ProfileSwitchError> {
    runtime.restart_desktop(app_path)?;
    Ok(RestartAppResult {
        target: "desktop".to_string(),
        restarted: true,
        message: "Codex Desktop restart requested".to_string(),
    })
}

pub fn retry_restart_vscode<R: ProfileSwitchRuntime>(
    runtime: &R,
    app_path: Option<&str>,
) -> Result<RestartAppResult, ProfileSwitchError> {
    runtime.restart_vscode(app_path)?;
    Ok(RestartAppResult {
        target: "vscode".to_string(),
        restarted: true,
        message: "VS Code restart requested".to_string(),
    })
}

fn verify_switched_identity(
    profile: &ProfileMetadata,
    plan: &RestorePlan,
    transaction: &SwitchTransaction,
) -> SwitchIdentityVerification {
    let target_account_hint = profile.account_hint.trim().to_string();
    if transaction.phase != TransactionPhase::Completed {
        return SwitchIdentityVerification {
            status: SwitchIdentityStatus::NotChecked,
            target_account_hint,
            observed: Vec::new(),
            message: "Restore transaction did not complete, so account identity was not checked"
                .to_string(),
        };
    }

    let mut observed = Vec::new();
    for artifact in &plan.artifacts {
        let Some(environment) = target_environment_from_key(&artifact.environment) else {
            continue;
        };
        let account_hint = redacted_account_hint_from_path(&artifact.target_path);
        let duplicate = observed
            .iter()
            .any(|entry: &EnvironmentIdentityObservation| {
                entry.environment == environment && entry.account_hint == account_hint
            });
        if !duplicate {
            observed.push(EnvironmentIdentityObservation {
                environment,
                account_hint,
            });
        }
    }

    let observed_hints: Vec<String> = observed
        .iter()
        .filter_map(|entry| entry.account_hint.as_deref())
        .filter(|hint| !hint.trim().is_empty() && *hint != "Unknown")
        .map(ToOwned::to_owned)
        .collect();

    if target_account_hint.is_empty() || target_account_hint == "Unknown" {
        return SwitchIdentityVerification {
            status: SwitchIdentityStatus::Incomplete,
            target_account_hint,
            observed,
            message: "Target profile has no redacted account hint to compare".to_string(),
        };
    }

    if observed_hints.is_empty() {
        return SwitchIdentityVerification {
            status: SwitchIdentityStatus::Incomplete,
            target_account_hint,
            observed,
            message: "No redacted account hint was discoverable after restore".to_string(),
        };
    }

    let mismatches: Vec<&str> = observed_hints
        .iter()
        .filter(|hint| hint.as_str() != target_account_hint)
        .map(|hint| hint.as_str())
        .collect();
    if !mismatches.is_empty() {
        return SwitchIdentityVerification {
            status: SwitchIdentityStatus::Mismatch,
            target_account_hint: target_account_hint.clone(),
            observed,
            message: format!(
                "expected {target_account_hint}, observed {}",
                mismatches.join(", ")
            ),
        };
    }

    SwitchIdentityVerification {
        status: SwitchIdentityStatus::Verified,
        target_account_hint: target_account_hint.clone(),
        observed,
        message: format!("Observed redacted account hint matched {target_account_hint}"),
    }
}

fn target_environment_from_key(key: &str) -> Option<TargetEnvironment> {
    match key {
        "cli" => Some(TargetEnvironment::Cli),
        "vscode" => Some(TargetEnvironment::Vscode),
        "desktop" => Some(TargetEnvironment::Desktop),
        _ => None,
    }
}

pub fn restore_plan_from_profile<S: SecretStore>(
    profile: &ProfileMetadata,
    environments: &[TargetEnvironment],
    vault: &SecretVault<S>,
    timestamp: &str,
) -> Result<RestorePlan, ProfileSwitchError> {
    let mut artifacts = Vec::new();
    for environment in environments {
        let state = profile
            .environments
            .iter()
            .find(|state| state.environment == *environment)
            .ok_or(ProfileSwitchError::EnvironmentUnavailable(*environment))?;
        if state.status != ProfileAuthStatus::Available {
            return Err(ProfileSwitchError::EnvironmentUnavailable(*environment));
        }
        let payload = vault
            .load_profile_payload(&profile.id, *environment)?
            .ok_or(ProfileSwitchError::SecretMissing(*environment))?;
        let snapshot: EnvironmentSnapshot = serde_json::from_str(&payload)
            .map_err(|error| ProfileSwitchError::Snapshot(error.to_string()))?;
        for artifact in snapshot.artifacts {
            if let Some(content_base64) = artifact.content_base64 {
                artifacts.push(RestoreArtifact {
                    environment: environment.key().to_string(),
                    target_path: PathBuf::from(artifact.source_path),
                    content_base64,
                });
            }
        }
    }

    if artifacts.is_empty() {
        return Err(ProfileSwitchError::NoRestorableArtifacts);
    }

    Ok(RestorePlan {
        transaction_id: format!("switch-{timestamp}-{}", profile.id),
        target_profile_id: profile.id.clone(),
        artifacts,
    })
}

fn latest_used_profile_name(
    profiles: &[ProfileMetadata],
    target_profile_id: &str,
) -> Option<String> {
    profiles
        .iter()
        .filter(|profile| profile.id != target_profile_id)
        .filter_map(|profile| {
            profile
                .last_used_at
                .as_ref()
                .map(|used_at| (used_at.parse::<u64>().unwrap_or(0), profile.name.clone()))
        })
        .max_by_key(|(used_at, _)| *used_at)
        .map(|(_, name)| name)
}

fn manual_actions_for(request: &ProfileSwitchRequest) -> Vec<String> {
    let mut actions = Vec::new();
    if request.environments.contains(&TargetEnvironment::Cli) {
        actions.push("Run codex --version or a harmless Codex CLI status command to confirm CLI availability".to_string());
    }
    if request.environments.contains(&TargetEnvironment::Vscode) {
        match request.vscode_reload_mode {
            VscodeReloadMode::ManualReloadWindow => {
                actions.push(
                    "In VS Code, run Developer: Reload Window after saving any unsaved work"
                        .to_string(),
                );
            }
            VscodeReloadMode::RestartApp => {
                actions.push("VS Code restart was requested; if restart fails, the restore transaction is rolled back".to_string());
            }
            VscodeReloadMode::None => {}
        }
    }
    if request.environments.contains(&TargetEnvironment::Desktop) && request.auto_restart_apps {
        actions.push("Codex Desktop restart was requested; if restart fails, the restore transaction is rolled back".to_string());
    }
    actions
}

pub trait ProfileSwitchRuntime {
    fn cli_running_tasks(&self) -> Result<Vec<String>, ProfileSwitchError>;
    fn desktop_running_processes(&self) -> Result<Vec<String>, ProfileSwitchError>;
    fn quit_desktop(&self) -> Result<(), ProfileSwitchError>;
    fn restart_desktop(&self, app_path: Option<&str>) -> Result<(), ProfileSwitchError>;
    fn vscode_running_processes(&self) -> Result<Vec<String>, ProfileSwitchError>;
    fn quit_vscode(&self) -> Result<(), ProfileSwitchError>;
    fn restart_vscode(&self, app_path: Option<&str>) -> Result<(), ProfileSwitchError>;
}

pub struct SystemProfileSwitchRuntime {
    cli: SystemCliRuntime,
    desktop: MacDesktopProcessController,
    vscode: MacVscodeProcessController,
}

impl Default for SystemProfileSwitchRuntime {
    fn default() -> Self {
        Self {
            cli: SystemCliRuntime::new(None),
            desktop: MacDesktopProcessController,
            vscode: MacVscodeProcessController,
        }
    }
}

impl ProfileSwitchRuntime for SystemProfileSwitchRuntime {
    fn cli_running_tasks(&self) -> Result<Vec<String>, ProfileSwitchError> {
        self.cli
            .running_tasks()
            .map_err(|error| ProfileSwitchError::Process(format!("{error:?}")))
    }

    fn desktop_running_processes(&self) -> Result<Vec<String>, ProfileSwitchError> {
        self.desktop
            .running_processes()
            .map_err(|error| ProfileSwitchError::Process(format!("{error:?}")))
    }

    fn quit_desktop(&self) -> Result<(), ProfileSwitchError> {
        self.desktop
            .request_quit(&["Codex", "Codex Desktop", "OpenAI Codex"])
            .map_err(|error| ProfileSwitchError::Process(format!("{error:?}")))
    }

    fn restart_desktop(&self, app_path: Option<&str>) -> Result<(), ProfileSwitchError> {
        self.desktop
            .restart(app_path)
            .map_err(|error| ProfileSwitchError::Process(format!("{error:?}")))
    }

    fn vscode_running_processes(&self) -> Result<Vec<String>, ProfileSwitchError> {
        self.vscode
            .running_processes()
            .map_err(|error| ProfileSwitchError::Process(format!("{error:?}")))
    }

    fn quit_vscode(&self) -> Result<(), ProfileSwitchError> {
        self.vscode
            .request_quit(&["Visual Studio Code", "Code", "Code - Insiders"])
            .map_err(|error| ProfileSwitchError::Process(format!("{error:?}")))
    }

    fn restart_vscode(&self, app_path: Option<&str>) -> Result<(), ProfileSwitchError> {
        self.vscode
            .restart(app_path)
            .map_err(|error| ProfileSwitchError::Process(format!("{error:?}")))
    }
}

fn close_running_processes<R: ProfileSwitchRuntime>(
    request: &ProfileSwitchRequest,
    runtime: &R,
) -> Result<Vec<String>, ProfileSwitchError> {
    let mut running = Vec::new();
    if request.environments.contains(&TargetEnvironment::Cli) {
        let tasks = runtime.cli_running_tasks()?;
        if !tasks.is_empty() {
            return Err(ProfileSwitchError::CliTaskRunning(tasks));
        }
    }
    if request.environments.contains(&TargetEnvironment::Desktop) {
        running.extend(runtime.desktop_running_processes()?);
    }
    if request.environments.contains(&TargetEnvironment::Vscode) {
        running.extend(runtime.vscode_running_processes()?);
    }
    if !running.is_empty() && !request.confirm_process_close {
        return Err(ProfileSwitchError::ProcessCloseConfirmationRequired(
            running,
        ));
    }

    let mut closed = Vec::new();
    if request.environments.contains(&TargetEnvironment::Desktop)
        && !runtime.desktop_running_processes()?.is_empty()
    {
        runtime.quit_desktop()?;
        wait_until(
            "desktop",
            || runtime.desktop_running_processes(),
            request.quit_timeout_ms,
        )?;
        closed.push("Codex Desktop".to_string());
    }
    if request.environments.contains(&TargetEnvironment::Vscode)
        && !runtime.vscode_running_processes()?.is_empty()
    {
        runtime.quit_vscode()?;
        wait_until(
            "vscode",
            || runtime.vscode_running_processes(),
            request.quit_timeout_ms,
        )?;
        closed.push("VS Code".to_string());
    }
    Ok(closed)
}

fn restart_after_restore<R: ProfileSwitchRuntime>(
    request: &ProfileSwitchRequest,
    runtime: &R,
    restarted_apps: &mut Vec<String>,
) -> Result<(), ProfileSwitchError> {
    if request.environments.contains(&TargetEnvironment::Desktop) && request.auto_restart_apps {
        runtime.restart_desktop(request.desktop_app_path.as_deref())?;
        restarted_apps.push("Codex Desktop".to_string());
    }
    if request.environments.contains(&TargetEnvironment::Vscode)
        && request.vscode_reload_mode == VscodeReloadMode::RestartApp
    {
        runtime.restart_vscode(request.vscode_app_path.as_deref())?;
        restarted_apps.push("VS Code".to_string());
    }
    Ok(())
}

fn wait_until<F>(
    label: &str,
    mut running_processes: F,
    timeout_ms: u64,
) -> Result<(), ProfileSwitchError>
where
    F: FnMut() -> Result<Vec<String>, ProfileSwitchError>,
{
    let start = Instant::now();
    let timeout = Duration::from_millis(timeout_ms.max(1));
    loop {
        let running = running_processes()?;
        if running.is_empty() {
            return Ok(());
        }
        if start.elapsed() >= timeout {
            return Err(ProfileSwitchError::Process(format!(
                "{label} did not exit before timeout: {}",
                running.join(", ")
            )));
        }
        thread::sleep(Duration::from_millis(100));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        importer::{CapturedArtifact, SnapshotPathKind},
        profile::EnvironmentProfileState,
        profile_store::ProfileRepository,
        secret_store::MemorySecretStore,
    };
    use base64::{engine::general_purpose::STANDARD, Engine};
    use std::{cell::RefCell, fs, path::Path, rc::Rc};

    #[derive(Default)]
    struct MockRuntimeState {
        cli_tasks: Vec<String>,
        desktop_running: bool,
        vscode_running: bool,
        closed: Vec<String>,
        restarted: Vec<String>,
        fail_desktop_restart: bool,
        fail_vscode_restart: bool,
    }

    #[derive(Clone, Default)]
    struct MockRuntime {
        state: Rc<RefCell<MockRuntimeState>>,
    }

    impl ProfileSwitchRuntime for MockRuntime {
        fn cli_running_tasks(&self) -> Result<Vec<String>, ProfileSwitchError> {
            Ok(self.state.borrow().cli_tasks.clone())
        }

        fn desktop_running_processes(&self) -> Result<Vec<String>, ProfileSwitchError> {
            if self.state.borrow().desktop_running {
                Ok(vec!["Codex".to_string()])
            } else {
                Ok(Vec::new())
            }
        }

        fn quit_desktop(&self) -> Result<(), ProfileSwitchError> {
            let mut state = self.state.borrow_mut();
            state.closed.push("desktop".to_string());
            state.desktop_running = false;
            Ok(())
        }

        fn restart_desktop(&self, _app_path: Option<&str>) -> Result<(), ProfileSwitchError> {
            let mut state = self.state.borrow_mut();
            if state.fail_desktop_restart {
                return Err(ProfileSwitchError::Process(
                    "desktop restart failed".to_string(),
                ));
            }
            state.restarted.push("desktop".to_string());
            Ok(())
        }

        fn vscode_running_processes(&self) -> Result<Vec<String>, ProfileSwitchError> {
            if self.state.borrow().vscode_running {
                Ok(vec!["Visual Studio Code".to_string()])
            } else {
                Ok(Vec::new())
            }
        }

        fn quit_vscode(&self) -> Result<(), ProfileSwitchError> {
            let mut state = self.state.borrow_mut();
            state.closed.push("vscode".to_string());
            state.vscode_running = false;
            Ok(())
        }

        fn restart_vscode(&self, _app_path: Option<&str>) -> Result<(), ProfileSwitchError> {
            let mut state = self.state.borrow_mut();
            if state.fail_vscode_restart {
                return Err(ProfileSwitchError::Process(
                    "vscode restart failed".to_string(),
                ));
            }
            state.restarted.push("vscode".to_string());
            Ok(())
        }
    }

    fn temp_root(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "codex-switch-profile-switch-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create temp root");
        path
    }

    fn profile(environment: TargetEnvironment) -> ProfileMetadata {
        ProfileMetadata {
            id: "profile-1".to_string(),
            name: "Work".to_string(),
            account_hint: "w***@example.com".to_string(),
            tags: Vec::new(),
            note: String::new(),
            default_profile: true,
            last_used_at: None,
            environments: vec![EnvironmentProfileState::available(
                environment,
                format!("profile:profile-1:environment:{}", environment.key()),
                "1000".to_string(),
            )],
        }
    }

    fn profile_with_environments(environments: &[TargetEnvironment]) -> ProfileMetadata {
        ProfileMetadata {
            id: "profile-1".to_string(),
            name: "Work".to_string(),
            account_hint: "w***@example.com".to_string(),
            tags: Vec::new(),
            note: String::new(),
            default_profile: true,
            last_used_at: None,
            environments: environments
                .iter()
                .map(|environment| {
                    EnvironmentProfileState::available(
                        *environment,
                        format!("profile:profile-1:environment:{}", environment.key()),
                        "1000".to_string(),
                    )
                })
                .collect(),
        }
    }

    fn store_snapshot(
        vault: &SecretVault<MemorySecretStore>,
        profile: &ProfileMetadata,
        environment: TargetEnvironment,
        target: &Path,
        content: &str,
    ) {
        let snapshot = EnvironmentSnapshot {
            schema_version: 1,
            environment,
            captured_at: "1000".to_string(),
            artifacts: vec![CapturedArtifact {
                kind: SnapshotPathKind::Auth,
                source_path: target.to_string_lossy().to_string(),
                relative_path: "auth.json".to_string(),
                content_base64: Some(STANDARD.encode(content.as_bytes())),
                skipped_reason: None,
            }],
        };
        vault
            .store_profile_payload(
                &profile.id,
                environment,
                &serde_json::to_string(&snapshot).expect("snapshot json"),
            )
            .expect("store snapshot");
    }

    #[test]
    fn builds_restore_plan_from_secret_snapshot() {
        let root = temp_root("plan");
        let target = root.join("auth.json");
        let profile = profile(TargetEnvironment::Cli);
        let vault = SecretVault::new(MemorySecretStore::default());
        store_snapshot(
            &vault,
            &profile,
            TargetEnvironment::Cli,
            &target,
            "new-auth",
        );

        let plan = restore_plan_from_profile(&profile, &[TargetEnvironment::Cli], &vault, "2000")
            .expect("restore plan");

        assert_eq!(plan.artifacts.len(), 1);
        assert_eq!(plan.artifacts[0].target_path, target);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn switch_saved_profile_restores_files_and_records_history() {
        let root = temp_root("switch");
        let target = root.join("auth.json");
        fs::write(&target, "old-auth").expect("write old");
        let profile = profile(TargetEnvironment::Cli);
        let vault = SecretVault::new(MemorySecretStore::default());
        store_snapshot(
            &vault,
            &profile,
            TargetEnvironment::Cli,
            &target,
            "new-auth",
        );
        let profile_repository = ProfileRepository::new(root.join("profiles.json"));
        profile_repository
            .upsert_profile(profile.clone())
            .expect("save profile");
        let app_state_repository = AppStateRepository::new(root.join("state"));

        let result = switch_saved_profile_with_runtime(
            ProfileSwitchRequest {
                profile_id: profile.id.clone(),
                environments: vec![TargetEnvironment::Cli],
                auto_restart_apps: false,
                vscode_reload_mode: VscodeReloadMode::None,
                confirm_process_close: true,
                desktop_app_path: None,
                vscode_app_path: None,
                quit_timeout_ms: 50,
            },
            &profile_repository,
            &app_state_repository,
            &vault,
            "3000".to_string(),
            &MockRuntime::default(),
        )
        .expect("switch profile");

        assert_eq!(result.transaction.phase, TransactionPhase::Completed);
        assert_eq!(
            result.identity_verification.status,
            SwitchIdentityStatus::Incomplete
        );
        assert_eq!(result.profile.last_used_at, Some("3000".to_string()));
        assert_eq!(fs::read_to_string(target).expect("read target"), "new-auth");
        let history = app_state_repository.list_history().expect("history");
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].from_profile, None);
        assert_eq!(history[0].status, SwitchHistoryStatus::Incomplete);
        assert_eq!(
            history[0].error_type,
            Some("IdentityIncomplete".to_string())
        );
        let journal = fs::read_to_string(app_state_repository.current_transaction_path())
            .expect("read transaction journal");
        let journal: SwitchTransaction = serde_json::from_str(&journal).expect("journal json");
        assert_eq!(journal.phase, TransactionPhase::Completed);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn switch_history_success_requires_matching_redacted_account_hint() {
        let root = temp_root("verified-identity");
        let target = root.join("auth.json");
        fs::write(&target, "old-auth").expect("write old");
        let profile = profile(TargetEnvironment::Cli);
        let vault = SecretVault::new(MemorySecretStore::default());
        store_snapshot(
            &vault,
            &profile,
            TargetEnvironment::Cli,
            &target,
            r#"{"email":"work@example.com"}"#,
        );
        let profile_repository = ProfileRepository::new(root.join("profiles.json"));
        profile_repository
            .upsert_profile(profile.clone())
            .expect("save profile");
        let app_state_repository = AppStateRepository::new(root.join("state"));

        let result = switch_saved_profile_with_runtime(
            ProfileSwitchRequest {
                profile_id: profile.id.clone(),
                environments: vec![TargetEnvironment::Cli],
                auto_restart_apps: false,
                vscode_reload_mode: VscodeReloadMode::None,
                confirm_process_close: true,
                desktop_app_path: None,
                vscode_app_path: None,
                quit_timeout_ms: 50,
            },
            &profile_repository,
            &app_state_repository,
            &vault,
            "3000".to_string(),
            &MockRuntime::default(),
        )
        .expect("switch profile");

        assert_eq!(
            result.identity_verification.status,
            SwitchIdentityStatus::Verified
        );
        assert!(result.warnings.is_empty());
        let history = app_state_repository.list_history().expect("history");
        assert_eq!(history[0].status, SwitchHistoryStatus::Success);
        assert_eq!(history[0].error_type, None);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn switch_history_records_identity_mismatch_without_logging_full_email() {
        let root = temp_root("mismatched-identity");
        let target = root.join("auth.json");
        fs::write(&target, "old-auth").expect("write old");
        let profile = profile(TargetEnvironment::Cli);
        let vault = SecretVault::new(MemorySecretStore::default());
        store_snapshot(
            &vault,
            &profile,
            TargetEnvironment::Cli,
            &target,
            r#"{"email":"other@example.com"}"#,
        );
        let profile_repository = ProfileRepository::new(root.join("profiles.json"));
        profile_repository
            .upsert_profile(profile.clone())
            .expect("save profile");
        let app_state_repository = AppStateRepository::new(root.join("state"));

        let result = switch_saved_profile_with_runtime(
            ProfileSwitchRequest {
                profile_id: profile.id.clone(),
                environments: vec![TargetEnvironment::Cli],
                auto_restart_apps: false,
                vscode_reload_mode: VscodeReloadMode::None,
                confirm_process_close: true,
                desktop_app_path: None,
                vscode_app_path: None,
                quit_timeout_ms: 50,
            },
            &profile_repository,
            &app_state_repository,
            &vault,
            "3000".to_string(),
            &MockRuntime::default(),
        )
        .expect("switch profile");

        assert_eq!(
            result.identity_verification.status,
            SwitchIdentityStatus::Mismatch
        );
        assert!(result
            .identity_verification
            .message
            .contains("o***@example.com"));
        assert!(!result
            .identity_verification
            .message
            .contains("other@example.com"));
        let history = app_state_repository.list_history().expect("history");
        assert_eq!(history[0].status, SwitchHistoryStatus::Incomplete);
        assert_eq!(history[0].error_type, Some("IdentityMismatch".to_string()));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn switch_history_records_previous_recent_profile() {
        let root = temp_root("previous-profile");
        let target = root.join("auth.json");
        fs::write(&target, "old-auth").expect("write old");
        let mut previous_profile = profile(TargetEnvironment::Cli);
        previous_profile.id = "profile-previous".to_string();
        previous_profile.name = "Previous".to_string();
        previous_profile.environments[0].secret_ref =
            Some("profile:profile-previous:environment:cli".to_string());
        previous_profile.last_used_at = Some("1000".to_string());
        let profile = profile(TargetEnvironment::Cli);
        let vault = SecretVault::new(MemorySecretStore::default());
        store_snapshot(
            &vault,
            &profile,
            TargetEnvironment::Cli,
            &target,
            "new-auth",
        );
        let profile_repository = ProfileRepository::new(root.join("profiles.json"));
        profile_repository
            .upsert_profile(previous_profile)
            .expect("save previous");
        profile_repository
            .upsert_profile(profile.clone())
            .expect("save target");
        let app_state_repository = AppStateRepository::new(root.join("state"));

        switch_saved_profile_with_runtime(
            ProfileSwitchRequest {
                profile_id: profile.id.clone(),
                environments: vec![TargetEnvironment::Cli],
                auto_restart_apps: false,
                vscode_reload_mode: VscodeReloadMode::None,
                confirm_process_close: true,
                desktop_app_path: None,
                vscode_app_path: None,
                quit_timeout_ms: 50,
            },
            &profile_repository,
            &app_state_repository,
            &vault,
            "3000".to_string(),
            &MockRuntime::default(),
        )
        .expect("switch profile");

        let history = app_state_repository.list_history().expect("history");
        assert_eq!(history[0].from_profile, Some("Previous".to_string()));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn restore_default_on_exit_skips_when_setting_is_disabled() {
        let root = temp_root("exit-disabled");
        let profile_repository = ProfileRepository::new(root.join("profiles.json"));
        let app_state_repository = AppStateRepository::new(root.join("state"));
        let vault = SecretVault::new(MemorySecretStore::default());

        let result = restore_default_on_exit_with_runtime(
            &profile_repository,
            &app_state_repository,
            &vault,
            "3000".to_string(),
            &MockRuntime::default(),
        )
        .expect("restore default on exit");

        assert!(!result.attempted);
        assert_eq!(result.reason, "Restore default on exit is disabled");
        assert!(result.switch_result.is_none());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn restore_default_on_exit_skips_when_default_is_current() {
        let root = temp_root("exit-default-current");
        let mut default_profile = profile(TargetEnvironment::Cli);
        default_profile.last_used_at = Some("2000".to_string());
        let profile_repository = ProfileRepository::new(root.join("profiles.json"));
        profile_repository
            .upsert_profile(default_profile)
            .expect("save default");
        let app_state_repository = AppStateRepository::new(root.join("state"));
        let mut settings = crate::app_state::AppSettings::default();
        settings.restore_default_on_exit = true;
        app_state_repository
            .save_settings(&settings)
            .expect("save settings");
        let vault = SecretVault::new(MemorySecretStore::default());

        let result = restore_default_on_exit_with_runtime(
            &profile_repository,
            &app_state_repository,
            &vault,
            "3000".to_string(),
            &MockRuntime::default(),
        )
        .expect("restore default on exit");

        assert!(!result.attempted);
        assert_eq!(
            result.reason,
            "Default profile is already the latest used profile"
        );
        assert!(result.switch_result.is_none());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn restore_default_on_exit_runs_default_profile_switch_transaction() {
        let root = temp_root("exit-restore-default");
        let target = root.join("auth.json");
        fs::write(&target, "other-auth").expect("write old");
        let mut default_profile = profile(TargetEnvironment::Cli);
        default_profile.last_used_at = Some("1000".to_string());
        let mut other_profile = profile(TargetEnvironment::Cli);
        other_profile.id = "profile-other".to_string();
        other_profile.name = "Other".to_string();
        other_profile.default_profile = false;
        other_profile.last_used_at = Some("2000".to_string());
        other_profile.environments[0].secret_ref =
            Some("profile:profile-other:environment:cli".to_string());
        let profile_repository = ProfileRepository::new(root.join("profiles.json"));
        profile_repository
            .upsert_profile(default_profile.clone())
            .expect("save default");
        profile_repository
            .upsert_profile(other_profile)
            .expect("save other");
        let app_state_repository = AppStateRepository::new(root.join("state"));
        let mut settings = crate::app_state::AppSettings::default();
        settings.restore_default_on_exit = true;
        settings.default_scope = vec![TargetEnvironment::Cli];
        app_state_repository
            .save_settings(&settings)
            .expect("save settings");
        let vault = SecretVault::new(MemorySecretStore::default());
        store_snapshot(
            &vault,
            &default_profile,
            TargetEnvironment::Cli,
            &target,
            r#"{"email":"work@example.com"}"#,
        );

        let result = restore_default_on_exit_with_runtime(
            &profile_repository,
            &app_state_repository,
            &vault,
            "3000".to_string(),
            &MockRuntime::default(),
        )
        .expect("restore default on exit");

        assert!(result.attempted);
        let switch_result = result.switch_result.expect("switch result");
        assert_eq!(switch_result.profile.id, default_profile.id);
        assert_eq!(
            switch_result.identity_verification.status,
            SwitchIdentityStatus::Verified
        );
        assert_eq!(
            fs::read_to_string(target).expect("read target"),
            r#"{"email":"work@example.com"}"#
        );
        let history = app_state_repository.list_history().expect("history");
        assert_eq!(history[0].from_profile, Some("Other".to_string()));
        assert_eq!(history[0].to_profile, "Work");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn retry_restart_desktop_uses_runtime_restart() {
        let runtime = MockRuntime::default();

        let result = retry_restart_desktop(&runtime, Some("/Applications/Codex.app"))
            .expect("retry desktop restart");

        assert!(result.restarted);
        assert_eq!(result.target, "desktop");
        assert_eq!(runtime.state.borrow().restarted, vec!["desktop"]);
    }

    #[test]
    fn retry_restart_vscode_reports_runtime_failure() {
        let runtime = MockRuntime::default();
        runtime.state.borrow_mut().fail_vscode_restart = true;

        let error = retry_restart_vscode(&runtime, Some("/Applications/Visual Studio Code.app"))
            .expect_err("retry vscode restart fails");

        assert_eq!(
            error,
            ProfileSwitchError::Process("vscode restart failed".to_string())
        );
    }

    #[test]
    fn running_processes_require_explicit_confirmation() {
        let root = temp_root("confirm-process");
        let target = root.join("desktop.json");
        fs::write(&target, "old").expect("write old");
        let profile = profile(TargetEnvironment::Desktop);
        let vault = SecretVault::new(MemorySecretStore::default());
        store_snapshot(&vault, &profile, TargetEnvironment::Desktop, &target, "new");
        let profile_repository = ProfileRepository::new(root.join("profiles.json"));
        profile_repository
            .upsert_profile(profile.clone())
            .expect("save profile");
        let app_state_repository = AppStateRepository::new(root.join("state"));
        let runtime = MockRuntime::default();
        runtime.state.borrow_mut().desktop_running = true;

        let error = switch_saved_profile_with_runtime(
            ProfileSwitchRequest {
                profile_id: profile.id.clone(),
                environments: vec![TargetEnvironment::Desktop],
                auto_restart_apps: true,
                vscode_reload_mode: VscodeReloadMode::None,
                confirm_process_close: false,
                desktop_app_path: Some("/Applications/Codex.app".to_string()),
                vscode_app_path: None,
                quit_timeout_ms: 50,
            },
            &profile_repository,
            &app_state_repository,
            &vault,
            "3000".to_string(),
            &runtime,
        )
        .expect_err("running desktop requires confirmation");

        assert_eq!(
            error,
            ProfileSwitchError::ProcessCloseConfirmationRequired(vec!["Codex".to_string()])
        );
        assert_eq!(fs::read_to_string(target).expect("read target"), "old");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn confirmed_switch_closes_and_restarts_selected_apps() {
        let root = temp_root("close-restart");
        let desktop_target = root.join("desktop.json");
        let vscode_target = root.join("vscode.json");
        fs::write(&desktop_target, "old-desktop").expect("write old desktop");
        fs::write(&vscode_target, "old-vscode").expect("write old vscode");
        let profile =
            profile_with_environments(&[TargetEnvironment::Desktop, TargetEnvironment::Vscode]);
        let vault = SecretVault::new(MemorySecretStore::default());
        store_snapshot(
            &vault,
            &profile,
            TargetEnvironment::Desktop,
            &desktop_target,
            "new-desktop",
        );
        store_snapshot(
            &vault,
            &profile,
            TargetEnvironment::Vscode,
            &vscode_target,
            "new-vscode",
        );
        let profile_repository = ProfileRepository::new(root.join("profiles.json"));
        profile_repository
            .upsert_profile(profile.clone())
            .expect("save profile");
        let app_state_repository = AppStateRepository::new(root.join("state"));
        let runtime = MockRuntime::default();
        {
            let mut state = runtime.state.borrow_mut();
            state.desktop_running = true;
            state.vscode_running = true;
        }

        let result = switch_saved_profile_with_runtime(
            ProfileSwitchRequest {
                profile_id: profile.id.clone(),
                environments: vec![TargetEnvironment::Desktop, TargetEnvironment::Vscode],
                auto_restart_apps: true,
                vscode_reload_mode: VscodeReloadMode::RestartApp,
                confirm_process_close: true,
                desktop_app_path: Some("/Applications/Codex.app".to_string()),
                vscode_app_path: Some("/Applications/Visual Studio Code.app".to_string()),
                quit_timeout_ms: 50,
            },
            &profile_repository,
            &app_state_repository,
            &vault,
            "3000".to_string(),
            &runtime,
        )
        .expect("switch profile");

        assert_eq!(result.transaction.phase, TransactionPhase::Completed);
        assert_eq!(
            fs::read_to_string(desktop_target).expect("read desktop"),
            "new-desktop"
        );
        assert_eq!(
            fs::read_to_string(vscode_target).expect("read vscode"),
            "new-vscode"
        );
        assert_eq!(runtime.state.borrow().closed, vec!["desktop", "vscode"]);
        assert_eq!(runtime.state.borrow().restarted, vec!["desktop", "vscode"]);
        assert_eq!(result.closed_processes, vec!["Codex Desktop", "VS Code"]);
        assert_eq!(result.restarted_apps, vec!["Codex Desktop", "VS Code"]);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn restart_failure_rolls_back_combined_switch() {
        let root = temp_root("restart-rollback");
        let target = root.join("desktop.json");
        fs::write(&target, "old").expect("write old");
        let profile = profile(TargetEnvironment::Desktop);
        let vault = SecretVault::new(MemorySecretStore::default());
        store_snapshot(&vault, &profile, TargetEnvironment::Desktop, &target, "new");
        let profile_repository = ProfileRepository::new(root.join("profiles.json"));
        profile_repository
            .upsert_profile(profile.clone())
            .expect("save profile");
        let app_state_repository = AppStateRepository::new(root.join("state"));
        let runtime = MockRuntime::default();
        runtime.state.borrow_mut().fail_desktop_restart = true;

        let result = switch_saved_profile_with_runtime(
            ProfileSwitchRequest {
                profile_id: profile.id.clone(),
                environments: vec![TargetEnvironment::Desktop],
                auto_restart_apps: true,
                vscode_reload_mode: VscodeReloadMode::None,
                confirm_process_close: true,
                desktop_app_path: Some("/Applications/Codex.app".to_string()),
                vscode_app_path: None,
                quit_timeout_ms: 50,
            },
            &profile_repository,
            &app_state_repository,
            &vault,
            "3000".to_string(),
            &runtime,
        )
        .expect("restart failure returns failed transaction");

        assert_eq!(result.transaction.phase, TransactionPhase::Failed);
        assert_eq!(fs::read_to_string(target).expect("read target"), "old");
        assert!(result
            .transaction
            .events
            .iter()
            .any(|event| event.phase == TransactionPhase::RolledBack));
        let journal = fs::read_to_string(app_state_repository.current_transaction_path())
            .expect("read transaction journal");
        let journal: SwitchTransaction = serde_json::from_str(&journal).expect("journal json");
        assert_eq!(journal.phase, TransactionPhase::Failed);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn unavailable_environment_is_rejected() {
        let root = temp_root("missing-env");
        let profile = profile(TargetEnvironment::Cli);
        let vault = SecretVault::new(MemorySecretStore::default());

        let error =
            restore_plan_from_profile(&profile, &[TargetEnvironment::Desktop], &vault, "2000")
                .expect_err("desktop is unavailable");

        assert_eq!(
            error,
            ProfileSwitchError::EnvironmentUnavailable(TargetEnvironment::Desktop)
        );
        let _ = fs::remove_dir_all(root);
    }
}
