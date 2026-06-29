use std::cmp::Ordering;
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use semver::Version;
use serde_json::Value;
use tauri::Emitter;
use tauri_plugin_opener::OpenerExt;

use crate::errors::{AppError, AppResult};
use crate::models::{InstallUpdateResponse, UpdateCheckResponse, UpdateDownloadProgress};
use crate::shared::paths::{RELEASES_URL, UPDATE_CHECK_URL};

const UPDATE_USER_AGENT: &str = "Codex-Switch-Updater";
const DOWNLOAD_TIMEOUT_SECONDS: u64 = 180;
pub const UPDATE_DOWNLOAD_PROGRESS_EVENT: &str = "codex-switch://update-download-progress";

#[derive(Debug, Clone)]
struct ReleaseAsset {
    name: String,
    download_url: String,
}

pub fn check_update(update_url: &str) -> AppResult<UpdateCheckResponse> {
    let checked_url = normalize_update_url(update_url)?;
    let payload = fetch_update_payload(&checked_url)?;
    let latest_version = read_latest_version(&payload)?;
    let current_version = env!("CARGO_PKG_VERSION").to_string();
    let has_update = compare_versions(&latest_version, &current_version)? == Ordering::Greater;
    let release_url = read_string_field(&payload, &["html_url", "release_url"])
        .or_else(|| Some(RELEASES_URL.to_string()));
    let notes = read_string_field(&payload, &["notes", "body"]);

    Ok(UpdateCheckResponse {
        ok: true,
        current_version,
        latest_version: Some(latest_version),
        has_update,
        release_url,
        notes,
        checked_url,
    })
}

pub fn download_and_open_update(
    app: &tauri::AppHandle,
    update_url: &str,
) -> AppResult<InstallUpdateResponse> {
    let checked_url = normalize_update_url(update_url)?;
    let payload = fetch_update_payload(&checked_url)?;
    let latest_version = read_latest_version(&payload)?;
    let current_version = env!("CARGO_PKG_VERSION");
    if compare_versions(&latest_version, current_version)? != Ordering::Greater {
        return Err(AppError::new(
            "UPDATE_ALREADY_CURRENT",
            format!("Current version {current_version} is already up to date."),
        ));
    }

    emit_update_progress(
        app,
        "preparing",
        0,
        None,
        Some(0),
        "Preparing update download.",
    );
    let asset = select_update_asset(&payload)?;
    emit_update_progress(
        app,
        "downloading",
        0,
        None,
        Some(0),
        &format!("Downloading {}.", asset.name),
    );
    let path = download_update_asset(&latest_version, &asset, |received, total| {
        let percent = total.filter(|value| *value > 0).map(|value| {
            ((received as f64 / value as f64) * 100.0)
                .round()
                .clamp(0.0, 100.0) as u8
        });
        emit_update_progress(
            app,
            "downloading",
            received,
            total,
            percent,
            "Downloading update asset.",
        );
    })?;
    emit_update_progress(
        app,
        "opening",
        0,
        None,
        Some(100),
        "Opening update installer.",
    );
    app.opener()
        .open_path(path.to_string_lossy().into_owned(), None::<&str>)
        .map_err(|error| {
            AppError::new(
                "UPDATE_INSTALLER_OPEN_FAILED",
                format!("Downloaded update but failed to open installer: {error}"),
            )
        })?;
    emit_update_progress(app, "opened", 0, None, Some(100), "Installer opened.");

    Ok(InstallUpdateResponse {
        ok: true,
        version: latest_version,
        asset_name: asset.name,
        path: path.to_string_lossy().into_owned(),
    })
}

fn emit_update_progress(
    app: &tauri::AppHandle,
    phase: &str,
    received_bytes: u64,
    total_bytes: Option<u64>,
    percent: Option<u8>,
    message: &str,
) {
    let _ = app.emit(
        UPDATE_DOWNLOAD_PROGRESS_EVENT,
        UpdateDownloadProgress {
            phase: phase.to_string(),
            received_bytes,
            total_bytes,
            percent,
            message: message.to_string(),
        },
    );
}

fn normalize_update_url(update_url: &str) -> AppResult<String> {
    let trimmed = update_url.trim();
    let url = if trimmed.is_empty() {
        UPDATE_CHECK_URL
    } else {
        trimmed
    };

    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err(AppError::new(
            "UPDATE_URL_INVALID",
            "Update URL must start with http:// or https://.",
        ));
    }

    Ok(url.to_string())
}

fn fetch_update_payload(checked_url: &str) -> AppResult<Value> {
    let body = fetch_update_json(checked_url)?;
    serde_json::from_str(&body).map_err(|error| {
        AppError::new(
            "UPDATE_JSON_INVALID",
            format!("Failed to parse update response JSON: {error}"),
        )
    })
}

fn read_latest_version(payload: &Value) -> AppResult<String> {
    read_string_field(payload, &["version", "tag_name"]).ok_or_else(|| {
        AppError::new(
            "UPDATE_VERSION_MISSING",
            "Update response did not include a version.",
        )
    })
}

fn read_string_field(payload: &Value, fields: &[&str]) -> Option<String> {
    fields.iter().find_map(|field| {
        payload
            .get(*field)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
    })
}

fn select_update_asset(payload: &Value) -> AppResult<ReleaseAsset> {
    let assets = payload
        .get("assets")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            AppError::new(
                "UPDATE_ASSETS_MISSING",
                "Update response did not include release assets.",
            )
        })?;

    let mut candidates = assets
        .iter()
        .filter_map(|asset| {
            let name = read_string_field(asset, &["name"])?;
            let download_url = read_string_field(asset, &["browser_download_url", "download_url"])?;
            let score = candidate_asset_score(&name)?;
            Some((score, ReleaseAsset { name, download_url }))
        })
        .collect::<Vec<_>>();

    candidates.sort_by(|left, right| {
        left.0
            .cmp(&right.0)
            .then_with(|| left.1.name.cmp(&right.1.name))
    });

    candidates
        .into_iter()
        .map(|(_, asset)| asset)
        .next()
        .ok_or_else(|| {
            AppError::new(
                "UPDATE_ASSET_UNSUPPORTED",
                format!(
                    "No update asset was found for this platform ({}/{}).",
                    std::env::consts::OS,
                    std::env::consts::ARCH
                ),
            )
        })
}

fn candidate_asset_score(name: &str) -> Option<u8> {
    let lower = name.to_ascii_lowercase();

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        if lower.ends_with("_aarch64.dmg") {
            return Some(0);
        }
        if lower.ends_with("_aarch64.pkg") {
            return Some(1);
        }
        return None;
    }

    #[cfg(all(windows, target_arch = "x86_64"))]
    {
        if lower.ends_with("_x64-setup.exe") || lower.ends_with("-x64-setup.exe") {
            return Some(0);
        }
        return None;
    }

    #[cfg(not(any(
        all(target_os = "macos", target_arch = "aarch64"),
        all(windows, target_arch = "x86_64")
    )))]
    {
        let _ = lower;
        None
    }
}

fn download_update_asset(
    version: &str,
    asset: &ReleaseAsset,
    mut on_progress: impl FnMut(u64, Option<u64>),
) -> AppResult<PathBuf> {
    let file_name = safe_asset_file_name(&asset.name)?;
    let url = asset.download_url.trim();
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err(AppError::new(
            "UPDATE_ASSET_URL_INVALID",
            "Update asset download URL must start with http:// or https://.",
        ));
    }

    let cache_dir = std::env::temp_dir()
        .join("codex-switch-updates")
        .join(safe_cache_segment(version));
    fs::create_dir_all(&cache_dir).map_err(|error| {
        AppError::new(
            "UPDATE_CACHE_CREATE_FAILED",
            format!("Failed to create update cache directory: {error}"),
        )
    })?;

    let path = cache_dir.join(file_name);
    let partial_path = path.with_extension(format!(
        "{}download",
        path.extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| format!("{extension}."))
            .unwrap_or_default()
    ));
    let _ = fs::remove_file(&partial_path);
    let mut response = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(DOWNLOAD_TIMEOUT_SECONDS))
        .user_agent(UPDATE_USER_AGENT)
        .build()
        .map_err(|error| {
            AppError::new(
                "UPDATE_DOWNLOAD_CLIENT_FAILED",
                format!("Failed to prepare update downloader: {error}"),
            )
        })?
        .get(url)
        .send()
        .map_err(|error| {
            AppError::new(
                "UPDATE_DOWNLOAD_FAILED",
                format!("Failed to download update asset: {error}"),
            )
        })?
        .error_for_status()
        .map_err(|error| {
            AppError::new(
                "UPDATE_DOWNLOAD_FAILED",
                format!("Update asset download failed: {error}"),
            )
        })?;
    let total = response.content_length();
    let mut file = fs::File::create(&partial_path).map_err(|error| {
        AppError::new(
            "UPDATE_ASSET_WRITE_FAILED",
            format!("Failed to create update asset: {error}"),
        )
    })?;

    let mut received = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = response.read(&mut buffer).map_err(|error| {
            AppError::new(
                "UPDATE_DOWNLOAD_FAILED",
                format!("Failed while downloading update asset: {error}"),
            )
        })?;
        if read == 0 {
            break;
        }
        file.write_all(&buffer[..read]).map_err(|error| {
            AppError::new(
                "UPDATE_ASSET_WRITE_FAILED",
                format!("Failed to write update asset: {error}"),
            )
        })?;
        received = received.saturating_add(read as u64);
        on_progress(received, total);
    }
    file.flush().map_err(|error| {
        AppError::new(
            "UPDATE_ASSET_WRITE_FAILED",
            format!("Failed to flush update asset: {error}"),
        )
    })?;

    if path.exists() {
        fs::remove_file(&path).map_err(|error| {
            AppError::new(
                "UPDATE_ASSET_WRITE_FAILED",
                format!("Failed to replace previous update asset: {error}"),
            )
        })?;
    }

    fs::rename(&partial_path, &path).map_err(|error| {
        AppError::new(
            "UPDATE_ASSET_WRITE_FAILED",
            format!("Failed to finalize update asset: {error}"),
        )
    })?;

    Ok(path)
}

fn safe_asset_file_name(name: &str) -> AppResult<&str> {
    let trimmed = name.trim();
    if trimmed.is_empty()
        || trimmed == "."
        || trimmed == ".."
        || trimmed.contains('/')
        || trimmed.contains('\\')
    {
        return Err(AppError::new(
            "UPDATE_ASSET_NAME_INVALID",
            "Update asset name is not safe to write locally.",
        ));
    }

    Ok(trimmed)
}

fn safe_cache_segment(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .collect()
}

fn compare_versions(latest_version: &str, current_version: &str) -> AppResult<Ordering> {
    let latest = parse_version(latest_version).ok_or_else(|| {
        AppError::new(
            "UPDATE_VERSION_INVALID",
            format!("Latest version is not a valid semantic version: {latest_version}"),
        )
    })?;
    let current = parse_version(current_version).ok_or_else(|| {
        AppError::new(
            "CURRENT_VERSION_INVALID",
            format!("Current version is not a valid semantic version: {current_version}"),
        )
    })?;

    Ok(latest.cmp(&current))
}

fn parse_version(version: &str) -> Option<Version> {
    let candidate = version
        .trim()
        .trim_start_matches(|character| character == 'v' || character == 'V')
        .split_whitespace()
        .next()?;

    if let Ok(version) = Version::parse(candidate) {
        return Some(version);
    }

    let normalized = normalize_short_version(candidate)?;
    Version::parse(&normalized).ok()
}

fn normalize_short_version(version: &str) -> Option<String> {
    let core_end = version
        .find(|character| character == '-' || character == '+')
        .unwrap_or(version.len());
    let core = &version[..core_end];
    if core.split('.').count() != 2 {
        return None;
    }

    Some(format!(
        "{}.0{}",
        core,
        version.get(core_end..).unwrap_or_default()
    ))
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn fetch_update_json(url: &str) -> AppResult<String> {
    let output = Command::new("curl")
        .args([
            "--fail",
            "--location",
            "--silent",
            "--show-error",
            "--max-time",
            "12",
            "--user-agent",
            UPDATE_USER_AGENT,
            url,
        ])
        .output()
        .map_err(|error| {
            AppError::new(
                "UPDATE_REQUEST_FAILED",
                format!("Failed to start update request: {error}"),
            )
        })?;

    parse_fetch_output(output.status.success(), &output.stdout, &output.stderr)
}

#[cfg(windows)]
fn fetch_update_json(url: &str) -> AppResult<String> {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x08000000;
    let script = format!(
        concat!(
            "$ProgressPreference='SilentlyContinue'; ",
            "[Net.ServicePointManager]::SecurityProtocol=[Net.SecurityProtocolType]::Tls12; ",
            "(Invoke-WebRequest -Uri $env:CODEX_SWITCH_UPDATE_URL -UseBasicParsing ",
            "-Headers @{{ 'User-Agent' = '{}' }} -TimeoutSec 12).Content"
        ),
        UPDATE_USER_AGENT,
    );

    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &script,
        ])
        .env("CODEX_SWITCH_UPDATE_URL", url)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|error| {
            AppError::new(
                "UPDATE_REQUEST_FAILED",
                format!("Failed to start update request: {error}"),
            )
        })?;

    parse_fetch_output(output.status.success(), &output.stdout, &output.stderr)
}

fn parse_fetch_output(success: bool, stdout: &[u8], stderr: &[u8]) -> AppResult<String> {
    if success {
        return String::from_utf8(stdout.to_vec()).map_err(|error| {
            AppError::new(
                "UPDATE_RESPONSE_INVALID",
                format!("Update response was not valid UTF-8: {error}"),
            )
        });
    }

    let stderr = String::from_utf8_lossy(stderr).trim().to_string();
    let message = if stderr.is_empty() {
        "Update request failed.".to_string()
    } else {
        format!("Update request failed: {stderr}")
    };
    Err(AppError::new("UPDATE_REQUEST_FAILED", message))
}

#[cfg(test)]
mod tests {
    use super::{compare_versions, parse_version};
    use std::cmp::Ordering;

    #[test]
    fn parses_versions_with_v_prefix() {
        assert_eq!(parse_version("v1.5.1").unwrap().to_string(), "1.5.1");
    }

    #[test]
    fn parses_two_part_release_versions() {
        assert_eq!(parse_version("1.5").unwrap().to_string(), "1.5.0");
        assert_eq!(parse_version("v1.5").unwrap().to_string(), "1.5.0");
    }

    #[test]
    fn compares_semantic_versions() {
        assert_eq!(
            compare_versions("v1.6.0", "1.5.9").unwrap(),
            Ordering::Greater
        );
        assert_eq!(
            compare_versions("v1.5.0", "1.5.0").unwrap(),
            Ordering::Equal
        );
        assert_eq!(compare_versions("v1.4.9", "1.5.0").unwrap(), Ordering::Less);
    }
}
