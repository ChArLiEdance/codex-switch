use crate::{
    app_state::{
        AppStateRepository, SwitchHistoryEntry, SwitchHistoryStatus, VscodeReloadMode,
    },
    importer::EnvironmentSnapshot,
    profile::{ProfileAuthStatus, ProfileMetadata, TargetEnvironment},
    profile_store::{ProfileRepository, ProfileStoreError},
    secret_store::{SecretStore, SecretStoreError, SecretVault},
    switch_transaction::{RestoreArtifact, RestorePlan, SwitchTransaction, TransactionPhase, TransactionRunner},
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileSwitchRequest {
    pub profile_id: String,
    pub environments: Vec<TargetEnvironment>,
    pub auto_restart_apps: bool,
    pub vscode_reload_mode: VscodeReloadMode,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileSwitchResult {
    pub profile: ProfileMetadata,
    pub transaction: SwitchTransaction,
    pub switched_environments: Vec<TargetEnvironment>,
    pub manual_actions: Vec<String>,
    pub warnings: Vec<String>,
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
    if request.environments.is_empty() {
        return Err(ProfileSwitchError::NoEnvironmentSelected);
    }

    let profiles = profile_repository.list_profiles()?;
    let profile = profiles
        .into_iter()
        .find(|profile| profile.id == request.profile_id)
        .ok_or_else(|| ProfileSwitchError::ProfileNotFound(request.profile_id.clone()))?;

    let plan = restore_plan_from_profile(&profile, &request.environments, vault, &timestamp)?;
    let backup_root = app_state_repository.root().join("backups");
    let runner = TransactionRunner::new(backup_root);
    let transaction = runner
        .run(&plan)
        .map_err(|error| ProfileSwitchError::Transaction(format!("{error:?}")))?;
    let status = if transaction.phase == TransactionPhase::Completed {
        SwitchHistoryStatus::Success
    } else if transaction
        .events
        .iter()
        .any(|event| event.phase == TransactionPhase::RolledBack)
    {
        SwitchHistoryStatus::RolledBack
    } else {
        SwitchHistoryStatus::Failed
    };
    app_state_repository
        .append_history(SwitchHistoryEntry {
            id: format!("history-{timestamp}-{}", profile.id),
            switched_at: timestamp.clone(),
            from_profile: None,
            to_profile: profile.name.clone(),
            environments: request.environments.clone(),
            status,
            error_type: if transaction.phase == TransactionPhase::Completed {
                None
            } else {
                Some(format!("{:?}", transaction.phase))
            },
        })
        .map_err(|error| ProfileSwitchError::AppState(format!("{error:?}")))?;

    let manual_actions = manual_actions_for(&request);
    let mut warnings = Vec::new();
    if transaction.phase != TransactionPhase::Completed {
        warnings.push("Switch restore failed; rollback was attempted and post-switch actions were skipped".to_string());
    } else {
        warnings.push("Configuration switched, but account identity verification is not complete".to_string());
    }

    Ok(ProfileSwitchResult {
        profile,
        transaction,
        switched_environments: request.environments,
        manual_actions,
        warnings,
    })
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

fn manual_actions_for(request: &ProfileSwitchRequest) -> Vec<String> {
    let mut actions = Vec::new();
    if request.environments.contains(&TargetEnvironment::Cli) {
        actions.push("Run codex --version or a harmless Codex CLI status command to confirm CLI availability".to_string());
    }
    if request.environments.contains(&TargetEnvironment::Vscode) {
        match request.vscode_reload_mode {
            VscodeReloadMode::ManualReloadWindow => {
                actions.push("In VS Code, run Developer: Reload Window after saving any unsaved work".to_string());
            }
            VscodeReloadMode::RestartApp => {
                actions.push("VS Code restart is configured; restart support is available in the adapter but this UI command currently returns guidance only".to_string());
            }
            VscodeReloadMode::None => {}
        }
    }
    if request.environments.contains(&TargetEnvironment::Desktop) && request.auto_restart_apps {
        actions.push("Restart Codex Desktop App if it was open; automatic restart is available in the adapter but not yet invoked by this combined UI command".to_string());
    }
    actions
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
    use std::{fs, path::Path};

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
        store_snapshot(&vault, &profile, TargetEnvironment::Cli, &target, "new-auth");

        let plan = restore_plan_from_profile(
            &profile,
            &[TargetEnvironment::Cli],
            &vault,
            "2000",
        )
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
        store_snapshot(&vault, &profile, TargetEnvironment::Cli, &target, "new-auth");
        let profile_repository = ProfileRepository::new(root.join("profiles.json"));
        profile_repository
            .upsert_profile(profile.clone())
            .expect("save profile");
        let app_state_repository = AppStateRepository::new(root.join("state"));

        let result = switch_saved_profile(
            ProfileSwitchRequest {
                profile_id: profile.id.clone(),
                environments: vec![TargetEnvironment::Cli],
                auto_restart_apps: false,
                vscode_reload_mode: VscodeReloadMode::None,
            },
            &profile_repository,
            &app_state_repository,
            &vault,
            "3000".to_string(),
        )
        .expect("switch profile");

        assert_eq!(result.transaction.phase, TransactionPhase::Completed);
        assert_eq!(fs::read_to_string(target).expect("read target"), "new-auth");
        assert_eq!(app_state_repository.list_history().expect("history").len(), 1);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn unavailable_environment_is_rejected() {
        let root = temp_root("missing-env");
        let profile = profile(TargetEnvironment::Cli);
        let vault = SecretVault::new(MemorySecretStore::default());

        let error = restore_plan_from_profile(
            &profile,
            &[TargetEnvironment::Desktop],
            &vault,
            "2000",
        )
        .expect_err("desktop is unavailable");

        assert_eq!(error, ProfileSwitchError::EnvironmentUnavailable(TargetEnvironment::Desktop));
        let _ = fs::remove_dir_all(root);
    }
}
