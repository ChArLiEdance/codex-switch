use std::path::{Path, PathBuf};

use crate::errors::{AppError, AppResult};
use crate::models::{CodexCliStatus, SwitchHealthResponse};

use super::metadata::{auth_is_empty_placeholder, load_account_identity_from_path};
use super::paths::{get_backup_root, get_codex_home, validate_profile_name};

pub fn check_switch_health_with_home(
    profile_name: &str,
    codex_home: Option<&Path>,
    cli_status: CodexCliStatus,
    codex_desktop_running: bool,
    vscode_running: bool,
) -> AppResult<SwitchHealthResponse> {
    let codex_home = codex_home.map(PathBuf::from).unwrap_or_else(get_codex_home);
    let backup_root = get_backup_root(Some(&codex_home));
    let profile_name = validate_profile_name(profile_name)?;
    let profile_dir = backup_root.join(&profile_name);
    if !profile_dir.is_dir() {
        return Err(AppError::new(
            "PROFILE_NOT_FOUND",
            format!("Profile not found: {profile_name}"),
        ));
    }

    let target_auth_path = profile_dir.join("auth.json");
    let root_auth_path = codex_home.join("auth.json");
    let target_auth_present = target_auth_path.is_file();
    let target_identity = load_account_identity_from_path(&target_auth_path);
    let current_identity = load_account_identity_from_path(&root_auth_path);
    let current_matches_target = match (&current_identity, &target_identity) {
        (Some(current), Some(target)) => current.same_account(target),
        _ => false,
    };
    let requires_relogin = !target_auth_present || auth_is_empty_placeholder(&target_auth_path);
    let cli_available = cli_status
        .resolved_path
        .as_deref()
        .is_some_and(|path| !path.trim().is_empty());
    let mut warnings = Vec::new();
    if !cli_available {
        warnings.push("CLI_NOT_AVAILABLE".to_string());
    }
    if requires_relogin {
        warnings.push("RELOGIN_REQUIRED".to_string());
    }
    if current_matches_target {
        warnings.push("ALREADY_CURRENT_ACCOUNT".to_string());
    }

    Ok(SwitchHealthResponse {
        profile: profile_name,
        cli_available,
        cli_path: cli_status.resolved_path,
        codex_desktop_running,
        vscode_running,
        target_auth_present,
        current_matches_target,
        requires_relogin,
        current_account_label: current_identity.and_then(|identity| identity.label()),
        target_account_label: target_identity.and_then(|identity| identity.label()),
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::check_switch_health_with_home;
    use crate::models::CodexCliStatus;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_codex_home(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("codex-switch-health-{name}-{unique}"))
    }

    fn auth_with_account(account_id: &str) -> String {
        format!(
            "{{\"tokens\":{{\"account_id\":{}}}}}",
            serde_json::Value::String(account_id.to_string())
        )
    }

    fn cli_status(path: Option<&str>) -> CodexCliStatus {
        CodexCliStatus {
            resolved_path: path.map(str::to_string),
            source: "discovery".to_string(),
            suggested_paths: Vec::new(),
        }
    }

    #[test]
    fn health_reports_current_match_and_processes() {
        let codex_home = temp_codex_home("match");
        let profile_dir = codex_home.join("account_backup").join("alpha");
        fs::create_dir_all(&profile_dir).unwrap();
        fs::write(profile_dir.join("auth.json"), auth_with_account("acct_A")).unwrap();
        fs::write(codex_home.join("auth.json"), auth_with_account("acct_A")).unwrap();

        let health = check_switch_health_with_home(
            "alpha",
            Some(&codex_home),
            cli_status(Some("/usr/local/bin/codex")),
            true,
            false,
        )
        .unwrap();

        assert!(health.cli_available);
        assert!(health.codex_desktop_running);
        assert!(!health.vscode_running);
        assert!(health.current_matches_target);
        assert!(!health.requires_relogin);
        assert_eq!(health.target_account_label.as_deref(), Some("acct_A"));

        let _ = fs::remove_dir_all(&codex_home);
    }

    #[test]
    fn health_reports_relogin_for_placeholder() {
        let codex_home = temp_codex_home("placeholder");
        let profile_dir = codex_home.join("account_backup").join("alpha");
        fs::create_dir_all(&profile_dir).unwrap();
        fs::write(
            profile_dir.join("auth.json"),
            r#"{"tokens":{"account_id":"replace-me"}}"#,
        )
        .unwrap();

        let health = check_switch_health_with_home(
            "alpha",
            Some(&codex_home),
            cli_status(None),
            false,
            true,
        )
        .unwrap();

        assert!(!health.cli_available);
        assert!(health.vscode_running);
        assert!(health.requires_relogin);
        assert!(!health.current_matches_target);

        let _ = fs::remove_dir_all(&codex_home);
    }
}
