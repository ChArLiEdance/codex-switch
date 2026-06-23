use crate::{profile::TargetEnvironment, switch_transaction::SwitchTransaction};
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
        }
    }
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
    pub from_profile: Option<String>,
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
        let content = fs::read_to_string(path).map_err(|error| AppStateError::Io(error.to_string()))?;
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

    fn load_history(&self) -> Result<HistoryDocument, AppStateError> {
        let path = self.history_path();
        if !path.exists() {
            return Ok(HistoryDocument::default());
        }
        let content = fs::read_to_string(path).map_err(|error| AppStateError::Io(error.to_string()))?;
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

pub fn load_recovery_status(repository: &AppStateRepository) -> Result<RecoveryStatus, AppStateError> {
    let path = repository.current_transaction_path();
    if !path.exists() {
        return Ok(RecoveryStatus {
            needs_recovery: false,
            transaction_id: None,
            phase: None,
            message: "No unfinished transaction journal found".to_string(),
        });
    }

    let content = fs::read_to_string(path).map_err(|error| AppStateError::Io(error.to_string()))?;
    let transaction: SwitchTransaction =
        serde_json::from_str(&content).map_err(|error| AppStateError::Json(error.to_string()))?;
    let phase = format!("{:?}", transaction.phase);
    let complete = matches!(
        transaction.phase,
        crate::switch_transaction::TransactionPhase::Completed
            | crate::switch_transaction::TransactionPhase::RolledBack
            | crate::switch_transaction::TransactionPhase::Failed
    );

    Ok(RecoveryStatus {
        needs_recovery: !complete,
        transaction_id: Some(transaction.id),
        phase: Some(phase.clone()),
        message: if complete {
            format!("Last transaction is terminal: {phase}")
        } else {
            format!("Unfinished transaction found in phase: {phase}")
        },
    })
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
    use crate::switch_transaction::SwitchTransaction;

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

        assert_eq!(repository.load_settings().expect("default settings"), AppSettings::default());
        let settings = AppSettings {
            auto_restart_apps: false,
            restore_default_on_exit: true,
            vscode_reload_mode: VscodeReloadMode::RestartApp,
            ..AppSettings::default()
        };
        repository.save_settings(&settings).expect("save settings");

        assert_eq!(repository.load_settings().expect("load settings"), settings);
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
                from_profile: Some("old".to_string()),
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
    fn recovery_status_detects_unfinished_transaction() {
        let root = temp_root("recovery");
        let repository = AppStateRepository::new(root.clone());
        let transaction = SwitchTransaction::new("tx-1".to_string(), "profile-1".to_string());
        write_json_atomic(&repository.current_transaction_path(), &transaction)
            .expect("write transaction");

        let status = load_recovery_status(&repository).expect("recovery status");

        assert!(status.needs_recovery);
        assert_eq!(status.transaction_id, Some("tx-1".to_string()));
        let _ = fs::remove_dir_all(root);
    }
}

