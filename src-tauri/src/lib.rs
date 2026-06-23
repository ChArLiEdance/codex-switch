pub mod profile;
pub mod secret_store;

use serde::Serialize;
use std::{
    env,
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct EnvironmentScan {
    os: &'static str,
    scanned_at: String,
    read_only: bool,
    environments: Vec<EnvironmentState>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct EnvironmentState {
    id: &'static str,
    installed: bool,
    executable_path: Option<String>,
    discovered_paths: Vec<DiscoveredPath>,
    running: bool,
    running_processes: Vec<String>,
    permission: PermissionState,
    account_hint: &'static str,
    support: SupportState,
    status_message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DiscoveredPath {
    kind: PathKind,
    path: String,
    exists: bool,
    permission: PermissionState,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum PermissionState {
    ReadWrite,
    ReadOnly,
    Missing,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "kebab-case")]
enum SupportState {
    Detected,
    Partial,
    NotDetected,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "kebab-case")]
enum PathKind {
    App,
    Auth,
    Config,
    Cache,
}

#[tauri::command]
fn backend_health() -> &'static str {
    "codex_switch_backend_ready"
}

#[tauri::command]
fn detect_environments() -> EnvironmentScan {
    let processes = running_processes();
    let environments = vec![
        detect_cli(&processes),
        detect_vscode(&processes),
        detect_desktop(&processes),
    ];

    EnvironmentScan {
        os: env::consts::OS,
        scanned_at: unix_timestamp_string(),
        read_only: true,
        environments,
    }
}

fn detect_cli(processes: &[String]) -> EnvironmentState {
    let executable_path = find_executable("codex");
    let mut discovered_paths = Vec::new();

    if let Ok(path) = env::var("CODEX_HOME") {
        discovered_paths.push(discovered_path(PathKind::Config, PathBuf::from(path)));
    }
    if let Some(home) = home_dir() {
        discovered_paths.push(discovered_path(PathKind::Config, home.join(".codex")));
        discovered_paths.push(discovered_path(PathKind::Auth, home.join(".codex").join("auth.json")));
        discovered_paths.push(discovered_path(PathKind::Cache, home.join(".codex").join("cache")));
    }

    let running_processes = matching_processes(processes, &["codex"]);
    environment_state(
        "CLI",
        executable_path,
        discovered_paths,
        running_processes,
        "Codex CLI executable and candidate local state paths discovered read-only",
    )
}

fn detect_vscode(processes: &[String]) -> EnvironmentState {
    let executable_path = find_executable("code").or_else(find_vscode_app);
    let mut discovered_paths = Vec::new();

    for path in vscode_candidate_paths() {
        discovered_paths.push(discovered_path(PathKind::Config, path.clone()));
        if path.exists() {
            for child in children_matching(&path, &["codex", "openai"]) {
                discovered_paths.push(discovered_path(PathKind::Auth, child));
            }
        }
    }

    let running_processes = matching_processes(processes, &["code", "visual studio code"]);
    environment_state(
        "VS Code",
        executable_path,
        discovered_paths,
        running_processes,
        "VS Code app and extension storage candidates discovered read-only",
    )
}

fn detect_desktop(processes: &[String]) -> EnvironmentState {
    let executable_path = find_desktop_app();
    let mut discovered_paths = Vec::new();

    for path in desktop_candidate_paths() {
        discovered_paths.push(discovered_path(PathKind::Config, path.clone()));
        if path.exists() {
            for child in children_matching(&path, &["codex", "openai"]) {
                discovered_paths.push(discovered_path(PathKind::Auth, child));
            }
        }
    }

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
        account_hint: "Unknown",
        support,
        status_message,
    }
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
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => PermissionState::ReadOnly,
        Err(_) => PermissionState::Unknown,
    }
}

fn summarize_permissions(paths: &[DiscoveredPath]) -> PermissionState {
    let existing: Vec<_> = paths.iter().filter(|path| path.exists).collect();
    if existing.is_empty() {
        return PermissionState::Unknown;
    }
    if existing.iter().all(|path| path.permission == PermissionState::ReadWrite) {
        PermissionState::ReadWrite
    } else if existing.iter().any(|path| path.permission == PermissionState::ReadOnly) {
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

fn vscode_candidate_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(home) = home_dir() {
        paths.push(home.join("Library/Application Support/Code/User/globalStorage"));
        paths.push(home.join("Library/Application Support/Code - Insiders/User/globalStorage"));
        paths.push(home.join(".config/Code/User/globalStorage"));
        paths.push(home.join(".config/Code - Insiders/User/globalStorage"));
    }
    if let Ok(appdata) = env::var("APPDATA") {
        paths.push(PathBuf::from(appdata).join("Code/User/globalStorage"));
    }
    paths
}

fn desktop_candidate_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(home) = home_dir() {
        paths.push(home.join("Library/Application Support/Codex"));
        paths.push(home.join("Library/Application Support/Codex Desktop"));
        paths.push(home.join("Library/Application Support/OpenAI/Codex"));
        paths.push(home.join(".config/codex-desktop"));
    }
    if let Ok(appdata) = env::var("APPDATA") {
        let appdata = PathBuf::from(appdata);
        paths.push(appdata.join("Codex"));
        paths.push(appdata.join("Codex Desktop"));
    }
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
        assert!(matches.iter().any(|process| process.contains("Visual Studio Code")));
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

        assert_eq!(summarize_permissions(&discovered), PermissionState::ReadWrite);
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
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![backend_health, detect_environments])
        .run(tauri::generate_context!())
        .expect("failed to run Codex Switch");
}
