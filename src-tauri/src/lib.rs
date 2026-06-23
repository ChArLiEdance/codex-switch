pub mod account_hint;
pub mod app_state;
pub mod cli_app;
pub mod desktop_app;
pub mod importer;
pub mod profile;
pub mod profile_store;
pub mod profile_switch;
pub mod secret_store;
pub mod switch_transaction;
pub mod vscode_app;

use account_hint::{redact_email_like_text, redacted_account_hint_from_path};
use app_state::{
    default_app_state_dir, load_recovery_status, AppSettings, AppStateRepository,
    EnvironmentPathOverride, RecoveryStatus, SwitchHistoryEntry,
};
use importer::{
    import_preflight_from_scan, import_profile_from_scan, ProfileImportPreflightRequest,
    ProfileImportPreflightResult, ProfileImportRequest, ProfileImportResult,
};
use profile::{ProfileMetadata, TargetEnvironment};
use profile_store::{ProfileRepository, ProfileStoreError, ProfileUpdateRequest};
use profile_switch::{
    restore_default_on_exit_with_runtime, retry_restart_desktop, retry_restart_vscode,
    switch_saved_profile, ProfileSwitchRequest, ProfileSwitchResult, RestartAppRequest,
    RestartAppResult, RestoreDefaultOnExitResult,
};
use secret_store::{KeychainSecretStore, SecretVault};
use serde::{Deserialize, Serialize};
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EnvironmentScan {
    pub(crate) os: &'static str,
    pub(crate) scanned_at: String,
    pub(crate) read_only: bool,
    pub(crate) environments: Vec<EnvironmentState>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EnvironmentDiagnosticsReport {
    pub(crate) schema_version: &'static str,
    pub(crate) generated_at: String,
    pub(crate) os: &'static str,
    pub(crate) read_only: bool,
    pub(crate) environments: Vec<EnvironmentDiagnosticsEntry>,
    pub(crate) notes: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EnvironmentDiagnosticsEntry {
    pub(crate) id: &'static str,
    pub(crate) installed: bool,
    pub(crate) executable_path: Option<String>,
    pub(crate) running: bool,
    pub(crate) running_processes: Vec<String>,
    pub(crate) permission: PermissionState,
    pub(crate) account_hint: String,
    pub(crate) support: SupportState,
    pub(crate) status_message: String,
    pub(crate) discovered_paths: Vec<DiagnosticPathSummary>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DiagnosticPathSummary {
    pub(crate) kind: PathKind,
    pub(crate) path: String,
    pub(crate) exists: bool,
    pub(crate) permission: PermissionState,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EnvironmentState {
    pub(crate) id: &'static str,
    pub(crate) installed: bool,
    pub(crate) executable_path: Option<String>,
    pub(crate) discovered_paths: Vec<DiscoveredPath>,
    pub(crate) running: bool,
    pub(crate) running_processes: Vec<String>,
    pub(crate) permission: PermissionState,
    pub(crate) account_hint: String,
    pub(crate) support: SupportState,
    pub(crate) status_message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DiscoveredPath {
    pub(crate) kind: PathKind,
    pub(crate) path: String,
    pub(crate) exists: bool,
    pub(crate) permission: PermissionState,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum PermissionState {
    ReadWrite,
    ReadOnly,
    Missing,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum SupportState {
    Detected,
    Partial,
    NotDetected,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum PathKind {
    App,
    Auth,
    Config,
    Cache,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProfileDeleteRequest {
    profile_id: String,
}

#[tauri::command]
fn backend_health() -> &'static str {
    "codex_switch_backend_ready"
}

#[tauri::command]
fn detect_environments() -> EnvironmentScan {
    let processes = running_processes();
    let settings = app_state_repository().load_settings().unwrap_or_default();
    let environments = vec![
        detect_cli(&processes, &settings.custom_paths),
        detect_vscode(&processes, &settings.custom_paths),
        detect_desktop(&processes, &settings.custom_paths),
    ];

    EnvironmentScan {
        os: env::consts::OS,
        scanned_at: unix_timestamp_string(),
        read_only: true,
        environments,
    }
}

#[tauri::command]
fn environment_diagnostics_report() -> EnvironmentDiagnosticsReport {
    diagnostics_report_from_scan(detect_environments())
}

#[tauri::command]
fn list_profiles() -> Result<Vec<ProfileMetadata>, String> {
    profile_repository()
        .list_profiles()
        .map_err(profile_store_error_message)
}

#[tauri::command]
fn import_current_profile(request: ProfileImportRequest) -> Result<ProfileImportResult, String> {
    let scan = detect_environments();
    let captured_at = scan.scanned_at.clone();
    let vault = SecretVault::new(KeychainSecretStore::new());
    let result = import_profile_from_scan(request, &scan.environments, captured_at, &vault)
        .map_err(|error| format!("{error:?}"))?;
    profile_repository()
        .upsert_profile(result.profile.clone())
        .map_err(profile_store_error_message)?;
    Ok(result)
}

#[tauri::command]
fn preview_current_import(
    request: ProfileImportPreflightRequest,
) -> Result<ProfileImportPreflightResult, String> {
    let scan = detect_environments();
    Ok(import_preflight_from_scan(request, &scan.environments))
}

#[tauri::command]
fn update_profile(request: ProfileUpdateRequest) -> Result<ProfileMetadata, String> {
    profile_repository()
        .update_profile(request)
        .map_err(profile_store_error_message)
}

#[tauri::command]
fn delete_profile(request: ProfileDeleteRequest) -> Result<(), String> {
    let repository = profile_repository();
    let profile = repository
        .list_profiles()
        .map_err(profile_store_error_message)?
        .into_iter()
        .find(|profile| profile.id == request.profile_id)
        .ok_or_else(|| {
            profile_store_error_message(ProfileStoreError::NotFound(request.profile_id.clone()))
        })?;
    let vault = SecretVault::new(KeychainSecretStore::new());
    for environment in [
        TargetEnvironment::Cli,
        TargetEnvironment::Vscode,
        TargetEnvironment::Desktop,
    ] {
        vault
            .delete_profile_payload(&profile.id, environment)
            .map_err(|error| format!("{error:?}"))?;
    }
    repository
        .delete_profile(&profile.id)
        .map_err(profile_store_error_message)?;
    Ok(())
}

#[tauri::command]
fn get_settings() -> Result<AppSettings, String> {
    app_state_repository()
        .load_settings()
        .map_err(|error| format!("{error:?}"))
}

#[tauri::command]
fn save_settings(settings: AppSettings) -> Result<AppSettings, String> {
    let repository = app_state_repository();
    repository
        .save_settings(&settings)
        .map_err(|error| format!("{error:?}"))?;
    Ok(settings)
}

#[tauri::command]
fn list_switch_history() -> Result<Vec<SwitchHistoryEntry>, String> {
    app_state_repository()
        .list_history()
        .map_err(|error| format!("{error:?}"))
}

#[tauri::command]
fn clear_switch_history() -> Result<(), String> {
    app_state_repository()
        .clear_history()
        .map_err(|error| format!("{error:?}"))
}

#[tauri::command]
fn check_recovery_status() -> Result<RecoveryStatus, String> {
    load_recovery_status(&app_state_repository()).map_err(|error| format!("{error:?}"))
}

#[tauri::command]
fn resolve_recovery_status() -> Result<RecoveryStatus, String> {
    app_state_repository()
        .resolve_unfinished_transaction()
        .map_err(|error| format!("{error:?}"))
}

#[tauri::command]
fn switch_to_profile(request: ProfileSwitchRequest) -> Result<ProfileSwitchResult, String> {
    let profile_repository = profile_repository();
    let app_state_repository = app_state_repository();
    let vault = SecretVault::new(KeychainSecretStore::new());
    switch_saved_profile(
        request,
        &profile_repository,
        &app_state_repository,
        &vault,
        unix_timestamp_string(),
    )
    .map_err(|error| format!("{error:?}"))
}

#[tauri::command]
fn restore_default_on_exit() -> Result<RestoreDefaultOnExitResult, String> {
    let profile_repository = profile_repository();
    let app_state_repository = app_state_repository();
    let vault = SecretVault::new(KeychainSecretStore::new());
    let runtime = profile_switch::SystemProfileSwitchRuntime::default();
    restore_default_on_exit_with_runtime(
        &profile_repository,
        &app_state_repository,
        &vault,
        unix_timestamp_string(),
        &runtime,
    )
    .map_err(|error| format!("{error:?}"))
}

#[tauri::command]
fn restart_desktop_app(request: RestartAppRequest) -> Result<RestartAppResult, String> {
    let runtime = profile_switch::SystemProfileSwitchRuntime::default();
    retry_restart_desktop(&runtime, request.app_path.as_deref())
        .map_err(|error| format!("{error:?}"))
}

#[tauri::command]
fn restart_vscode_app(request: RestartAppRequest) -> Result<RestartAppResult, String> {
    let runtime = profile_switch::SystemProfileSwitchRuntime::default();
    retry_restart_vscode(&runtime, request.app_path.as_deref())
        .map_err(|error| format!("{error:?}"))
}

fn detect_cli(processes: &[String], custom_paths: &[EnvironmentPathOverride]) -> EnvironmentState {
    let executable_path = find_executable("codex");
    let mut discovered_paths = Vec::new();

    if let Ok(path) = env::var("CODEX_HOME") {
        discovered_paths.push(discovered_path(PathKind::Config, PathBuf::from(path)));
    }
    if let Some(home) = home_dir() {
        discovered_paths.push(discovered_path(PathKind::Config, home.join(".codex")));
        discovered_paths.push(discovered_path(
            PathKind::Auth,
            home.join(".codex").join("auth.json"),
        ));
        discovered_paths.push(discovered_path(
            PathKind::Cache,
            home.join(".codex").join("cache"),
        ));
    }
    append_custom_paths(
        &mut discovered_paths,
        custom_paths,
        TargetEnvironment::Cli,
    );

    let running_processes = matching_processes(processes, &["codex"]);
    environment_state(
        "CLI",
        executable_path,
        discovered_paths,
        running_processes,
        "Codex CLI executable and candidate local state paths discovered read-only",
    )
}

fn detect_vscode(
    processes: &[String],
    custom_paths: &[EnvironmentPathOverride],
) -> EnvironmentState {
    let executable_path = find_executable("code").or_else(find_vscode_app);
    let mut discovered_paths = Vec::new();

    for (kind, path) in vscode_candidate_paths() {
        push_discovered_path(&mut discovered_paths, kind, path.clone());
        if path.exists() {
            for child in children_matching(&path, &["codex", "openai"]) {
                push_discovered_path(&mut discovered_paths, PathKind::Auth, child);
            }
        }
    }
    append_custom_paths(
        &mut discovered_paths,
        custom_paths,
        TargetEnvironment::Vscode,
    );

    let running_processes = matching_processes(processes, &["code", "visual studio code"]);
    environment_state(
        "VS Code",
        executable_path,
        discovered_paths,
        running_processes,
        "VS Code app and extension storage candidates discovered read-only",
    )
}

fn detect_desktop(
    processes: &[String],
    custom_paths: &[EnvironmentPathOverride],
) -> EnvironmentState {
    let executable_path = find_desktop_app();
    let mut discovered_paths = Vec::new();

    for (kind, path) in desktop_candidate_paths() {
        push_discovered_path(&mut discovered_paths, kind, path.clone());
        if path.exists() {
            for child in children_matching(&path, &["codex", "openai"]) {
                push_discovered_path(&mut discovered_paths, PathKind::Auth, child);
            }
        }
    }
    append_custom_paths(
        &mut discovered_paths,
        custom_paths,
        TargetEnvironment::Desktop,
    );

    let running_processes = matching_processes(processes, &["codex", "codex desktop"]);
    environment_state(
        "Desktop",
        executable_path,
        discovered_paths,
        running_processes,
        "Codex Desktop app and support-directory candidates discovered read-only",
    )
}

fn environment_state(
    id: &'static str,
    executable_path: Option<String>,
    mut discovered_paths: Vec<DiscoveredPath>,
    running_processes: Vec<String>,
    detected_message: &str,
) -> EnvironmentState {
    let installed = executable_path.is_some();
    if let Some(path) = &executable_path {
        discovered_paths.insert(0, discovered_path(PathKind::App, PathBuf::from(path)));
    }
    let permission = summarize_permissions(&discovered_paths);
    let account_hint = account_hint_from_paths(&discovered_paths);
    let support = if installed {
        SupportState::Detected
    } else if discovered_paths.iter().any(|path| path.exists) || !running_processes.is_empty() {
        SupportState::Partial
    } else {
        SupportState::NotDetected
    };
    let status_message = if installed {
        detected_message.to_string()
    } else if matches!(support, SupportState::Partial) {
        "Partial evidence found; installation path was not confirmed".to_string()
    } else {
        "No confirmed installation or local state path found".to_string()
    };

    EnvironmentState {
        id,
        installed,
        executable_path,
        discovered_paths,
        running: !running_processes.is_empty(),
        running_processes,
        permission,
        account_hint,
        support,
        status_message,
    }
}

fn account_hint_from_paths(paths: &[DiscoveredPath]) -> String {
    for path in paths
        .iter()
        .filter(|path| path.exists && matches!(path.kind, PathKind::Auth | PathKind::Config))
    {
        if let Some(hint) = redacted_account_hint_from_path(&PathBuf::from(&path.path)) {
            return hint;
        }
    }
    "Unknown".to_string()
}

fn discovered_path(kind: PathKind, path: PathBuf) -> DiscoveredPath {
    let exists = path.exists();
    DiscoveredPath {
        kind,
        path: path.to_string_lossy().to_string(),
        exists,
        permission: permission_for_path(&path),
    }
}

fn push_discovered_path(paths: &mut Vec<DiscoveredPath>, kind: PathKind, path: PathBuf) {
    let path_text = path.to_string_lossy().to_string();
    if paths
        .iter()
        .any(|candidate| candidate.kind == kind && candidate.path == path_text)
    {
        return;
    }
    paths.push(discovered_path(kind, path));
}

fn append_custom_paths(
    paths: &mut Vec<DiscoveredPath>,
    custom_paths: &[EnvironmentPathOverride],
    environment: TargetEnvironment,
) {
    for custom_path in custom_paths
        .iter()
        .filter(|custom_path| custom_path.environment == environment)
    {
        let path = custom_path.path.trim();
        if path.is_empty() {
            continue;
        }
        push_discovered_path(paths, custom_path.kind, expand_user_path(path));
    }
}

fn expand_user_path(path: &str) -> PathBuf {
    if path == "~" {
        if let Some(home) = home_dir() {
            return home;
        }
    } else if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(path)
}

fn permission_for_path(path: &Path) -> PermissionState {
    match fs::metadata(path) {
        Ok(metadata) => {
            if metadata.permissions().readonly() {
                PermissionState::ReadOnly
            } else {
                PermissionState::ReadWrite
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => PermissionState::Missing,
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            PermissionState::ReadOnly
        }
        Err(_) => PermissionState::Unknown,
    }
}

fn summarize_permissions(paths: &[DiscoveredPath]) -> PermissionState {
    let existing: Vec<_> = paths.iter().filter(|path| path.exists).collect();
    if existing.is_empty() {
        return PermissionState::Unknown;
    }
    if existing
        .iter()
        .all(|path| path.permission == PermissionState::ReadWrite)
    {
        PermissionState::ReadWrite
    } else if existing
        .iter()
        .any(|path| path.permission == PermissionState::ReadOnly)
    {
        PermissionState::ReadOnly
    } else {
        PermissionState::Unknown
    }
}

fn find_executable(name: &str) -> Option<String> {
    let paths = env::var_os("PATH")?;
    let candidates = env::split_paths(&paths);

    #[cfg(windows)]
    let extensions: Vec<String> = env::var("PATHEXT")
        .unwrap_or_else(|_| ".EXE;.BAT;.CMD".to_string())
        .split(';')
        .map(|value| value.to_ascii_lowercase())
        .collect();
    #[cfg(not(windows))]
    let extensions: Vec<String> = vec!["".to_string()];

    for directory in candidates {
        for extension in &extensions {
            let candidate = directory.join(format!("{name}{extension}"));
            if candidate.is_file() {
                return Some(candidate.to_string_lossy().to_string());
            }
        }
    }
    None
}

fn find_vscode_app() -> Option<String> {
    first_existing(&[
        "/Applications/Visual Studio Code.app",
        "/Applications/Visual Studio Code - Insiders.app",
    ])
}

fn find_desktop_app() -> Option<String> {
    first_existing(&[
        "/Applications/Codex.app",
        "/Applications/Codex Desktop.app",
        "/Applications/OpenAI Codex.app",
    ])
}

fn first_existing(paths: &[&str]) -> Option<String> {
    paths
        .iter()
        .map(PathBuf::from)
        .find(|path| path.exists())
        .map(|path| path.to_string_lossy().to_string())
}

fn vscode_candidate_paths() -> Vec<(PathKind, PathBuf)> {
    let mut paths = Vec::new();
    if let Some(home) = home_dir() {
        paths.extend(vscode_candidate_paths_for_home(&home));
    }
    if let Ok(appdata) = env::var("APPDATA") {
        let appdata = PathBuf::from(appdata);
        for product in ["Code", "Code - Insiders"] {
            let storage = appdata.join(product).join("User/globalStorage");
            paths.push((PathKind::Auth, storage.join("openai.chatgpt")));
            paths.push((PathKind::Auth, storage.join("openai.codex")));
            paths.push((PathKind::Config, storage));
        }
    }
    paths
}

fn vscode_candidate_paths_for_home(home: &Path) -> Vec<(PathKind, PathBuf)> {
    let mut paths = Vec::new();
    for product in ["Code", "Code - Insiders"] {
        let storage = home
            .join("Library/Application Support")
            .join(product)
            .join("User/globalStorage");
        paths.push((PathKind::Auth, storage.join("openai.chatgpt")));
        paths.push((PathKind::Auth, storage.join("openai.codex")));
        paths.push((PathKind::Config, storage));
    }
    for product in ["Code", "Code - Insiders"] {
        let storage = home.join(".config").join(product).join("User/globalStorage");
        paths.push((PathKind::Auth, storage.join("openai.chatgpt")));
        paths.push((PathKind::Auth, storage.join("openai.codex")));
        paths.push((PathKind::Config, storage));
    }
    paths
}

fn desktop_candidate_paths() -> Vec<(PathKind, PathBuf)> {
    let mut paths = Vec::new();
    if let Some(home) = home_dir() {
        paths.extend(desktop_candidate_paths_for_home(&home));
    }
    if let Ok(appdata) = env::var("APPDATA") {
        let appdata = PathBuf::from(appdata);
        for product in ["Codex", "Codex Desktop", "OpenAI/Codex"] {
            let root = appdata.join(product);
            paths.push((PathKind::Auth, root.join("Default/Local Storage")));
            paths.push((PathKind::Auth, root.join("Default/Session Storage")));
            paths.push((PathKind::Auth, root.join("Default/Network")));
            paths.push((PathKind::Cache, root.join("Cache")));
            paths.push((PathKind::Config, root));
        }
    }
    paths
}

fn desktop_candidate_paths_for_home(home: &Path) -> Vec<(PathKind, PathBuf)> {
    let support = home.join("Library/Application Support");
    let mut paths = Vec::new();
    for product in ["Codex", "Codex Desktop", "OpenAI/Codex"] {
        let root = support.join(product);
        paths.push((PathKind::Auth, root.join("Default/Local Storage")));
        paths.push((PathKind::Auth, root.join("Default/Session Storage")));
        paths.push((PathKind::Auth, root.join("Default/Network")));
        paths.push((PathKind::Auth, root.join("Default/Cookies")));
        paths.push((PathKind::Auth, root.join("Partitions/codex-browser-app")));
        paths.push((PathKind::Cache, root.join("Cache")));
        paths.push((PathKind::Cache, root.join("Code Cache")));
        paths.push((PathKind::Cache, root.join("GPUCache")));
        paths.push((PathKind::Config, root));
    }
    let bundle_root = support.join("com.openai.codex");
    paths.push((PathKind::Auth, bundle_root.join("web")));
    paths.push((PathKind::Config, bundle_root));
    paths.push((PathKind::Config, home.join(".config/codex-desktop")));
    paths
}

fn children_matching(path: &Path, needles: &[&str]) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(path) else {
        return Vec::new();
    };

    entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default()
                .to_ascii_lowercase();
            needles.iter().any(|needle| name.contains(needle))
        })
        .collect()
}

fn running_processes() -> Vec<String> {
    #[cfg(windows)]
    let output = Command::new("tasklist").output();
    #[cfg(not(windows))]
    let output = Command::new("ps").args(["-axo", "comm="]).output();

    match output {
        Ok(output) if output.status.success() => String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(ToOwned::to_owned)
            .collect(),
        _ => Vec::new(),
    }
}

fn matching_processes(processes: &[String], needles: &[&str]) -> Vec<String> {
    processes
        .iter()
        .filter(|process| {
            let lower = process.to_ascii_lowercase();
            needles.iter().any(|needle| lower.contains(needle))
        })
        .cloned()
        .collect()
}

fn home_dir() -> Option<PathBuf> {
    env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

fn unix_timestamp_string() -> String {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_secs().to_string(),
        Err(_) => "0".to_string(),
    }
}

fn diagnostics_report_from_scan(scan: EnvironmentScan) -> EnvironmentDiagnosticsReport {
    EnvironmentDiagnosticsReport {
        schema_version: "environment-diagnostics/v1",
        generated_at: scan.scanned_at.clone(),
        os: scan.os,
        read_only: scan.read_only,
        environments: scan
            .environments
            .into_iter()
            .map(diagnostics_entry_from_environment)
            .collect(),
        notes: vec![
            "Generated from read-only environment detection.".to_string(),
            "Does not include token values, cookies, API keys, file contents, or unredacted account emails.".to_string(),
            "Home-directory paths are shortened to ~ when they match the current user home.".to_string(),
            "Desktop and VS Code auth path candidates remain unverified until tested on real signed-in installations.".to_string(),
        ],
    }
}

fn diagnostics_entry_from_environment(
    environment: EnvironmentState,
) -> EnvironmentDiagnosticsEntry {
    EnvironmentDiagnosticsEntry {
        id: environment.id,
        installed: environment.installed,
        executable_path: environment
            .executable_path
            .map(|path| sanitize_diagnostic_text(&path)),
        running: environment.running,
        running_processes: environment
            .running_processes
            .into_iter()
            .map(|process| sanitize_diagnostic_text(&process))
            .collect(),
        permission: environment.permission,
        account_hint: sanitize_diagnostic_text(&environment.account_hint),
        support: environment.support,
        status_message: sanitize_diagnostic_text(&environment.status_message),
        discovered_paths: environment
            .discovered_paths
            .into_iter()
            .map(|path| DiagnosticPathSummary {
                kind: path.kind,
                path: sanitize_diagnostic_text(&path.path),
                exists: path.exists,
                permission: path.permission,
            })
            .collect(),
    }
}

fn sanitize_diagnostic_text(value: &str) -> String {
    redact_diagnostic_text(value, home_dir().as_deref())
}

fn redact_diagnostic_text(value: &str, home: Option<&Path>) -> String {
    let shortened = if let Some(home) = home {
        shorten_home_path(value, home)
    } else {
        value.to_string()
    };
    redact_email_like_text(&shortened)
}

fn shorten_home_path(value: &str, home: &Path) -> String {
    let path = Path::new(value);
    if path == home {
        return "~".to_string();
    }
    match path.strip_prefix(home) {
        Ok(stripped) if stripped.as_os_str().is_empty() => "~".to_string(),
        Ok(stripped) => format!("~/{}", stripped.to_string_lossy()),
        Err(_) => value.to_string(),
    }
}

fn profile_repository() -> ProfileRepository {
    ProfileRepository::new(profile_metadata_path())
}

fn app_state_repository() -> AppStateRepository {
    AppStateRepository::new(default_app_state_dir(
        home_dir().unwrap_or_else(|| PathBuf::from(".")),
    ))
}

fn profile_metadata_path() -> PathBuf {
    home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".codex-switch")
        .join("profiles.json")
}

fn profile_store_error_message(error: ProfileStoreError) -> String {
    format!("{error:?}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matching_processes_is_case_insensitive() {
        let processes = vec![
            "/Applications/Visual Studio Code.app/Contents/MacOS/Electron".to_string(),
            "/opt/homebrew/bin/codex".to_string(),
            "Safari".to_string(),
        ];

        let matches = matching_processes(&processes, &["visual studio code", "codex"]);

        assert_eq!(matches.len(), 2);
        assert!(matches
            .iter()
            .any(|process| process.contains("Visual Studio Code")));
        assert!(matches.iter().any(|process| process.contains("codex")));
    }

    #[test]
    fn missing_path_reports_missing_permission() {
        let path = env::temp_dir().join("codex-switch-path-that-should-not-exist");

        assert_eq!(permission_for_path(&path), PermissionState::Missing);
    }

    #[test]
    fn existing_writable_path_summarizes_as_read_write() {
        let path = env::temp_dir();
        let discovered = vec![discovered_path(PathKind::Config, path)];

        assert_eq!(
            summarize_permissions(&discovered),
            PermissionState::ReadWrite
        );
    }

    #[test]
    fn environment_without_install_but_with_state_is_partial() {
        let path = env::temp_dir();
        let state = environment_state(
            "CLI",
            None,
            vec![discovered_path(PathKind::Config, path)],
            Vec::new(),
            "detected",
        );

        assert!(!state.installed);
        assert!(matches!(state.support, SupportState::Partial));
    }

    #[test]
    fn redacts_email_from_json_account_hint() {
        let content = r#"{"auth":{"email":"charlie@example.com","access_token":"secret"}}"#;

        assert_eq!(
            account_hint::redacted_account_hint_from_content(content),
            Some("c***@example.com".to_string())
        );
    }

    #[test]
    fn redacts_email_from_text_fallback() {
        let content = "signed in as user.name+codex@example.org";

        assert_eq!(
            account_hint::redacted_account_hint_from_content(content),
            Some("u***@example.org".to_string())
        );
    }

    #[test]
    fn environment_state_reports_redacted_account_hint_from_auth_path() {
        let root =
            env::temp_dir().join(format!("codex-switch-account-hint-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create hint dir");
        let auth_path = root.join("auth.json");
        fs::write(&auth_path, r#"{"email":"person@example.com"}"#).expect("write auth");

        let state = environment_state(
            "CLI",
            None,
            vec![discovered_path(PathKind::Auth, auth_path)],
            Vec::new(),
            "detected",
        );

        assert_eq!(state.account_hint, "p***@example.com");
        assert!(!state.account_hint.contains("person@example.com"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn diagnostic_text_redacts_home_and_email_like_segments() {
        let home = PathBuf::from("/Users/person@example.com");
        let value = "/Users/person@example.com/.codex/auth.json";

        assert_eq!(
            redact_diagnostic_text(value, Some(&home)),
            "~/.codex/auth.json"
        );
        assert_eq!(
            redact_diagnostic_text("/tmp/other.user@example.org/auth.json", Some(&home)),
            "/tmp/o***@example.org/auth.json"
        );
    }

    #[test]
    fn diagnostics_report_contains_redacted_read_only_scan_evidence() {
        let scan = EnvironmentScan {
            os: "macos",
            scanned_at: "1234".to_string(),
            read_only: true,
            environments: vec![EnvironmentState {
                id: "CLI",
                installed: true,
                executable_path: Some("/tmp/user@example.com/bin/codex".to_string()),
                discovered_paths: vec![DiscoveredPath {
                    kind: PathKind::Auth,
                    path: "/tmp/user@example.com/.codex/auth.json".to_string(),
                    exists: true,
                    permission: PermissionState::ReadWrite,
                }],
                running: true,
                running_processes: vec!["codex-user@example.com".to_string()],
                permission: PermissionState::ReadWrite,
                account_hint: "u***@example.com".to_string(),
                support: SupportState::Detected,
                status_message: "detected for user@example.com".to_string(),
            }],
        };

        let report = diagnostics_report_from_scan(scan);
        let json = serde_json::to_string(&report).expect("serialize diagnostics report");

        assert_eq!(report.schema_version, "environment-diagnostics/v1");
        assert!(report.read_only);
        assert!(json.contains("u***@example.com"));
        assert!(!json.contains("user@example.com"));
        assert!(json.contains("/tmp/u***@example.com/.codex/auth.json"));
    }

    #[test]
    fn custom_paths_are_added_to_matching_environment_detection() {
        let root = env::temp_dir().join(format!(
            "codex-switch-custom-path-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create custom path");
        let custom_paths = vec![EnvironmentPathOverride {
            environment: TargetEnvironment::Vscode,
            kind: PathKind::Auth,
            path: root.to_string_lossy().to_string(),
        }];

        let vscode = detect_vscode(&[], &custom_paths);
        let cli = detect_cli(&[], &custom_paths);

        assert!(vscode
            .discovered_paths
            .iter()
            .any(|path| path.kind == PathKind::Auth && path.path == root.to_string_lossy()));
        assert!(!cli
            .discovered_paths
            .iter()
            .any(|path| path.path == root.to_string_lossy()));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn vscode_candidates_include_openai_extension_storage() {
        let home = PathBuf::from("/Users/example");
        let candidates = vscode_candidate_paths_for_home(&home);

        assert!(candidates.iter().any(|(kind, path)| {
            *kind == PathKind::Auth
                && path.ends_with("Library/Application Support/Code/User/globalStorage/openai.chatgpt")
        }));
        assert!(candidates.iter().any(|(kind, path)| {
            *kind == PathKind::Auth
                && path.ends_with("Library/Application Support/Code/User/globalStorage/openai.codex")
        }));
    }

    #[test]
    fn desktop_candidates_include_browser_auth_and_cache_roots() {
        let home = PathBuf::from("/Users/example");
        let candidates = desktop_candidate_paths_for_home(&home);

        assert!(candidates.iter().any(|(kind, path)| {
            *kind == PathKind::Auth
                && path.ends_with("Library/Application Support/Codex/Default/Local Storage")
        }));
        assert!(candidates.iter().any(|(kind, path)| {
            *kind == PathKind::Auth
                && path.ends_with("Library/Application Support/Codex/Partitions/codex-browser-app")
        }));
        assert!(candidates.iter().any(|(kind, path)| {
            *kind == PathKind::Cache
                && path.ends_with("Library/Application Support/Codex/Cache")
        }));
        assert!(candidates.iter().any(|(kind, path)| {
            *kind == PathKind::Auth
                && path.ends_with("Library/Application Support/com.openai.codex/web")
        }));
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            backend_health,
            detect_environments,
            environment_diagnostics_report,
            list_profiles,
            preview_current_import,
            import_current_profile,
            update_profile,
            delete_profile,
            get_settings,
            save_settings,
            list_switch_history,
            clear_switch_history,
            check_recovery_status,
            resolve_recovery_status,
            switch_to_profile,
            restore_default_on_exit,
            restart_desktop_app,
            restart_vscode_app
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Codex Switch");
}
