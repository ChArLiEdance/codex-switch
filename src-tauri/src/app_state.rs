use crate::{
    profile::TargetEnvironment,
    switch_transaction::TransactionRunner,
    switch_transaction::{BackupManifest, SwitchTransaction, TransactionEvent, TransactionPhase},
    PathKind,
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub default_scope: Vec<TargetEnvironment>,
    pub confirm_before_closing_apps: bool,
    pub auto_restart_apps: bool,
    pub restore_default_on_exit: bool,
    pub vscode_reload_mode: VscodeReloadMode,
    #[serde(default)]
    pub custom_paths: Vec<EnvironmentPathOverride>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            default_scope: vec![
                TargetEnvironment::Cli,
                TargetEnvironment::Vscode,
                TargetEnvironment::Desktop,
            ],
            confirm_before_closing_apps: true,
            auto_restart_apps: true,
            restore_default_on_exit: false,
            vscode_reload_mode: VscodeReloadMode::ManualReloadWindow,
            custom_paths: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentPathOverride {
    pub environment: TargetEnvironment,
    pub kind: PathKind,
    pub path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VscodeReloadMode {
    ManualReloadWindow,
    RestartApp,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SwitchHistoryStatus {
    Success,
    Failed,
    RolledBack,
    Incomplete,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchHistoryEntry {
    pub id: String,
    pub switched_at: String,
    #[serde(default)]
    pub from_profile_id: Option<String>,
    pub from_profile: Option<String>,
    #[serde(default)]
    pub to_profile_id: Option<String>,
    pub to_profile: String,
    pub environments: Vec<TargetEnvironment>,
    pub status: SwitchHistoryStatus,
    pub error_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryStatus {
    pub needs_recovery: bool,
    pub transaction_id: Option<String>,
    pub phase: Option<String>,
    pub message: String,
    pub backup_manifest_found: bool,
    pub backup_entry_count: Option<usize>,
    pub rollback_available: bool,
    pub latest_event_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryRollbackResult {
    pub transaction: SwitchTransaction,
    pub status: RecoveryStatus,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HistoryDocument {
    schema_version: u32,
    entries: Vec<SwitchHistoryEntry>,
}

impl Default for HistoryDocument {
    fn default() -> Self {
        Self {
            schema_version: 1,
            entries: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppStateError {
    Io(String),
    Json(String),
    Recovery(String),
}

pub struct AppStateRepository {
    root: PathBuf,
}

impl AppStateRepository {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn load_settings(&self) -> Result<AppSettings, AppStateError> {
        let path = self.settings_path();
        if !path.exists() {
            return Ok(AppSettings::default());
        }
        let content =
            fs::read_to_string(path).map_err(|error| AppStateError::Io(error.to_string()))?;
        serde_json::from_str(&content).map_err(|error| AppStateError::Json(error.to_string()))
    }

    pub fn save_settings(&self, settings: &AppSettings) -> Result<(), AppStateError> {
        write_json_atomic(&self.settings_path(), settings)
    }

    pub fn list_history(&self) -> Result<Vec<SwitchHistoryEntry>, AppStateError> {
        Ok(self.load_history()?.entries)
    }

    pub fn append_history(&self, entry: SwitchHistoryEntry) -> Result<(), AppStateError> {
        let mut document = self.load_history()?;
        document.entries.insert(0, entry);
        document.entries.truncate(200);
        write_json_atomic(&self.history_path(), &document)
    }

    pub fn clear_history(&self) -> Result<(), AppStateError> {
        write_json_atomic(&self.history_path(), &HistoryDocument::default())
    }

    pub fn current_transaction_path(&self) -> PathBuf {
        self.root.join("transactions").join("current.json")
    }

    pub fn save_current_transaction(
        &self,
        transaction: &SwitchTransaction,
    ) -> Result<(), AppStateError> {
        write_json_atomic(&self.current_transaction_path(), transaction)
    }

    pub fn resolve_unfinished_transaction(&self) -> Result<RecoveryStatus, AppStateError> {
        let path = self.current_transaction_path();
        if !path.exists() {
            return load_recovery_status(self);
        }
        let content =
            fs::read_to_string(&path).map_err(|error| AppStateError::Io(error.to_string()))?;
        let mut transaction: SwitchTransaction = serde_json::from_str(&content)
            .map_err(|error| AppStateError::Json(error.to_string()))?;
        let terminal = matches!(
            transaction.phase,
            TransactionPhase::Completed | TransactionPhase::RolledBack | TransactionPhase::Failed
        );
        if !terminal {
            transaction.phase = TransactionPhase::Failed;
            transaction.events.push(TransactionEvent {
                phase: TransactionPhase::Failed,
                message: "Recovery marked unresolved transaction as failed after user review"
                    .to_string(),
            });
            self.save_current_transaction(&transaction)?;
        }
        load_recovery_status(self)
    }

    pub fn rollback_unfinished_transaction_from_backup(
        &self,
    ) -> Result<RecoveryRollbackResult, AppStateError> {
        let mut transaction = self.load_current_transaction()?;
        if is_terminal_phase(transaction.phase) {
            return Err(AppStateError::Recovery(format!(
                "Transaction is already terminal: {:?}",
                transaction.phase
            )));
        }
        let manifest = self.load_backup_manifest(&transaction.id)?;
        if manifest.transaction_id != transaction.id {
            return Err(AppStateError::Recovery(format!(
                "Backup manifest transaction id mismatch: expected {}, found {}",
                transaction.id, manifest.transaction_id
            )));
        }

        transaction.transition(
            TransactionPhase::RollingBack,
            "Manual recovery rollback from persisted backup manifest",
        );
        self.save_current_transaction(&transaction)?;
        let runner = TransactionRunner::new(self.root.join("backups"));
        match runner.rollback(&manifest) {
            Ok(()) => {
                transaction.transition(
                    TransactionPhase::RolledBack,
                    "Manual recovery rollback complete",
                );
                self.save_current_transaction(&transaction)?;
                Ok(RecoveryRollbackResult {
                    status: load_recovery_status(self)?,
                    transaction,
                    message: "Manual recovery rollback completed".to_string(),
                })
            }
            Err(error) => {
                transaction.transition(
                    TransactionPhase::Failed,
                    format!("Manual recovery rollback failed: {error:?}"),
                );
                self.save_current_transaction(&transaction)?;
                Ok(RecoveryRollbackResult {
                    status: load_recovery_status(self)?,
                    transaction,
                    message: format!("Manual recovery rollback failed: {error:?}"),
                })
            }
        }
    }

    fn load_current_transaction(&self) -> Result<SwitchTransaction, AppStateError> {
        let path = self.current_transaction_path();
        if !path.exists() {
            return Err(AppStateError::Recovery(
                "No transaction journal found".to_string(),
            ));
        }
        let content =
            fs::read_to_string(path).map_err(|error| AppStateError::Io(error.to_string()))?;
        serde_json::from_str(&content).map_err(|error| AppStateError::Json(error.to_string()))
    }

    fn load_backup_manifest(&self, transaction_id: &str) -> Result<BackupManifest, AppStateError> {
        let path = self.backup_manifest_path(transaction_id);
        if !path.exists() {
            return Err(AppStateError::Recovery(format!(
                "Backup manifest not found for transaction {transaction_id}"
            )));
        }
        let content =
            fs::read_to_string(path).map_err(|error| AppStateError::Io(error.to_string()))?;
        serde_json::from_str(&content).map_err(|error| AppStateError::Json(error.to_string()))
    }

    fn backup_manifest_path(&self, transaction_id: &str) -> PathBuf {
        self.root
            .join("backups")
            .join(transaction_id)
            .join("manifest.json")
    }

    fn load_history(&self) -> Result<HistoryDocument, AppStateError> {
        let path = self.history_path();
        if !path.exists() {
            return Ok(HistoryDocument::default());
        }
        let content =
            fs::read_to_string(path).map_err(|error| AppStateError::Io(error.to_string()))?;
        serde_json::from_str(&content).map_err(|error| AppStateError::Json(error.to_string()))
    }

    fn settings_path(&self) -> PathBuf {
        self.root.join("settings.json")
    }

    fn history_path(&self) -> PathBuf {
        self.root.join("history.json")
    }
}

pub fn default_app_state_dir(home: PathBuf) -> PathBuf {
    home.join(".codex-switch")
}

pub fn load_recovery_status(
    repository: &AppStateRepository,
) -> Result<RecoveryStatus, AppStateError> {
    let path = repository.current_transaction_path();
    if !path.exists() {
        return Ok(RecoveryStatus {
            needs_recovery: false,
            transaction_id: None,
            phase: None,
            message: "No unfinished transaction journal found".to_string(),
            backup_manifest_found: false,
            backup_entry_count: None,
            rollback_available: false,
            latest_event_message: None,
        });
    }

    let content = fs::read_to_string(path).map_err(|error| AppStateError::Io(error.to_string()))?;
    let transaction: SwitchTransaction =
        serde_json::from_str(&content).map_err(|error| AppStateError::Json(error.to_string()))?;
    let phase = format!("{:?}", transaction.phase);
    let latest_event_message = transaction.events.last().map(|event| event.message.clone());
    let complete = is_terminal_phase(transaction.phase);
    let backup_summary = load_backup_manifest_summary(repository, &transaction.id);
    let rollback_available = !complete && backup_summary.entry_count.unwrap_or(0) > 0;

    Ok(RecoveryStatus {
        needs_recovery: !complete,
        transaction_id: Some(transaction.id),
        phase: Some(phase.clone()),
        message: if complete {
            format!("Last transaction is terminal: {phase}")
        } else {
            format!("Unfinished transaction found in phase: {phase}")
        },
        backup_manifest_found: backup_summary.found,
        backup_entry_count: backup_summary.entry_count,
        rollback_available,
        latest_event_message,
    })
}

fn is_terminal_phase(phase: TransactionPhase) -> bool {
    matches!(
        phase,
        TransactionPhase::Completed | TransactionPhase::RolledBack | TransactionPhase::Failed
    )
}

struct BackupManifestSummary {
    found: bool,
    entry_count: Option<usize>,
}

fn load_backup_manifest_summary(
    repository: &AppStateRepository,
    transaction_id: &str,
) -> BackupManifestSummary {
    let path = repository
        .root()
        .join("backups")
        .join(transaction_id)
        .join("manifest.json");
    let Ok(content) = fs::read_to_string(path) else {
        return BackupManifestSummary {
            found: false,
            entry_count: None,
        };
    };
    let Ok(manifest) = serde_json::from_str::<BackupManifest>(&content) else {
        return BackupManifestSummary {
            found: true,
            entry_count: None,
        };
    };
    BackupManifestSummary {
        found: true,
        entry_count: Some(manifest.entries.len()),
    }
}

fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<(), AppStateError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| AppStateError::Io(error.to_string()))?;
    }
    let content = serde_json::to_string_pretty(value)
        .map_err(|error| AppStateError::Json(error.to_string()))?;
    let temporary_path = path.with_extension("json.tmp");
    fs::write(&temporary_path, content).map_err(|error| AppStateError::Io(error.to_string()))?;
    fs::rename(&temporary_path, path).map_err(|error| AppStateError::Io(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::switch_transaction::{BackupEntry, BackupManifest, SwitchTransaction};

    fn temp_root(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "codex-switch-app-state-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create temp dir");
        path
    }

    #[test]
    fn settings_round_trip_with_defaults() {
        let root = temp_root("settings");
        let repository = AppStateRepository::new(root.clone());

        assert_eq!(
            repository.load_settings().expect("default settings"),
            AppSettings::default()
        );
        let settings = AppSettings {
            auto_restart_apps: false,
            restore_default_on_exit: true,
            vscode_reload_mode: VscodeReloadMode::RestartApp,
            custom_paths: vec![EnvironmentPathOverride {
                environment: TargetEnvironment::Vscode,
                kind: PathKind::Auth,
                path: "~/Library/Application Support/Code/User/globalStorage/openai.chatgpt"
                    .to_string(),
            }],
            ..AppSettings::default()
        };
        repository.save_settings(&settings).expect("save settings");

        assert_eq!(repository.load_settings().expect("load settings"), settings);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn legacy_settings_without_custom_paths_load_with_empty_overrides() {
        let root = temp_root("legacy-settings");
        fs::write(
            root.join("settings.json"),
            r#"{
              "defaultScope": ["cli", "vscode", "desktop"],
              "confirmBeforeClosingApps": true,
              "autoRestartApps": true,
              "restoreDefaultOnExit": false,
              "vscodeReloadMode": "manual_reload_window"
            }"#,
        )
        .expect("write legacy settings");
        let repository = AppStateRepository::new(root.clone());

        let settings = repository.load_settings().expect("load legacy settings");

        assert!(settings.custom_paths.is_empty());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn history_append_and_clear_do_not_store_sensitive_content() {
        let root = temp_root("history");
        let repository = AppStateRepository::new(root.clone());
        repository
            .append_history(SwitchHistoryEntry {
                id: "history-1".to_string(),
                switched_at: "1000".to_string(),
                from_profile_id: Some("profile-old".to_string()),
                from_profile: Some("old".to_string()),
                to_profile_id: Some("profile-new".to_string()),
                to_profile: "new".to_string(),
                environments: vec![TargetEnvironment::Cli],
                status: SwitchHistoryStatus::Success,
                error_type: None,
            })
            .expect("append history");

        let content = fs::read_to_string(root.join("history.json")).expect("read history");
        assert!(content.contains("history-1"));
        assert!(!content.contains("access_token"));
        assert_eq!(repository.list_history().expect("list history").len(), 1);

        repository.clear_history().expect("clear history");
        assert!(repository.list_history().expect("list cleared").is_empty());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn legacy_history_without_profile_ids_loads_with_empty_identity_fields() {
        let root = temp_root("legacy-history");
        fs::write(
            root.join("history.json"),
            r#"{
              "schemaVersion": 1,
              "entries": [{
                "id": "history-legacy",
                "switchedAt": "1000",
                "fromProfile": "Old",
                "toProfile": "New",
                "environments": ["cli"],
                "status": "success",
                "errorType": null
              }]
            }"#,
        )
        .expect("write legacy history");
        let repository = AppStateRepository::new(root.clone());

        let history = repository.list_history().expect("load legacy history");

        assert_eq!(history.len(), 1);
        assert_eq!(history[0].from_profile_id, None);
        assert_eq!(history[0].from_profile, Some("Old".to_string()));
        assert_eq!(history[0].to_profile_id, None);
        assert_eq!(history[0].to_profile, "New");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn recovery_status_detects_unfinished_transaction() {
        let root = temp_root("recovery");
        let repository = AppStateRepository::new(root.clone());
        let transaction = SwitchTransaction::new("tx-1".to_string(), "profile-1".to_string());
        repository
            .save_current_transaction(&transaction)
            .expect("write transaction");

        let status = load_recovery_status(&repository).expect("recovery status");

        assert!(status.needs_recovery);
        assert_eq!(status.transaction_id, Some("tx-1".to_string()));
        assert!(!status.backup_manifest_found);
        assert_eq!(status.backup_entry_count, None);
        assert!(!status.rollback_available);
        assert_eq!(
            status.latest_event_message,
            Some("Transaction planned".to_string())
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn recovery_status_reports_backup_manifest_summary() {
        let root = temp_root("recovery-manifest");
        let repository = AppStateRepository::new(root.clone());
        let transaction =
            SwitchTransaction::new("tx-manifest".to_string(), "profile-1".to_string());
        repository
            .save_current_transaction(&transaction)
            .expect("write transaction");
        let manifest = BackupManifest {
            transaction_id: "tx-manifest".to_string(),
            entries: vec![BackupEntry {
                original_path: root.join("auth.json"),
                backup_path: Some(root.join("state/backups/tx-manifest/artifact-0.bak")),
                existed: true,
            }],
        };
        let manifest_path = root.join("backups/tx-manifest/manifest.json");
        fs::create_dir_all(manifest_path.parent().expect("manifest parent"))
            .expect("create manifest parent");
        fs::write(
            &manifest_path,
            serde_json::to_string_pretty(&manifest).expect("manifest json"),
        )
        .expect("write manifest");

        let status = load_recovery_status(&repository).expect("recovery status");

        assert!(status.needs_recovery);
        assert!(status.backup_manifest_found);
        assert_eq!(status.backup_entry_count, Some(1));
        assert!(status.rollback_available);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn recovery_status_treats_terminal_transaction_as_complete() {
        let root = temp_root("recovery-complete");
        let repository = AppStateRepository::new(root.clone());
        let mut transaction = SwitchTransaction::new("tx-2".to_string(), "profile-1".to_string());
        transaction.phase = TransactionPhase::Completed;
        repository
            .save_current_transaction(&transaction)
            .expect("write transaction");

        let status = load_recovery_status(&repository).expect("recovery status");

        assert!(!status.needs_recovery);
        assert_eq!(status.transaction_id, Some("tx-2".to_string()));
        assert!(!status.rollback_available);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn resolve_unfinished_transaction_marks_journal_failed() {
        let root = temp_root("recovery-resolve");
        let repository = AppStateRepository::new(root.clone());
        let transaction = SwitchTransaction::new("tx-3".to_string(), "profile-1".to_string());
        repository
            .save_current_transaction(&transaction)
            .expect("write transaction");

        let status = repository
            .resolve_unfinished_transaction()
            .expect("resolve transaction");

        assert!(!status.needs_recovery);
        let content =
            fs::read_to_string(repository.current_transaction_path()).expect("read journal");
        let transaction: SwitchTransaction = serde_json::from_str(&content).expect("journal json");
        assert_eq!(transaction.phase, TransactionPhase::Failed);
        assert!(transaction
            .events
            .iter()
            .any(|event| event.message.contains("user review")));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn manual_recovery_rollback_restores_from_backup_manifest() {
        let root = temp_root("recovery-rollback");
        let repository = AppStateRepository::new(root.clone());
        let target = root.join("auth.json");
        let backup = root.join("backups/tx-rollback/artifact-0.bak");
        fs::write(&target, "new").expect("write new target");
        fs::create_dir_all(backup.parent().expect("backup parent")).expect("create backup parent");
        fs::write(&backup, "old").expect("write backup");
        let manifest = BackupManifest {
            transaction_id: "tx-rollback".to_string(),
            entries: vec![BackupEntry {
                original_path: target.clone(),
                backup_path: Some(backup),
                existed: true,
            }],
        };
        let manifest_path = root.join("backups/tx-rollback/manifest.json");
        fs::write(
            &manifest_path,
            serde_json::to_string_pretty(&manifest).expect("manifest json"),
        )
        .expect("write manifest");
        let transaction =
            SwitchTransaction::new("tx-rollback".to_string(), "profile-1".to_string());
        repository
            .save_current_transaction(&transaction)
            .expect("write transaction");

        let result = repository
            .rollback_unfinished_transaction_from_backup()
            .expect("rollback from backup");

        assert_eq!(result.transaction.phase, TransactionPhase::RolledBack);
        assert!(!result.status.needs_recovery);
        assert_eq!(fs::read_to_string(target).expect("read target"), "old");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn manual_recovery_rollback_requires_backup_manifest() {
        let root = temp_root("recovery-rollback-missing-manifest");
        let repository = AppStateRepository::new(root.clone());
        let transaction =
            SwitchTransaction::new("tx-missing-manifest".to_string(), "profile-1".to_string());
        repository
            .save_current_transaction(&transaction)
            .expect("write transaction");

        let error = repository
            .rollback_unfinished_transaction_from_backup()
            .expect_err("rollback should require manifest");

        assert!(matches!(
            error,
            AppStateError::Recovery(message) if message.contains("Backup manifest not found")
        ));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn manual_recovery_rollback_rejects_terminal_transaction() {
        let root = temp_root("recovery-rollback-terminal");
        let repository = AppStateRepository::new(root.clone());
        let mut transaction =
            SwitchTransaction::new("tx-terminal".to_string(), "profile-1".to_string());
        transaction.phase = TransactionPhase::Completed;
        repository
            .save_current_transaction(&transaction)
            .expect("write transaction");

        let error = repository
            .rollback_unfinished_transaction_from_backup()
            .expect_err("terminal transaction should not rollback");

        assert!(matches!(
            error,
            AppStateError::Recovery(message) if message.contains("already terminal")
        ));
        let _ = fs::remove_dir_all(root);
    }
}
