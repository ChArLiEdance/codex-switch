use crate::{
    profile::{EnvironmentProfileState, ProfileAuthStatus, ProfileMetadata, TargetEnvironment},
    secret_store::{SecretStore, SecretStoreError, SecretVault},
    DiscoveredPath, EnvironmentState, PathKind, SupportState,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

const MAX_FILES_PER_ENVIRONMENT: usize = 256;
const MAX_BYTES_PER_ENVIRONMENT: usize = 256 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileImportRequest {
    pub name: String,
    pub tags: Vec<String>,
    pub note: String,
    pub environments: Vec<TargetEnvironment>,
    pub confirm_same_account: bool,
    pub default_profile: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileImportResult {
    pub profile: ProfileMetadata,
    pub imported_environments: Vec<ImportedEnvironmentSummary>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedEnvironmentSummary {
    pub environment: TargetEnvironment,
    pub artifact_count: usize,
    pub captured_bytes: usize,
    pub skipped_count: usize,
    pub secret_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileImportPreflightRequest {
    pub environments: Vec<TargetEnvironment>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileImportPreflightResult {
    pub environments: Vec<ImportPreflightEnvironment>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreflightEnvironment {
    pub environment: TargetEnvironment,
    pub selected: bool,
    pub scan_available: bool,
    pub support: String,
    pub account_hint: String,
    pub candidate_path_count: usize,
    pub existing_candidate_path_count: usize,
    pub captured_artifact_count: usize,
    pub captured_bytes: usize,
    pub skipped_artifact_count: usize,
    pub skipped_reasons: Vec<SkippedReasonSummary>,
    pub readiness: ImportReadiness,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkippedReasonSummary {
    pub reason: String,
    pub count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportReadiness {
    Ready,
    NotSelected,
    ScanMissing,
    NoReadableArtifacts,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentSnapshot {
    pub schema_version: u32,
    pub environment: TargetEnvironment,
    pub captured_at: String,
    pub artifacts: Vec<CapturedArtifact>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapturedArtifact {
    pub kind: SnapshotPathKind,
    pub source_path: String,
    pub relative_path: String,
    pub content_base64: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unix_mode: Option<u32>,
    pub skipped_reason: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotPathKind {
    Auth,
    Config,
    Cache,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportError {
    EmptyName,
    NoEnvironmentSelected,
    SameAccountConfirmationRequired,
    NoReadableArtifacts,
    SecretStore(String),
    Serialization(String),
}

impl From<SecretStoreError> for ImportError {
    fn from(value: SecretStoreError) -> Self {
        ImportError::SecretStore(format!("{value:?}"))
    }
}

pub(crate) fn import_profile_from_scan<S: SecretStore>(
    request: ProfileImportRequest,
    scan_environments: &[EnvironmentState],
    captured_at: String,
    vault: &SecretVault<S>,
) -> Result<ProfileImportResult, ImportError> {
    let name = request.name.trim();
    if name.is_empty() {
        return Err(ImportError::EmptyName);
    }
    if request.environments.is_empty() {
        return Err(ImportError::NoEnvironmentSelected);
    }
    if request.environments.len() > 1 && !request.confirm_same_account {
        return Err(ImportError::SameAccountConfirmationRequired);
    }

    let profile_id = profile_id_from_name(name, &captured_at);
    let mut warnings = Vec::new();
    let mut summaries = Vec::new();
    let mut environment_states = Vec::new();

    for environment in [
        TargetEnvironment::Cli,
        TargetEnvironment::Vscode,
        TargetEnvironment::Desktop,
    ] {
        if !request.environments.contains(&environment) {
            environment_states.push(EnvironmentProfileState::missing(
                environment,
                "Not selected for this import",
            ));
            continue;
        }

        let Some(scan_environment) = scan_environments
            .iter()
            .find(|candidate| target_matches_scan(environment, candidate.id))
        else {
            environment_states.push(EnvironmentProfileState::missing(
                environment,
                "Environment scan result was unavailable",
            ));
            warnings.push(format!(
                "No scan result available for {}",
                environment.key()
            ));
            continue;
        };

        let snapshot = capture_environment_snapshot(environment, scan_environment, &captured_at);
        if snapshot
            .artifacts
            .iter()
            .all(|artifact| artifact.content_base64.is_none())
        {
            environment_states.push(EnvironmentProfileState::missing(
                environment,
                "No readable auth, config, or cache artifacts were discovered",
            ));
            warnings.push(format!(
                "No readable artifacts were imported for {}",
                environment.key()
            ));
            summaries.push(ImportedEnvironmentSummary {
                environment,
                artifact_count: 0,
                captured_bytes: 0,
                skipped_count: snapshot.artifacts.len(),
                secret_ref: None,
            });
            continue;
        }

        let artifact_count = snapshot
            .artifacts
            .iter()
            .filter(|artifact| artifact.content_base64.is_some())
            .count();
        let skipped_count = snapshot
            .artifacts
            .iter()
            .filter(|artifact| artifact.content_base64.is_none())
            .count();
        let captured_bytes = snapshot
            .artifacts
            .iter()
            .filter_map(|artifact| artifact.content_base64.as_ref())
            .map(|content| content.len())
            .sum();
        let payload = serde_json::to_string(&snapshot)
            .map_err(|error| ImportError::Serialization(error.to_string()))?;
        let envelope = vault.store_profile_payload(&profile_id, environment, &payload)?;
        environment_states.push(EnvironmentProfileState::available(
            environment,
            envelope.key.clone(),
            captured_at.clone(),
        ));
        summaries.push(ImportedEnvironmentSummary {
            environment,
            artifact_count,
            captured_bytes,
            skipped_count,
            secret_ref: Some(envelope.key),
        });
    }

    let profile = ProfileMetadata {
        id: profile_id,
        name: name.to_string(),
        account_hint: account_hint_from_scan(scan_environments),
        tags: request
            .tags
            .into_iter()
            .map(|tag| tag.trim().to_string())
            .filter(|tag| !tag.is_empty())
            .collect(),
        note: request.note,
        default_profile: request.default_profile,
        last_used_at: None,
        environments: environment_states,
    };

    profile
        .validate()
        .map_err(|error| ImportError::Serialization(format!("{error:?}")))?;
    if !profile
        .environments
        .iter()
        .any(|state| state.status == ProfileAuthStatus::Available)
    {
        return Err(ImportError::NoReadableArtifacts);
    }

    Ok(ProfileImportResult {
        profile,
        imported_environments: summaries,
        warnings,
    })
}

pub(crate) fn import_preflight_from_scan(
    request: ProfileImportPreflightRequest,
    scan_environments: &[EnvironmentState],
) -> ProfileImportPreflightResult {
    let mut warnings = Vec::new();
    let environments = [
        TargetEnvironment::Cli,
        TargetEnvironment::Vscode,
        TargetEnvironment::Desktop,
    ]
    .into_iter()
    .map(|environment| {
        let selected = request.environments.contains(&environment);
        let scan_environment = scan_environments
            .iter()
            .find(|candidate| target_matches_scan(environment, candidate.id));
        match (selected, scan_environment) {
            (false, scan_environment) => preflight_unselected(environment, scan_environment),
            (true, None) => {
                warnings.push(format!(
                    "No scan result available for {}",
                    environment.key()
                ));
                ImportPreflightEnvironment {
                    environment,
                    selected,
                    scan_available: false,
                    support: "not-detected".to_string(),
                    account_hint: "Unknown".to_string(),
                    candidate_path_count: 0,
                    existing_candidate_path_count: 0,
                    captured_artifact_count: 0,
                    captured_bytes: 0,
                    skipped_artifact_count: 0,
                    skipped_reasons: Vec::new(),
                    readiness: ImportReadiness::ScanMissing,
                }
            }
            (true, Some(scan_environment)) => {
                let snapshot =
                    capture_environment_snapshot(environment, scan_environment, "preflight");
                let captured_artifact_count = captured_artifact_count(&snapshot);
                let skipped_artifact_count = skipped_artifact_count(&snapshot);
                let readiness = if captured_artifact_count == 0 {
                    warnings.push(format!(
                        "{} has no readable auth, config, or cache artifacts in the current scan",
                        environment.key()
                    ));
                    ImportReadiness::NoReadableArtifacts
                } else {
                    ImportReadiness::Ready
                };
                ImportPreflightEnvironment {
                    environment,
                    selected,
                    scan_available: true,
                    support: support_key(scan_environment.support).to_string(),
                    account_hint: scan_environment.account_hint.clone(),
                    candidate_path_count: import_candidate_path_count(scan_environment),
                    existing_candidate_path_count: existing_import_candidate_path_count(
                        scan_environment,
                    ),
                    captured_artifact_count,
                    captured_bytes: captured_bytes(&snapshot),
                    skipped_artifact_count,
                    skipped_reasons: skipped_reason_summaries(&snapshot),
                    readiness,
                }
            }
        }
    })
    .collect();

    ProfileImportPreflightResult {
        environments,
        warnings,
    }
}

fn capture_environment_snapshot(
    environment: TargetEnvironment,
    scan_environment: &EnvironmentState,
    captured_at: &str,
) -> EnvironmentSnapshot {
    let mut artifacts = Vec::new();
    let mut budget = CaptureBudget {
        files_remaining: MAX_FILES_PER_ENVIRONMENT,
        bytes_remaining: MAX_BYTES_PER_ENVIRONMENT,
    };

    for path in scan_environment
        .discovered_paths
        .iter()
        .filter(|path| path.exists && snapshot_kind(path.kind).is_some())
    {
        let root = PathBuf::from(&path.path);
        capture_path(path, &root, &root, &mut budget, &mut artifacts);
    }

    EnvironmentSnapshot {
        schema_version: 1,
        environment,
        captured_at: captured_at.to_string(),
        artifacts,
    }
}

fn preflight_unselected(
    environment: TargetEnvironment,
    scan_environment: Option<&EnvironmentState>,
) -> ImportPreflightEnvironment {
    ImportPreflightEnvironment {
        environment,
        selected: false,
        scan_available: scan_environment.is_some(),
        support: scan_environment
            .map(|environment| support_key(environment.support).to_string())
            .unwrap_or_else(|| "not-detected".to_string()),
        account_hint: scan_environment
            .map(|environment| environment.account_hint.clone())
            .unwrap_or_else(|| "Unknown".to_string()),
        candidate_path_count: scan_environment
            .map(import_candidate_path_count)
            .unwrap_or(0),
        existing_candidate_path_count: scan_environment
            .map(existing_import_candidate_path_count)
            .unwrap_or(0),
        captured_artifact_count: 0,
        captured_bytes: 0,
        skipped_artifact_count: 0,
        skipped_reasons: Vec::new(),
        readiness: ImportReadiness::NotSelected,
    }
}

fn captured_artifact_count(snapshot: &EnvironmentSnapshot) -> usize {
    snapshot
        .artifacts
        .iter()
        .filter(|artifact| artifact.content_base64.is_some())
        .count()
}

fn skipped_artifact_count(snapshot: &EnvironmentSnapshot) -> usize {
    snapshot
        .artifacts
        .iter()
        .filter(|artifact| artifact.content_base64.is_none())
        .count()
}

fn captured_bytes(snapshot: &EnvironmentSnapshot) -> usize {
    snapshot
        .artifacts
        .iter()
        .filter_map(|artifact| artifact.content_base64.as_ref())
        .map(|content| content.len())
        .sum()
}

fn skipped_reason_summaries(snapshot: &EnvironmentSnapshot) -> Vec<SkippedReasonSummary> {
    let mut summaries: Vec<SkippedReasonSummary> = Vec::new();
    for reason in snapshot
        .artifacts
        .iter()
        .filter_map(|artifact| artifact.skipped_reason.as_ref())
    {
        if let Some(summary) = summaries
            .iter_mut()
            .find(|summary| summary.reason == *reason)
        {
            summary.count += 1;
        } else {
            summaries.push(SkippedReasonSummary {
                reason: reason.clone(),
                count: 1,
            });
        }
    }
    summaries.sort_by(|left, right| left.reason.cmp(&right.reason));
    summaries
}

fn import_candidate_path_count(scan_environment: &EnvironmentState) -> usize {
    scan_environment
        .discovered_paths
        .iter()
        .filter(|path| snapshot_kind(path.kind).is_some())
        .count()
}

fn existing_import_candidate_path_count(scan_environment: &EnvironmentState) -> usize {
    scan_environment
        .discovered_paths
        .iter()
        .filter(|path| path.exists && snapshot_kind(path.kind).is_some())
        .count()
}

fn support_key(support: SupportState) -> &'static str {
    match support {
        SupportState::Detected => "detected",
        SupportState::Partial => "partial",
        SupportState::NotDetected => "not-detected",
    }
}

fn capture_path(
    discovered: &DiscoveredPath,
    root: &Path,
    current: &Path,
    budget: &mut CaptureBudget,
    artifacts: &mut Vec<CapturedArtifact>,
) {
    let Some(kind) = snapshot_kind(discovered.kind) else {
        return;
    };
    let Ok(metadata) = fs::symlink_metadata(current) else {
        artifacts.push(skipped_artifact(
            kind,
            current,
            root,
            "Unable to read metadata",
        ));
        return;
    };
    if metadata.file_type().is_symlink() {
        artifacts.push(skipped_artifact(kind, current, root, "Symlink skipped"));
        return;
    }
    if metadata.is_dir() {
        let Ok(entries) = fs::read_dir(current) else {
            artifacts.push(skipped_artifact(
                kind,
                current,
                root,
                "Unable to read directory",
            ));
            return;
        };
        for entry in entries.filter_map(Result::ok) {
            capture_path(discovered, root, &entry.path(), budget, artifacts);
        }
        return;
    }
    if !metadata.is_file() {
        artifacts.push(skipped_artifact(
            kind,
            current,
            root,
            "Unsupported filesystem entry",
        ));
        return;
    }
    if budget.files_remaining == 0 {
        artifacts.push(skipped_artifact(kind, current, root, "File limit reached"));
        return;
    }
    let size = metadata.len() as usize;
    if size > budget.bytes_remaining {
        artifacts.push(skipped_artifact(kind, current, root, "Byte limit reached"));
        return;
    }
    match fs::read(current) {
        Ok(bytes) => {
            budget.files_remaining -= 1;
            budget.bytes_remaining = budget.bytes_remaining.saturating_sub(bytes.len());
            artifacts.push(CapturedArtifact {
                kind,
                source_path: current.to_string_lossy().to_string(),
                relative_path: relative_path(root, current),
                content_base64: Some(STANDARD.encode(bytes)),
                unix_mode: file_unix_mode(&metadata),
                skipped_reason: None,
            });
        }
        Err(error) => artifacts.push(skipped_artifact(
            kind,
            current,
            root,
            &format!("Unable to read file: {}", error.kind()),
        )),
    }
}

fn skipped_artifact(
    kind: SnapshotPathKind,
    path: &Path,
    root: &Path,
    reason: &str,
) -> CapturedArtifact {
    CapturedArtifact {
        kind,
        source_path: path.to_string_lossy().to_string(),
        relative_path: relative_path(root, path),
        content_base64: None,
        unix_mode: None,
        skipped_reason: Some(reason.to_string()),
    }
}

#[cfg(unix)]
fn file_unix_mode(metadata: &fs::Metadata) -> Option<u32> {
    use std::os::unix::fs::PermissionsExt;

    Some(metadata.permissions().mode() & 0o7777)
}

#[cfg(not(unix))]
fn file_unix_mode(_metadata: &fs::Metadata) -> Option<u32> {
    None
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .ok()
        .filter(|relative| !relative.as_os_str().is_empty())
        .unwrap_or_else(|| path.file_name().map(Path::new).unwrap_or(path))
        .to_string_lossy()
        .to_string()
}

fn snapshot_kind(kind: PathKind) -> Option<SnapshotPathKind> {
    match kind {
        PathKind::Auth => Some(SnapshotPathKind::Auth),
        PathKind::Config => Some(SnapshotPathKind::Config),
        PathKind::Cache => Some(SnapshotPathKind::Cache),
        PathKind::App => None,
    }
}

fn target_matches_scan(environment: TargetEnvironment, scan_id: &str) -> bool {
    matches!(
        (environment, scan_id),
        (TargetEnvironment::Cli, "CLI")
            | (TargetEnvironment::Vscode, "VS Code")
            | (TargetEnvironment::Desktop, "Desktop")
    )
}

fn account_hint_from_scan(scan_environments: &[EnvironmentState]) -> String {
    scan_environments
        .iter()
        .map(|environment| environment.account_hint.trim())
        .find(|hint| !hint.is_empty() && *hint != "Unknown")
        .unwrap_or("Unknown")
        .to_string()
}

fn profile_id_from_name(name: &str, captured_at: &str) -> String {
    let slug: String = name
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    format!(
        "profile-{}-{}",
        captured_at,
        if slug.is_empty() { "imported" } else { &slug }
    )
}

struct CaptureBudget {
    files_remaining: usize,
    bytes_remaining: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secret_store::{MemorySecretStore, SecretVault};

    fn temp_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "codex-switch-importer-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create temp dir");
        path
    }

    fn scan_state(root: &Path) -> EnvironmentState {
        EnvironmentState {
            id: "CLI",
            installed: true,
            executable_path: Some("/usr/local/bin/codex".to_string()),
            discovered_paths: vec![DiscoveredPath {
                kind: PathKind::Auth,
                path: root.to_string_lossy().to_string(),
                exists: true,
                permission: crate::PermissionState::ReadWrite,
            }],
            running: false,
            running_processes: Vec::new(),
            permission: crate::PermissionState::ReadWrite,
            account_hint: "u***@example.com".to_string(),
            support: crate::SupportState::Detected,
            status_message: "test".to_string(),
        }
    }

    #[test]
    fn requires_confirmation_for_multi_environment_import() {
        let vault = SecretVault::new(MemorySecretStore::default());
        let request = ProfileImportRequest {
            name: "Merged".to_string(),
            tags: Vec::new(),
            note: String::new(),
            environments: vec![TargetEnvironment::Cli, TargetEnvironment::Vscode],
            confirm_same_account: false,
            default_profile: false,
        };

        let error = import_profile_from_scan(request, &[], "1000".to_string(), &vault)
            .expect_err("multi-environment import should require confirmation");
        assert_eq!(error, ImportError::SameAccountConfirmationRequired);
    }

    #[test]
    fn imports_readable_artifacts_into_secret_vault_only() {
        let root = temp_dir("capture");
        fs::write(root.join("auth.json"), "{\"access_token\":\"secret\"}").expect("write auth");
        let vault = SecretVault::new(MemorySecretStore::default());
        let request = ProfileImportRequest {
            name: "Work".to_string(),
            tags: vec!["cli".to_string()],
            note: "Imported".to_string(),
            environments: vec![TargetEnvironment::Cli],
            confirm_same_account: true,
            default_profile: true,
        };

        let result =
            import_profile_from_scan(request, &[scan_state(&root)], "1000".to_string(), &vault)
                .expect("import profile");

        assert_eq!(result.imported_environments[0].artifact_count, 1);
        assert!(!format!("{:?}", result.profile).contains("access_token"));
        let payload = vault
            .load_profile_payload(&result.profile.id, TargetEnvironment::Cli)
            .expect("load secret")
            .expect("secret payload");
        assert!(!payload.contains("access_token"));
        let snapshot: EnvironmentSnapshot =
            serde_json::from_str(&payload).expect("decode snapshot");
        let content = snapshot.artifacts[0]
            .content_base64
            .as_ref()
            .expect("captured content");
        let decoded = STANDARD.decode(content).expect("decode content");
        assert_eq!(
            String::from_utf8(decoded).expect("utf8"),
            "{\"access_token\":\"secret\"}"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn import_preflight_reports_selected_capture_coverage() {
        let root = temp_dir("preflight");
        fs::write(root.join("auth.json"), "secret-free-test").expect("write auth");
        let request = ProfileImportPreflightRequest {
            environments: vec![TargetEnvironment::Cli],
        };

        let result = import_preflight_from_scan(request, &[scan_state(&root)]);
        let cli = result
            .environments
            .iter()
            .find(|environment| environment.environment == TargetEnvironment::Cli)
            .expect("cli preflight");
        let vscode = result
            .environments
            .iter()
            .find(|environment| environment.environment == TargetEnvironment::Vscode)
            .expect("vscode preflight");

        assert_eq!(cli.readiness, ImportReadiness::Ready);
        assert_eq!(cli.candidate_path_count, 1);
        assert_eq!(cli.existing_candidate_path_count, 1);
        assert_eq!(cli.captured_artifact_count, 1);
        assert!(cli.captured_bytes > 0);
        assert_eq!(vscode.readiness, ImportReadiness::NotSelected);
        assert_eq!(vscode.captured_artifact_count, 0);
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn import_preflight_reports_skip_reasons() {
        use std::os::unix::fs::symlink;

        let root = temp_dir("preflight-skips");
        let real_auth = root.join("auth.json");
        fs::write(&real_auth, "secret-free-test").expect("write auth");
        symlink(&real_auth, root.join("auth-link.json")).expect("create symlink");
        let request = ProfileImportPreflightRequest {
            environments: vec![TargetEnvironment::Cli],
        };

        let result = import_preflight_from_scan(request, &[scan_state(&root)]);
        let cli = result
            .environments
            .iter()
            .find(|environment| environment.environment == TargetEnvironment::Cli)
            .expect("cli preflight");

        assert_eq!(cli.readiness, ImportReadiness::Ready);
        assert_eq!(cli.captured_artifact_count, 1);
        assert_eq!(cli.skipped_artifact_count, 1);
        assert!(cli
            .skipped_reasons
            .iter()
            .any(|summary| summary.reason == "Symlink skipped" && summary.count == 1));
        let _ = fs::remove_dir_all(root);
    }
}
