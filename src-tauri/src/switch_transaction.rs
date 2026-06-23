use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fs,
    path::{Component, Path, PathBuf},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransactionPhase {
    Planned,
    BackingUp,
    BackupComplete,
    Restoring,
    RestoreComplete,
    Verifying,
    Completed,
    RollingBack,
    RolledBack,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionEvent {
    pub phase: TransactionPhase,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchTransaction {
    pub id: String,
    pub target_profile_id: String,
    pub phase: TransactionPhase,
    pub events: Vec<TransactionEvent>,
}

impl SwitchTransaction {
    pub fn new(id: String, target_profile_id: String) -> Self {
        let mut transaction = Self {
            id,
            target_profile_id,
            phase: TransactionPhase::Planned,
            events: Vec::new(),
        };
        transaction.transition(TransactionPhase::Planned, "Transaction planned");
        transaction
    }

    fn transition(&mut self, phase: TransactionPhase, message: impl Into<String>) {
        self.phase = phase;
        self.events.push(TransactionEvent {
            phase,
            message: message.into(),
        });
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreArtifact {
    pub environment: String,
    pub kind: RestoreArtifactKind,
    pub target_path: PathBuf,
    pub content_base64: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unix_mode: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RestoreArtifactKind {
    Auth,
    Config,
    Cache,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestorePlan {
    pub transaction_id: String,
    pub target_profile_id: String,
    pub artifacts: Vec<RestoreArtifact>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupEntry {
    pub original_path: PathBuf,
    pub backup_path: Option<PathBuf>,
    pub existed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupManifest {
    pub transaction_id: String,
    pub entries: Vec<BackupEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransactionError {
    BackupVerification(String),
    EmptyRestorePlan,
    ConflictingRestoreTarget(String),
    InvalidBase64(String),
    Io(String),
    InjectedFailure,
    InvalidRestoreTarget(String),
    UnsafeRestoreTarget(String),
    Verification(String),
    PostRestore(String),
}

pub struct TransactionRunner {
    backup_root: PathBuf,
}

impl TransactionRunner {
    pub fn new(backup_root: PathBuf) -> Self {
        Self { backup_root }
    }

    pub fn run(&self, plan: &RestorePlan) -> Result<SwitchTransaction, TransactionError> {
        self.run_inner(plan, None, None, None, None)
    }

    pub fn run_with_post_restore<F>(
        &self,
        plan: &RestorePlan,
        mut post_restore: F,
    ) -> Result<SwitchTransaction, TransactionError>
    where
        F: FnMut() -> Result<(), TransactionError>,
    {
        self.run_inner(plan, None, None, None, Some(&mut post_restore))
    }

    #[cfg(test)]
    fn run_with_failure_after(
        &self,
        plan: &RestorePlan,
        writes_before_failure: usize,
    ) -> Result<SwitchTransaction, TransactionError> {
        self.run_inner(plan, Some(writes_before_failure), None, None, None)
    }

    #[cfg(test)]
    fn run_with_tamper_after_backup<F>(
        &self,
        plan: &RestorePlan,
        mut tamper_after_backup: F,
    ) -> Result<SwitchTransaction, TransactionError>
    where
        F: FnMut(&BackupManifest) -> Result<(), TransactionError>,
    {
        self.run_inner(plan, None, Some(&mut tamper_after_backup), None, None)
    }

    #[cfg(test)]
    fn run_with_tamper_after_restore<F>(
        &self,
        plan: &RestorePlan,
        mut tamper_after_restore: F,
    ) -> Result<SwitchTransaction, TransactionError>
    where
        F: FnMut() -> Result<(), TransactionError>,
    {
        self.run_inner(plan, None, None, Some(&mut tamper_after_restore), None)
    }

    fn run_inner(
        &self,
        plan: &RestorePlan,
        fail_after_writes: Option<usize>,
        tamper_after_backup: Option<
            &mut dyn FnMut(&BackupManifest) -> Result<(), TransactionError>,
        >,
        mut tamper_after_restore: Option<&mut dyn FnMut() -> Result<(), TransactionError>>,
        mut post_restore: Option<&mut dyn FnMut() -> Result<(), TransactionError>>,
    ) -> Result<SwitchTransaction, TransactionError> {
        if plan.artifacts.is_empty() {
            return Err(TransactionError::EmptyRestorePlan);
        }

        let mut transaction =
            SwitchTransaction::new(plan.transaction_id.clone(), plan.target_profile_id.clone());
        if let Err(error) = validate_unique_restore_targets(plan) {
            transaction.transition(
                TransactionPhase::Failed,
                format!("Restore plan rejected: {error:?}"),
            );
            return Ok(transaction);
        }
        if let Err(error) = self.validate_backup_location(plan) {
            transaction.transition(
                TransactionPhase::Failed,
                format!("Backup location rejected: {error:?}"),
            );
            return Ok(transaction);
        }
        transaction.transition(TransactionPhase::BackingUp, "Backing up current state");
        let manifest = match self.backup_inner(plan, tamper_after_backup) {
            Ok(manifest) => manifest,
            Err(error) => {
                transaction.transition(
                    TransactionPhase::Failed,
                    format!("Backup failed: {error:?}"),
                );
                return Ok(transaction);
            }
        };
        transaction.transition(TransactionPhase::BackupComplete, "Backup complete");
        transaction.transition(
            TransactionPhase::Restoring,
            "Restoring target profile state",
        );

        let restore_result = self.restore(plan, fail_after_writes);
        if let Err(error) = restore_result {
            transaction.transition(
                TransactionPhase::RollingBack,
                "Restore failed; rolling back",
            );
            self.rollback(&manifest)?;
            transaction.transition(TransactionPhase::RolledBack, "Rollback complete");
            transaction.transition(
                TransactionPhase::Failed,
                format!("Restore failed: {error:?}"),
            );
            return Ok(transaction);
        }

        transaction.transition(TransactionPhase::RestoreComplete, "Restore complete");
        if let Err(error) = self.refresh_cache(plan) {
            transaction.transition(
                TransactionPhase::RollingBack,
                "Cache refresh failed; rolling back",
            );
            self.rollback(&manifest)?;
            transaction.transition(TransactionPhase::RolledBack, "Rollback complete");
            transaction.transition(
                TransactionPhase::Failed,
                format!("Cache refresh failed: {error:?}"),
            );
            return Ok(transaction);
        }
        if let Some(tamper_after_restore) = tamper_after_restore.as_mut() {
            tamper_after_restore()?;
        }
        transaction.transition(
            TransactionPhase::Verifying,
            "Verifying restored auth/config artifacts",
        );
        if let Err(error) = self.verify_restored_artifacts(plan) {
            transaction.transition(
                TransactionPhase::RollingBack,
                "File verification failed; rolling back",
            );
            self.rollback(&manifest)?;
            transaction.transition(TransactionPhase::RolledBack, "Rollback complete");
            transaction.transition(
                TransactionPhase::Failed,
                format!("File verification failed: {error:?}"),
            );
            return Ok(transaction);
        }
        if let Some(post_restore) = post_restore.as_mut() {
            if let Err(error) = post_restore() {
                transaction.transition(
                    TransactionPhase::RollingBack,
                    "Post-restore action failed; rolling back",
                );
                self.rollback(&manifest)?;
                transaction.transition(TransactionPhase::RolledBack, "Rollback complete");
                transaction.transition(
                    TransactionPhase::Failed,
                    format!("Post-restore failed: {error:?}"),
                );
                return Ok(transaction);
            }
        }
        transaction.transition(TransactionPhase::Completed, "Transaction complete");
        Ok(transaction)
    }

    pub fn backup(&self, plan: &RestorePlan) -> Result<BackupManifest, TransactionError> {
        self.backup_inner(plan, None)
    }

    fn backup_inner(
        &self,
        plan: &RestorePlan,
        mut tamper_after_backup: Option<
            &mut dyn FnMut(&BackupManifest) -> Result<(), TransactionError>,
        >,
    ) -> Result<BackupManifest, TransactionError> {
        validate_unique_restore_targets(plan)?;
        self.validate_backup_location(plan)?;
        let transaction_backup_root = self.backup_root.join(&plan.transaction_id);
        fs::create_dir_all(&transaction_backup_root)
            .map_err(|error| TransactionError::Io(error.to_string()))?;

        let mut entries = Vec::new();
        for (index, artifact) in plan.artifacts.iter().enumerate() {
            let original_path = artifact.target_path.clone();
            if original_path.exists() {
                let backup_path = transaction_backup_root.join(format!("artifact-{index}.bak"));
                if original_path.is_dir() {
                    copy_dir_all(&original_path, &backup_path)?;
                } else {
                    if let Some(parent) = backup_path.parent() {
                        fs::create_dir_all(parent)
                            .map_err(|error| TransactionError::Io(error.to_string()))?;
                    }
                    fs::copy(&original_path, &backup_path)
                        .map_err(|error| TransactionError::Io(error.to_string()))?;
                }
                entries.push(BackupEntry {
                    original_path,
                    backup_path: Some(backup_path),
                    existed: true,
                });
            } else {
                entries.push(BackupEntry {
                    original_path,
                    backup_path: None,
                    existed: false,
                });
            }
        }

        let manifest = BackupManifest {
            transaction_id: plan.transaction_id.clone(),
            entries,
        };
        if let Some(tamper_after_backup) = tamper_after_backup.as_mut() {
            tamper_after_backup(&manifest)?;
        }
        verify_backup_manifest(&manifest)?;
        Ok(manifest)
    }

    fn validate_backup_location(&self, plan: &RestorePlan) -> Result<(), TransactionError> {
        validate_backup_transaction_id(&plan.transaction_id)?;
        validate_backup_directory(&self.backup_root, "backup root")?;
        validate_backup_directory(
            &self.backup_root.join(&plan.transaction_id),
            "transaction backup directory",
        )
    }

    fn restore(
        &self,
        plan: &RestorePlan,
        fail_after_writes: Option<usize>,
    ) -> Result<(), TransactionError> {
        let mut writes = 0;
        for artifact in &plan.artifacts {
            if fail_after_writes == Some(writes) {
                return Err(TransactionError::InjectedFailure);
            }
            let content = STANDARD
                .decode(&artifact.content_base64)
                .map_err(|error| TransactionError::InvalidBase64(error.to_string()))?;
            atomic_write(&artifact.target_path, &content, artifact.unix_mode)?;
            writes += 1;
        }
        Ok(())
    }

    fn verify_restored_artifacts(&self, plan: &RestorePlan) -> Result<(), TransactionError> {
        for artifact in plan
            .artifacts
            .iter()
            .filter(|artifact| artifact.kind != RestoreArtifactKind::Cache)
        {
            let expected = STANDARD
                .decode(&artifact.content_base64)
                .map_err(|error| TransactionError::InvalidBase64(error.to_string()))?;
            let actual = fs::read(&artifact.target_path)
                .map_err(|error| TransactionError::Verification(error.to_string()))?;
            if actual != expected {
                return Err(TransactionError::Verification(format!(
                    "restored content mismatch for {} {}",
                    artifact.environment,
                    artifact.target_path.display()
                )));
            }
            verify_unix_mode(artifact)?;
        }
        Ok(())
    }

    fn refresh_cache(&self, plan: &RestorePlan) -> Result<(), TransactionError> {
        let mut cache_paths = Vec::new();
        for artifact in plan
            .artifacts
            .iter()
            .filter(|artifact| artifact.kind == RestoreArtifactKind::Cache)
        {
            if !cache_paths.contains(&artifact.target_path) {
                cache_paths.push(artifact.target_path.clone());
            }
        }
        for path in cache_paths {
            if path.exists() {
                remove_path(&path)?;
            }
        }
        Ok(())
    }

    pub fn rollback(&self, manifest: &BackupManifest) -> Result<(), TransactionError> {
        for entry in manifest.entries.iter().rev() {
            if entry.existed {
                let Some(backup_path) = &entry.backup_path else {
                    continue;
                };
                if backup_path.is_dir() {
                    if entry.original_path.exists() {
                        remove_path(&entry.original_path)?;
                    }
                    copy_dir_all(backup_path, &entry.original_path)?;
                } else {
                    if let Some(parent) = entry.original_path.parent() {
                        fs::create_dir_all(parent)
                            .map_err(|error| TransactionError::Io(error.to_string()))?;
                    }
                    fs::copy(backup_path, &entry.original_path)
                        .map_err(|error| TransactionError::Io(error.to_string()))?;
                }
            } else if entry.original_path.exists() {
                remove_path(&entry.original_path)?;
            }
        }
        Ok(())
    }
}

fn validate_unique_restore_targets(plan: &RestorePlan) -> Result<(), TransactionError> {
    let mut seen = HashSet::new();
    for artifact in &plan.artifacts {
        if let Err(error) = reject_symlink_target_or_ancestor(&artifact.target_path) {
            return Err(error);
        }
        if let Err(error) = reject_non_file_target(&artifact.target_path) {
            return Err(error);
        }
        let normalized = normalize_restore_path(&artifact.target_path);
        if !seen.insert(normalized.clone()) {
            return Err(TransactionError::ConflictingRestoreTarget(format!(
                "duplicate restore target {}",
                normalized.display()
            )));
        }
    }
    Ok(())
}

fn verify_backup_manifest(manifest: &BackupManifest) -> Result<(), TransactionError> {
    for entry in manifest.entries.iter().filter(|entry| entry.existed) {
        let backup_path = entry.backup_path.as_ref().ok_or_else(|| {
            TransactionError::BackupVerification(format!(
                "missing backup path for {}",
                entry.original_path.display()
            ))
        })?;
        let original_bytes = fs::read(&entry.original_path).map_err(|error| {
            TransactionError::BackupVerification(format!(
                "unable to read original {}: {}",
                entry.original_path.display(),
                error
            ))
        })?;
        let backup_bytes = fs::read(backup_path).map_err(|error| {
            TransactionError::BackupVerification(format!(
                "unable to read backup {}: {}",
                backup_path.display(),
                error
            ))
        })?;
        if backup_bytes != original_bytes {
            return Err(TransactionError::BackupVerification(format!(
                "backup content mismatch for {}",
                entry.original_path.display()
            )));
        }
    }
    Ok(())
}

fn validate_backup_transaction_id(transaction_id: &str) -> Result<(), TransactionError> {
    let mut components = Path::new(transaction_id).components();
    match (components.next(), components.next()) {
        (Some(Component::Normal(_)), None) if !transaction_id.trim().is_empty() => Ok(()),
        _ => Err(TransactionError::InvalidRestoreTarget(format!(
            "transaction id is not a safe path segment {transaction_id}"
        ))),
    }
}

fn validate_backup_directory(path: &Path, label: &str) -> Result<(), TransactionError> {
    if path_is_symlink(path)? {
        return Err(TransactionError::UnsafeRestoreTarget(format!(
            "{label} is a symlink {}",
            path.display()
        )));
    }
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() => Ok(()),
        Ok(_) => Err(TransactionError::InvalidRestoreTarget(format!(
            "{label} is not a directory {}",
            path.display()
        ))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            reject_existing_symlink_parent(path, label)
        }
        Err(error) => Err(TransactionError::Io(error.to_string())),
    }
}

fn reject_existing_symlink_parent(path: &Path, label: &str) -> Result<(), TransactionError> {
    for ancestor in path.ancestors().skip(1) {
        if ancestor.as_os_str().is_empty() {
            continue;
        }
        match fs::symlink_metadata(ancestor) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(TransactionError::UnsafeRestoreTarget(format!(
                    "{label} ancestor is a symlink {}",
                    ancestor.display()
                )));
            }
            Ok(_) => return Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(TransactionError::Io(error.to_string())),
        }
    }
    Ok(())
}

fn reject_non_file_target(path: &Path) -> Result<(), TransactionError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() => Ok(()),
        Ok(metadata) => {
            let target_type = if metadata.is_dir() {
                "directory"
            } else {
                "unsupported filesystem entry"
            };
            Err(TransactionError::InvalidRestoreTarget(format!(
                "restore target is a {target_type} {}",
                path.display()
            )))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(TransactionError::Io(error.to_string())),
    }
}

fn reject_symlink_target_or_ancestor(path: &Path) -> Result<(), TransactionError> {
    if path_is_symlink(path)? {
        return Err(TransactionError::UnsafeRestoreTarget(format!(
            "restore target is a symlink {}",
            path.display()
        )));
    }
    for ancestor in path.ancestors().skip(1) {
        if ancestor.as_os_str().is_empty() {
            continue;
        }
        match fs::symlink_metadata(ancestor) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(TransactionError::UnsafeRestoreTarget(format!(
                    "restore target ancestor is a symlink {}",
                    ancestor.display()
                )));
            }
            Ok(_) => return Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(TransactionError::Io(error.to_string())),
        }
    }
    Ok(())
}

fn path_is_symlink(path: &Path) -> Result<bool, TransactionError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => Ok(metadata.file_type().is_symlink()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(TransactionError::Io(error.to_string())),
    }
}

fn normalize_restore_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(part) => normalized.push(part),
            Component::RootDir | Component::Prefix(_) => normalized.push(component.as_os_str()),
        }
    }
    normalized
}

fn atomic_write(
    path: &Path,
    content: &[u8],
    unix_mode: Option<u32>,
) -> Result<(), TransactionError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| TransactionError::Io(error.to_string()))?;
    }
    let temporary_path = path.with_extension("codex-switch.tmp");
    fs::write(&temporary_path, content).map_err(|error| TransactionError::Io(error.to_string()))?;
    fs::rename(&temporary_path, path).map_err(|error| TransactionError::Io(error.to_string()))?;
    apply_unix_mode(path, unix_mode)
}

#[cfg(unix)]
fn apply_unix_mode(path: &Path, unix_mode: Option<u32>) -> Result<(), TransactionError> {
    use std::os::unix::fs::PermissionsExt;

    let Some(mode) = unix_mode else {
        return Ok(());
    };
    let permissions = fs::Permissions::from_mode(mode);
    fs::set_permissions(path, permissions).map_err(|error| TransactionError::Io(error.to_string()))
}

#[cfg(not(unix))]
fn apply_unix_mode(_path: &Path, _unix_mode: Option<u32>) -> Result<(), TransactionError> {
    Ok(())
}

#[cfg(unix)]
fn verify_unix_mode(artifact: &RestoreArtifact) -> Result<(), TransactionError> {
    use std::os::unix::fs::PermissionsExt;

    let Some(expected_mode) = artifact.unix_mode else {
        return Ok(());
    };
    let metadata = fs::metadata(&artifact.target_path)
        .map_err(|error| TransactionError::Verification(error.to_string()))?;
    let actual_mode = metadata.permissions().mode() & 0o7777;
    if actual_mode != expected_mode {
        return Err(TransactionError::Verification(format!(
            "restored permission mismatch for {} {}",
            artifact.environment,
            artifact.target_path.display()
        )));
    }
    Ok(())
}

#[cfg(not(unix))]
fn verify_unix_mode(_artifact: &RestoreArtifact) -> Result<(), TransactionError> {
    Ok(())
}

fn copy_dir_all(source: &Path, destination: &Path) -> Result<(), TransactionError> {
    fs::create_dir_all(destination).map_err(|error| TransactionError::Io(error.to_string()))?;
    for entry in fs::read_dir(source).map_err(|error| TransactionError::Io(error.to_string()))? {
        let entry = entry.map_err(|error| TransactionError::Io(error.to_string()))?;
        let file_type = entry
            .file_type()
            .map_err(|error| TransactionError::Io(error.to_string()))?;
        let target = destination.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_all(&entry.path(), &target)?;
        } else if file_type.is_file() {
            fs::copy(entry.path(), target)
                .map_err(|error| TransactionError::Io(error.to_string()))?;
        }
    }
    Ok(())
}

fn remove_path(path: &Path) -> Result<(), TransactionError> {
    if path.is_dir() {
        fs::remove_dir_all(path).map_err(|error| TransactionError::Io(error.to_string()))
    } else {
        fs::remove_file(path).map_err(|error| TransactionError::Io(error.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "codex-switch-transaction-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create temp dir");
        path
    }

    fn artifact(path: PathBuf, content: &str) -> RestoreArtifact {
        RestoreArtifact {
            environment: "cli".to_string(),
            kind: RestoreArtifactKind::Auth,
            target_path: path,
            content_base64: STANDARD.encode(content.as_bytes()),
            unix_mode: None,
        }
    }

    fn cache_artifact(path: PathBuf, content: &str) -> RestoreArtifact {
        RestoreArtifact {
            environment: "desktop".to_string(),
            kind: RestoreArtifactKind::Cache,
            target_path: path,
            content_base64: STANDARD.encode(content.as_bytes()),
            unix_mode: None,
        }
    }

    #[test]
    fn successful_transaction_backs_up_and_restores_files() {
        let root = temp_dir("success");
        let target = root.join("auth.json");
        fs::write(&target, "old").expect("write old file");
        let plan = RestorePlan {
            transaction_id: "tx-success".to_string(),
            target_profile_id: "profile-1".to_string(),
            artifacts: vec![artifact(target.clone(), "new")],
        };
        let runner = TransactionRunner::new(root.join("backups"));

        let transaction = runner.run(&plan).expect("run transaction");

        assert_eq!(transaction.phase, TransactionPhase::Completed);
        assert_eq!(fs::read_to_string(target).expect("read target"), "new");
        assert!(root.join("backups/tx-success/artifact-0.bak").exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn backup_verification_failure_skips_restore() {
        let root = temp_dir("backup-verify");
        let target = root.join("auth.json");
        fs::write(&target, "old").expect("write old file");
        let plan = RestorePlan {
            transaction_id: "tx-backup-verify".to_string(),
            target_profile_id: "profile-1".to_string(),
            artifacts: vec![artifact(target.clone(), "new")],
        };
        let runner = TransactionRunner::new(root.join("backups"));

        let transaction = runner
            .run_with_tamper_after_backup(&plan, |manifest| {
                let backup_path = manifest.entries[0]
                    .backup_path
                    .as_ref()
                    .expect("backup path");
                fs::write(backup_path, "tampered")
                    .map_err(|error| TransactionError::Io(error.to_string()))
            })
            .expect("run transaction");

        assert_eq!(transaction.phase, TransactionPhase::Failed);
        assert!(transaction.events.iter().any(|event| {
            event.phase == TransactionPhase::Failed
                && event.message.contains("backup content mismatch")
        }));
        assert!(!transaction
            .events
            .iter()
            .any(|event| event.phase == TransactionPhase::Restoring));
        assert_eq!(fs::read_to_string(target).expect("read target"), "old");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn failed_restore_rolls_back_prior_writes() {
        let root = temp_dir("rollback");
        let first = root.join("first.json");
        let second = root.join("second.json");
        fs::write(&first, "old-first").expect("write first");
        fs::write(&second, "old-second").expect("write second");
        let plan = RestorePlan {
            transaction_id: "tx-rollback".to_string(),
            target_profile_id: "profile-1".to_string(),
            artifacts: vec![
                artifact(first.clone(), "new-first"),
                artifact(second.clone(), "new-second"),
            ],
        };
        let runner = TransactionRunner::new(root.join("backups"));

        let transaction = runner
            .run_with_failure_after(&plan, 1)
            .expect("run transaction with injected failure");

        assert_eq!(transaction.phase, TransactionPhase::Failed);
        assert!(transaction
            .events
            .iter()
            .any(|event| event.phase == TransactionPhase::RolledBack));
        assert_eq!(fs::read_to_string(first).expect("read first"), "old-first");
        assert_eq!(
            fs::read_to_string(second).expect("read second"),
            "old-second"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rollback_removes_files_that_did_not_exist_before_switch() {
        let root = temp_dir("remove-created");
        let target = root.join("new-auth.json");
        let plan = RestorePlan {
            transaction_id: "tx-created".to_string(),
            target_profile_id: "profile-1".to_string(),
            artifacts: vec![artifact(target.clone(), "created")],
        };
        let runner = TransactionRunner::new(root.join("backups"));
        let manifest = runner.backup(&plan).expect("backup");
        atomic_write(&target, b"created", None).expect("write target");

        runner.rollback(&manifest).expect("rollback");

        assert!(!target.exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn successful_transaction_refreshes_cache_artifacts() {
        let root = temp_dir("cache-refresh");
        let auth = root.join("auth.json");
        let cache = root.join("Cache/session.bin");
        fs::create_dir_all(cache.parent().expect("cache parent")).expect("create cache dir");
        fs::write(&auth, "old-auth").expect("write auth");
        fs::write(&cache, "old-cache").expect("write cache");
        let plan = RestorePlan {
            transaction_id: "tx-cache-refresh".to_string(),
            target_profile_id: "profile-1".to_string(),
            artifacts: vec![
                artifact(auth.clone(), "new-auth"),
                cache_artifact(cache.clone(), "new-cache"),
            ],
        };
        let runner = TransactionRunner::new(root.join("backups"));

        let transaction = runner.run(&plan).expect("run transaction");

        assert_eq!(transaction.phase, TransactionPhase::Completed);
        assert_eq!(fs::read_to_string(auth).expect("read auth"), "new-auth");
        assert!(!cache.exists());
        assert!(root
            .join("backups/tx-cache-refresh/artifact-1.bak")
            .exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rollback_restores_cache_after_post_restore_failure() {
        let root = temp_dir("cache-rollback");
        let cache = root.join("Cache/session.bin");
        fs::create_dir_all(cache.parent().expect("cache parent")).expect("create cache dir");
        fs::write(&cache, "old-cache").expect("write old cache");
        let plan = RestorePlan {
            transaction_id: "tx-cache-rollback".to_string(),
            target_profile_id: "profile-1".to_string(),
            artifacts: vec![cache_artifact(cache.clone(), "new-cache")],
        };
        let runner = TransactionRunner::new(root.join("backups"));

        let transaction = runner
            .run_with_post_restore(&plan, || {
                Err(TransactionError::PostRestore("restart failed".to_string()))
            })
            .expect("run transaction");

        assert_eq!(transaction.phase, TransactionPhase::Failed);
        assert_eq!(fs::read_to_string(cache).expect("read cache"), "old-cache");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn duplicate_restore_targets_fail_before_backup_or_write() {
        let root = temp_dir("duplicate-target");
        let target = root.join("auth.json");
        fs::write(&target, "old").expect("write old");
        let plan = RestorePlan {
            transaction_id: "tx-duplicate-target".to_string(),
            target_profile_id: "profile-1".to_string(),
            artifacts: vec![
                artifact(target.clone(), "expected-one"),
                artifact(target.clone(), "expected-two"),
            ],
        };
        let runner = TransactionRunner::new(root.join("backups"));

        let transaction = runner.run(&plan).expect("run transaction");

        assert_eq!(transaction.phase, TransactionPhase::Failed);
        assert!(transaction.events.iter().any(|event| {
            event.phase == TransactionPhase::Failed
                && event.message.contains("duplicate restore target")
        }));
        assert!(!transaction
            .events
            .iter()
            .any(|event| event.phase == TransactionPhase::BackingUp));
        assert_eq!(fs::read_to_string(target).expect("read target"), "old");
        assert!(!root.join("backups/tx-duplicate-target").exists());
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn symlink_restore_target_fails_before_backup_or_write() {
        use std::os::unix::fs::symlink;

        let root = temp_dir("symlink-target");
        let real_target = root.join("real-auth.json");
        let symlink_target = root.join("auth-link.json");
        fs::write(&real_target, "old").expect("write real target");
        symlink(&real_target, &symlink_target).expect("create symlink");
        let plan = RestorePlan {
            transaction_id: "tx-symlink-target".to_string(),
            target_profile_id: "profile-1".to_string(),
            artifacts: vec![artifact(symlink_target.clone(), "new")],
        };
        let runner = TransactionRunner::new(root.join("backups"));

        let transaction = runner.run(&plan).expect("run transaction");

        assert_eq!(transaction.phase, TransactionPhase::Failed);
        assert!(transaction.events.iter().any(|event| {
            event.phase == TransactionPhase::Failed
                && event.message.contains("restore target is a symlink")
        }));
        assert!(!transaction
            .events
            .iter()
            .any(|event| event.phase == TransactionPhase::BackingUp));
        assert_eq!(
            fs::read_to_string(real_target).expect("read real target"),
            "old"
        );
        assert!(fs::symlink_metadata(symlink_target)
            .expect("symlink metadata")
            .file_type()
            .is_symlink());
        assert!(!root.join("backups/tx-symlink-target").exists());
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn symlink_restore_target_ancestor_fails_before_backup_or_write() {
        use std::os::unix::fs::symlink;

        let root = temp_dir("symlink-parent");
        let real_parent = root.join("real-parent");
        let symlink_parent = root.join("auth-parent-link");
        fs::create_dir_all(&real_parent).expect("create real parent");
        symlink(&real_parent, &symlink_parent).expect("create parent symlink");
        let target = symlink_parent.join("auth.json");
        let plan = RestorePlan {
            transaction_id: "tx-symlink-parent".to_string(),
            target_profile_id: "profile-1".to_string(),
            artifacts: vec![artifact(target.clone(), "new")],
        };
        let runner = TransactionRunner::new(root.join("backups"));

        let transaction = runner.run(&plan).expect("run transaction");

        assert_eq!(transaction.phase, TransactionPhase::Failed);
        assert!(transaction.events.iter().any(|event| {
            event.phase == TransactionPhase::Failed
                && event
                    .message
                    .contains("restore target ancestor is a symlink")
        }));
        assert!(!transaction
            .events
            .iter()
            .any(|event| event.phase == TransactionPhase::BackingUp));
        assert!(!real_parent.join("auth.json").exists());
        assert!(fs::symlink_metadata(symlink_parent)
            .expect("symlink metadata")
            .file_type()
            .is_symlink());
        assert!(!root.join("backups/tx-symlink-parent").exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn directory_restore_target_fails_before_backup_or_write() {
        let root = temp_dir("directory-target");
        let target = root.join("auth-dir");
        fs::create_dir_all(&target).expect("create target dir");
        let plan = RestorePlan {
            transaction_id: "tx-directory-target".to_string(),
            target_profile_id: "profile-1".to_string(),
            artifacts: vec![artifact(target.clone(), "new")],
        };
        let runner = TransactionRunner::new(root.join("backups"));

        let transaction = runner.run(&plan).expect("run transaction");

        assert_eq!(transaction.phase, TransactionPhase::Failed);
        assert!(transaction.events.iter().any(|event| {
            event.phase == TransactionPhase::Failed
                && event.message.contains("restore target is a directory")
        }));
        assert!(!transaction
            .events
            .iter()
            .any(|event| event.phase == TransactionPhase::BackingUp));
        assert!(target.is_dir());
        assert!(!root.join("backups/tx-directory-target").exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn unsafe_transaction_id_fails_before_backup_or_write() {
        let root = temp_dir("unsafe-transaction-id");
        let target = root.join("auth.json");
        fs::write(&target, "old").expect("write old");
        let plan = RestorePlan {
            transaction_id: "../escape".to_string(),
            target_profile_id: "profile-1".to_string(),
            artifacts: vec![artifact(target.clone(), "new")],
        };
        let runner = TransactionRunner::new(root.join("backups"));

        let transaction = runner.run(&plan).expect("run transaction");

        assert_eq!(transaction.phase, TransactionPhase::Failed);
        assert!(transaction.events.iter().any(|event| {
            event.phase == TransactionPhase::Failed
                && event
                    .message
                    .contains("transaction id is not a safe path segment")
        }));
        assert!(!transaction
            .events
            .iter()
            .any(|event| event.phase == TransactionPhase::BackingUp));
        assert_eq!(fs::read_to_string(target).expect("read target"), "old");
        assert!(!root.join("backups").exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn file_backup_root_fails_before_backup_or_write() {
        let root = temp_dir("file-backup-root");
        let target = root.join("auth.json");
        let backup_root = root.join("backups");
        fs::write(&target, "old").expect("write old");
        fs::write(&backup_root, "not a directory").expect("write backup root file");
        let plan = RestorePlan {
            transaction_id: "tx-file-backup-root".to_string(),
            target_profile_id: "profile-1".to_string(),
            artifacts: vec![artifact(target.clone(), "new")],
        };
        let runner = TransactionRunner::new(backup_root.clone());

        let transaction = runner.run(&plan).expect("run transaction");

        assert_eq!(transaction.phase, TransactionPhase::Failed);
        assert!(transaction.events.iter().any(|event| {
            event.phase == TransactionPhase::Failed
                && event.message.contains("backup root is not a directory")
        }));
        assert!(!transaction
            .events
            .iter()
            .any(|event| event.phase == TransactionPhase::BackingUp));
        assert_eq!(fs::read_to_string(target).expect("read target"), "old");
        assert_eq!(
            fs::read_to_string(backup_root).expect("read backup root"),
            "not a directory"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn symlink_backup_root_fails_before_backup_or_write() {
        use std::os::unix::fs::symlink;

        let root = temp_dir("symlink-backup-root");
        let target = root.join("auth.json");
        let real_backup_root = root.join("real-backups");
        let symlink_backup_root = root.join("backup-link");
        fs::write(&target, "old").expect("write old");
        fs::create_dir_all(&real_backup_root).expect("create real backup root");
        symlink(&real_backup_root, &symlink_backup_root).expect("create backup symlink");
        let plan = RestorePlan {
            transaction_id: "tx-symlink-backup-root".to_string(),
            target_profile_id: "profile-1".to_string(),
            artifacts: vec![artifact(target.clone(), "new")],
        };
        let runner = TransactionRunner::new(symlink_backup_root.clone());

        let transaction = runner.run(&plan).expect("run transaction");

        assert_eq!(transaction.phase, TransactionPhase::Failed);
        assert!(transaction.events.iter().any(|event| {
            event.phase == TransactionPhase::Failed
                && event.message.contains("backup root is a symlink")
        }));
        assert!(!transaction
            .events
            .iter()
            .any(|event| event.phase == TransactionPhase::BackingUp));
        assert_eq!(fs::read_to_string(target).expect("read target"), "old");
        assert!(!real_backup_root.join("tx-symlink-backup-root").exists());
        assert!(fs::symlink_metadata(symlink_backup_root)
            .expect("symlink metadata")
            .file_type()
            .is_symlink());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn missing_restore_target_parent_is_created() {
        let root = temp_dir("missing-parent");
        let target = root.join("missing/auth.json");
        let plan = RestorePlan {
            transaction_id: "tx-missing-parent".to_string(),
            target_profile_id: "profile-1".to_string(),
            artifacts: vec![artifact(target.clone(), "new")],
        };
        let runner = TransactionRunner::new(root.join("backups"));

        let transaction = runner.run(&plan).expect("run transaction");

        assert_eq!(transaction.phase, TransactionPhase::Completed);
        assert_eq!(fs::read_to_string(target).expect("read target"), "new");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn verification_failure_rolls_back_restored_files() {
        let root = temp_dir("verification-rollback");
        let target = root.join("auth.json");
        fs::write(&target, "old").expect("write old");
        let plan = RestorePlan {
            transaction_id: "tx-verification-rollback".to_string(),
            target_profile_id: "profile-1".to_string(),
            artifacts: vec![artifact(target.clone(), "expected")],
        };
        let runner = TransactionRunner::new(root.join("backups"));

        let transaction = runner
            .run_with_tamper_after_restore(&plan, || {
                fs::write(&target, "tampered")
                    .map_err(|error| TransactionError::Io(error.to_string()))
            })
            .expect("run transaction");

        assert_eq!(transaction.phase, TransactionPhase::Failed);
        assert!(transaction.events.iter().any(|event| {
            event.phase == TransactionPhase::Failed
                && event.message.contains("File verification failed")
        }));
        assert_eq!(fs::read_to_string(target).expect("read target"), "old");
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn successful_transaction_restores_unix_mode() {
        use std::os::unix::fs::PermissionsExt;

        let root = temp_dir("mode-restore");
        let target = root.join("auth.json");
        fs::write(&target, "old").expect("write old");
        fs::set_permissions(&target, fs::Permissions::from_mode(0o644)).expect("set old mode");
        let plan = RestorePlan {
            transaction_id: "tx-mode-restore".to_string(),
            target_profile_id: "profile-1".to_string(),
            artifacts: vec![RestoreArtifact {
                environment: "cli".to_string(),
                kind: RestoreArtifactKind::Auth,
                target_path: target.clone(),
                content_base64: STANDARD.encode("new".as_bytes()),
                unix_mode: Some(0o600),
            }],
        };
        let runner = TransactionRunner::new(root.join("backups"));

        let transaction = runner.run(&plan).expect("run transaction");

        assert_eq!(transaction.phase, TransactionPhase::Completed);
        let mode = fs::metadata(&target)
            .expect("metadata")
            .permissions()
            .mode()
            & 0o7777;
        assert_eq!(mode, 0o600);
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn permission_verification_failure_rolls_back_restored_files() {
        use std::os::unix::fs::PermissionsExt;

        let root = temp_dir("mode-rollback");
        let target = root.join("auth.json");
        fs::write(&target, "old").expect("write old");
        let plan = RestorePlan {
            transaction_id: "tx-mode-rollback".to_string(),
            target_profile_id: "profile-1".to_string(),
            artifacts: vec![RestoreArtifact {
                environment: "cli".to_string(),
                kind: RestoreArtifactKind::Auth,
                target_path: target.clone(),
                content_base64: STANDARD.encode("same".as_bytes()),
                unix_mode: Some(0o600),
            }],
        };
        let runner = TransactionRunner::new(root.join("backups"));

        let transaction = runner
            .run_with_tamper_after_restore(&plan, || {
                fs::set_permissions(&target, fs::Permissions::from_mode(0o644))
                    .map_err(|error| TransactionError::Io(error.to_string()))
            })
            .expect("run transaction");

        assert_eq!(transaction.phase, TransactionPhase::Failed);
        assert_eq!(fs::read_to_string(target).expect("read target"), "old");
        assert!(transaction.events.iter().any(|event| {
            event.phase == TransactionPhase::Failed
                && event.message.contains("File verification failed")
        }));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_empty_restore_plan() {
        let root = temp_dir("empty");
        let plan = RestorePlan {
            transaction_id: "tx-empty".to_string(),
            target_profile_id: "profile-1".to_string(),
            artifacts: Vec::new(),
        };
        let runner = TransactionRunner::new(root.clone());

        assert_eq!(runner.run(&plan), Err(TransactionError::EmptyRestorePlan));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn post_restore_failure_rolls_back_restored_files() {
        let root = temp_dir("post-restore");
        let target = root.join("auth.json");
        fs::write(&target, "old").expect("write old");
        let plan = RestorePlan {
            transaction_id: "tx-post-restore".to_string(),
            target_profile_id: "profile-1".to_string(),
            artifacts: vec![artifact(target.clone(), "new")],
        };
        let runner = TransactionRunner::new(root.join("backups"));

        let transaction = runner
            .run_with_post_restore(&plan, || {
                Err(TransactionError::PostRestore("restart failed".to_string()))
            })
            .expect("run transaction");

        assert_eq!(transaction.phase, TransactionPhase::Failed);
        assert!(transaction
            .events
            .iter()
            .any(|event| event.phase == TransactionPhase::RolledBack));
        assert_eq!(fs::read_to_string(target).expect("read target"), "old");
        let _ = fs::remove_dir_all(root);
    }
}
