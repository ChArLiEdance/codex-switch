use std::fs;
use std::num::NonZeroU32;
use std::path::{Component, Path, PathBuf};

use base64::{engine::general_purpose, Engine as _};
use ring::aead;
use ring::pbkdf2;
use ring::rand::{SecureRandom, SystemRandom};
use serde::{Deserialize, Serialize};

use crate::errors::{AppError, AppResult};
use crate::models::ProfilesBackupResponse;

use super::fs_ops::{backup_root_state_to_profile, remove_path, set_active_marker};
use super::paths::{get_backup_root, get_codex_home, list_profile_dirs, validate_profile_name};
use super::profiles::{resolve_backup_target, resolve_current_profile};
use super::profiles_index::load_profiles_index;

const BACKUP_SCHEMA_VERSION: u32 = 1;
const BACKUP_APP_ID: &str = "codex-switch";
const BACKUP_KDF: &str = "PBKDF2-HMAC-SHA256";
const BACKUP_CIPHER: &str = "AES-256-GCM";
const BACKUP_AAD: &[u8] = b"codex-switch-profile-backup-v1";
const PBKDF2_ITERATIONS: u32 = 210_000;
const MAX_BACKUP_FILE_BYTES: u64 = 32 * 1024 * 1024;
const MAX_BACKUP_TOTAL_BYTES: u64 = 128 * 1024 * 1024;

#[derive(Debug, Serialize, Deserialize)]
struct EncryptedBackupPackage {
    schema_version: u32,
    app: String,
    created_at: String,
    kdf: String,
    kdf_iterations: u32,
    cipher: String,
    salt_b64: String,
    nonce_b64: String,
    payload_b64: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct PlaintextBackup {
    schema_version: u32,
    app: String,
    exported_current_profile: Option<String>,
    profiles: Vec<BackupProfile>,
}

#[derive(Debug, Serialize, Deserialize)]
struct BackupProfile {
    folder_name: String,
    files: Vec<BackupFile>,
}

#[derive(Debug, Serialize, Deserialize)]
struct BackupFile {
    relative_path: String,
    content_b64: String,
}

fn non_empty_password(password: &str) -> AppResult<&str> {
    let trimmed = password.trim();
    if trimmed.is_empty() {
        Err(AppError::new(
            "BACKUP_PASSWORD_REQUIRED",
            "Backup password cannot be empty.",
        ))
    } else {
        Ok(trimmed)
    }
}

fn random_bytes<const N: usize>() -> AppResult<[u8; N]> {
    let rng = SystemRandom::new();
    let mut bytes = [0u8; N];
    rng.fill(&mut bytes).map_err(|_| {
        AppError::new(
            "BACKUP_RANDOM_FAILED",
            "Failed to generate encryption randomness.",
        )
    })?;
    Ok(bytes)
}

fn derive_key(password: &str, salt: &[u8], iterations: u32) -> AppResult<[u8; 32]> {
    let iterations = NonZeroU32::new(iterations).ok_or_else(|| {
        AppError::new(
            "BACKUP_KDF_INVALID",
            "Backup key-derivation iteration count is invalid.",
        )
    })?;
    let mut key = [0u8; 32];
    pbkdf2::derive(
        pbkdf2::PBKDF2_HMAC_SHA256,
        iterations,
        salt,
        password.as_bytes(),
        &mut key,
    );
    Ok(key)
}

fn seal_payload(payload: &[u8], password: &str) -> AppResult<EncryptedBackupPackage> {
    let salt = random_bytes::<16>()?;
    let nonce_bytes = random_bytes::<12>()?;
    let key = derive_key(password, &salt, PBKDF2_ITERATIONS)?;
    let unbound = aead::UnboundKey::new(&aead::AES_256_GCM, &key).map_err(|_| {
        AppError::new("BACKUP_ENCRYPT_FAILED", "Failed to initialize backup cipher.")
    })?;
    let sealing_key = aead::LessSafeKey::new(unbound);
    let nonce = aead::Nonce::assume_unique_for_key(nonce_bytes);
    let mut encrypted = payload.to_vec();
    sealing_key
        .seal_in_place_append_tag(nonce, aead::Aad::from(BACKUP_AAD), &mut encrypted)
        .map_err(|_| AppError::new("BACKUP_ENCRYPT_FAILED", "Failed to encrypt backup."))?;

    Ok(EncryptedBackupPackage {
        schema_version: BACKUP_SCHEMA_VERSION,
        app: BACKUP_APP_ID.to_string(),
        created_at: super::paths::utc_timestamp(),
        kdf: BACKUP_KDF.to_string(),
        kdf_iterations: PBKDF2_ITERATIONS,
        cipher: BACKUP_CIPHER.to_string(),
        salt_b64: general_purpose::STANDARD.encode(salt),
        nonce_b64: general_purpose::STANDARD.encode(nonce_bytes),
        payload_b64: general_purpose::STANDARD.encode(encrypted),
    })
}

fn open_payload(package: &EncryptedBackupPackage, password: &str) -> AppResult<Vec<u8>> {
    if package.schema_version != BACKUP_SCHEMA_VERSION
        || package.app != BACKUP_APP_ID
        || package.kdf != BACKUP_KDF
        || package.cipher != BACKUP_CIPHER
    {
        return Err(AppError::new(
            "BACKUP_FORMAT_UNSUPPORTED",
            "Unsupported Codex Switch backup file.",
        ));
    }

    let salt = general_purpose::STANDARD
        .decode(&package.salt_b64)
        .map_err(|_| AppError::new("BACKUP_FORMAT_INVALID", "Backup salt is invalid."))?;
    let nonce_vec = general_purpose::STANDARD
        .decode(&package.nonce_b64)
        .map_err(|_| AppError::new("BACKUP_FORMAT_INVALID", "Backup nonce is invalid."))?;
    let nonce_bytes: [u8; 12] = nonce_vec
        .as_slice()
        .try_into()
        .map_err(|_| AppError::new("BACKUP_FORMAT_INVALID", "Backup nonce length is invalid."))?;
    let mut encrypted = general_purpose::STANDARD
        .decode(&package.payload_b64)
        .map_err(|_| AppError::new("BACKUP_FORMAT_INVALID", "Backup payload is invalid."))?;
    let key = derive_key(password, &salt, package.kdf_iterations)?;
    let unbound = aead::UnboundKey::new(&aead::AES_256_GCM, &key).map_err(|_| {
        AppError::new("BACKUP_DECRYPT_FAILED", "Failed to initialize backup cipher.")
    })?;
    let opening_key = aead::LessSafeKey::new(unbound);
    let nonce = aead::Nonce::assume_unique_for_key(nonce_bytes);
    let plaintext = opening_key
        .open_in_place(nonce, aead::Aad::from(BACKUP_AAD), &mut encrypted)
        .map_err(|_| {
            AppError::new(
                "BACKUP_PASSWORD_INVALID",
                "Backup password is wrong or the file is corrupted.",
            )
        })?;
    Ok(plaintext.to_vec())
}

fn relative_file_path(profile_dir: &Path, path: &Path) -> AppResult<String> {
    let relative = path.strip_prefix(profile_dir).map_err(|_| {
        AppError::new(
            "BACKUP_PATH_INVALID",
            format!("Backup path is outside profile directory: {}", path.display()),
        )
    })?;
    Ok(relative.to_string_lossy().replace('\\', "/"))
}

fn collect_profile_files(
    profile_dir: &Path,
    current_dir: &Path,
    total_bytes: &mut u64,
    files: &mut Vec<BackupFile>,
) -> AppResult<()> {
    let mut entries = fs::read_dir(current_dir)
        .map_err(|error| {
            AppError::new(
                "BACKUP_READ_FAILED",
                format!("Failed to read {}: {error}", current_dir.display()),
            )
        })?
        .filter_map(Result::ok)
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path).map_err(|error| {
            AppError::new(
                "BACKUP_READ_FAILED",
                format!("Failed to read metadata {}: {error}", path.display()),
            )
        })?;
        if metadata.file_type().is_symlink() {
            continue;
        }
        if metadata.is_dir() {
            collect_profile_files(profile_dir, &path, total_bytes, files)?;
            continue;
        }
        if !metadata.is_file() {
            continue;
        }
        if metadata.len() > MAX_BACKUP_FILE_BYTES {
            return Err(AppError::new(
                "BACKUP_FILE_TOO_LARGE",
                format!("Profile file is too large to export: {}", path.display()),
            ));
        }
        *total_bytes = total_bytes.saturating_add(metadata.len());
        if *total_bytes > MAX_BACKUP_TOTAL_BYTES {
            return Err(AppError::new(
                "BACKUP_TOO_LARGE",
                "Profile backup is too large to export.",
            ));
        }

        let content = fs::read(&path).map_err(|error| {
            AppError::new(
                "BACKUP_READ_FAILED",
                format!("Failed to read {}: {error}", path.display()),
            )
        })?;
        files.push(BackupFile {
            relative_path: relative_file_path(profile_dir, &path)?,
            content_b64: general_purpose::STANDARD.encode(content),
        });
    }
    Ok(())
}

fn build_plaintext_backup(codex_home: &Path) -> AppResult<PlaintextBackup> {
    let backup_root = get_backup_root(Some(codex_home));
    if let Some(profile) = resolve_backup_target(&backup_root, codex_home) {
        backup_root_state_to_profile(&profile, codex_home, &backup_root)?;
    }
    load_profiles_index(Some(codex_home))?;

    let mut total_bytes = 0;
    let mut profiles = Vec::new();
    for profile_dir in list_profile_dirs(&backup_root) {
        let Some(folder_name) = profile_dir.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        let folder_name = validate_profile_name(folder_name)?;
        let mut files = Vec::new();
        collect_profile_files(&profile_dir, &profile_dir, &mut total_bytes, &mut files)?;
        profiles.push(BackupProfile { folder_name, files });
    }

    Ok(PlaintextBackup {
        schema_version: BACKUP_SCHEMA_VERSION,
        app: BACKUP_APP_ID.to_string(),
        exported_current_profile: resolve_current_profile(&backup_root),
        profiles,
    })
}

fn safe_relative_file_path(relative_path: &str) -> AppResult<PathBuf> {
    let mut path = PathBuf::new();
    for component in Path::new(relative_path).components() {
        match component {
            Component::Normal(value) => path.push(value),
            _ => {
                return Err(AppError::new(
                    "BACKUP_PATH_INVALID",
                    format!("Invalid backup relative path: {relative_path}"),
                ))
            }
        }
    }
    if path.as_os_str().is_empty() {
        return Err(AppError::new(
            "BACKUP_PATH_INVALID",
            "Backup contains an empty relative path.",
        ));
    }
    Ok(path)
}

fn import_plaintext_backup(
    backup: PlaintextBackup,
    overwrite: bool,
    codex_home: &Path,
) -> AppResult<ProfilesBackupResponse> {
    if backup.schema_version != BACKUP_SCHEMA_VERSION || backup.app != BACKUP_APP_ID {
        return Err(AppError::new(
            "BACKUP_FORMAT_UNSUPPORTED",
            "Unsupported Codex Switch backup payload.",
        ));
    }

    let backup_root = get_backup_root(Some(codex_home));
    fs::create_dir_all(&backup_root).map_err(|error| {
        AppError::new(
            "BACKUP_WRITE_FAILED",
            format!(
                "Failed to create backup root {}: {error}",
                backup_root.display()
            ),
        )
    })?;

    let mut imported_profiles = Vec::new();
    for profile in backup.profiles {
        let folder_name = validate_profile_name(&profile.folder_name)?;
        let profile_dir = backup_root.join(&folder_name);
        if profile_dir.exists() {
            if !overwrite {
                return Err(AppError::new(
                    "BACKUP_PROFILE_EXISTS",
                    format!("Profile already exists: {folder_name}"),
                ));
            }
            remove_path(&profile_dir)?;
        }
        fs::create_dir_all(&profile_dir).map_err(|error| {
            AppError::new(
                "BACKUP_WRITE_FAILED",
                format!(
                    "Failed to create profile directory {}: {error}",
                    profile_dir.display()
                ),
            )
        })?;

        for file in profile.files {
            let relative = safe_relative_file_path(&file.relative_path)?;
            let content = general_purpose::STANDARD
                .decode(&file.content_b64)
                .map_err(|_| AppError::new("BACKUP_FORMAT_INVALID", "Backup file is invalid."))?;
            let target = profile_dir.join(relative);
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(|error| {
                    AppError::new(
                        "BACKUP_WRITE_FAILED",
                        format!("Failed to create directory {}: {error}", parent.display()),
                    )
                })?;
            }
            fs::write(&target, content).map_err(|error| {
                AppError::new(
                    "BACKUP_WRITE_FAILED",
                    format!("Failed to write {}: {error}", target.display()),
                )
            })?;
        }
        imported_profiles.push(folder_name);
    }

    let imported_current_profile = backup
        .exported_current_profile
        .filter(|profile| imported_profiles.iter().any(|value| value == profile));
    if let Some(profile) = imported_current_profile.as_deref() {
        set_active_marker(profile, &backup_root)?;
    }
    load_profiles_index(Some(codex_home))?;

    Ok(ProfilesBackupResponse {
        ok: true,
        path: String::new(),
        profiles: imported_profiles,
        imported_current_profile,
    })
}

pub fn export_profiles_backup(path: &str, password: &str) -> AppResult<ProfilesBackupResponse> {
    export_profiles_backup_with_home(path, password, &get_codex_home())
}

fn export_profiles_backup_with_home(
    path: &str,
    password: &str,
    codex_home: &Path,
) -> AppResult<ProfilesBackupResponse> {
    let password = non_empty_password(password)?;
    let path = PathBuf::from(path);
    let plaintext = build_plaintext_backup(codex_home)?;
    let profiles = plaintext
        .profiles
        .iter()
        .map(|profile| profile.folder_name.clone())
        .collect::<Vec<_>>();
    let payload = serde_json::to_vec(&plaintext).map_err(|error| {
        AppError::new(
            "BACKUP_SERIALIZE_FAILED",
            format!("Failed to serialize backup payload: {error}"),
        )
    })?;
    let package = seal_payload(&payload, password)?;
    let serialized = serde_json::to_string_pretty(&package).map_err(|error| {
        AppError::new(
            "BACKUP_SERIALIZE_FAILED",
            format!("Failed to serialize encrypted backup: {error}"),
        )
    })?;
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|error| {
                AppError::new(
                    "BACKUP_WRITE_FAILED",
                    format!("Failed to create directory {}: {error}", parent.display()),
                )
            })?;
        }
    }
    fs::write(&path, format!("{serialized}\n")).map_err(|error| {
        AppError::new(
            "BACKUP_WRITE_FAILED",
            format!("Failed to write backup {}: {error}", path.display()),
        )
    })?;

    Ok(ProfilesBackupResponse {
        ok: true,
        path: path.to_string_lossy().into_owned(),
        profiles,
        imported_current_profile: None,
    })
}

pub fn import_profiles_backup(
    path: &str,
    password: &str,
    overwrite: bool,
) -> AppResult<ProfilesBackupResponse> {
    import_profiles_backup_with_home(path, password, overwrite, &get_codex_home())
}

fn import_profiles_backup_with_home(
    path: &str,
    password: &str,
    overwrite: bool,
    codex_home: &Path,
) -> AppResult<ProfilesBackupResponse> {
    let password = non_empty_password(password)?;
    let path = PathBuf::from(path);
    let raw = fs::read_to_string(&path).map_err(|error| {
        AppError::new(
            "BACKUP_READ_FAILED",
            format!("Failed to read backup {}: {error}", path.display()),
        )
    })?;
    let package = serde_json::from_str::<EncryptedBackupPackage>(&raw).map_err(|error| {
        AppError::new(
            "BACKUP_FORMAT_INVALID",
            format!("Backup package JSON is invalid: {error}"),
        )
    })?;
    let plaintext = open_payload(&package, password)?;
    let backup = serde_json::from_slice::<PlaintextBackup>(&plaintext).map_err(|error| {
        AppError::new(
            "BACKUP_FORMAT_INVALID",
            format!("Backup payload JSON is invalid: {error}"),
        )
    })?;
    let mut response = import_plaintext_backup(backup, overwrite, codex_home)?;
    response.path = path.to_string_lossy().into_owned();
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::{
        export_profiles_backup_with_home, import_profiles_backup_with_home, MAX_BACKUP_TOTAL_BYTES,
    };
    use crate::shared::paths::get_backup_root;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_codex_home(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("codex-switch-profile-backup-{name}-{unique}"))
    }

    fn seed_profile(codex_home: &PathBuf, profile: &str) {
        let profile_dir = get_backup_root(Some(codex_home)).join(profile);
        fs::create_dir_all(&profile_dir).unwrap();
        fs::write(
            profile_dir.join("auth.json"),
            format!(r#"{{"tokens":{{"account_id":"acct_{profile}"}}}}"#),
        )
        .unwrap();
        fs::write(
            profile_dir.join("profile.json"),
            format!(r#"{{"folder_name":"{profile}","account_label":"{profile}@example.com"}}"#),
        )
        .unwrap();
    }

    #[test]
    fn encrypted_backup_round_trips_profiles() {
        let source = temp_codex_home("roundtrip-source");
        let target = temp_codex_home("roundtrip-target");
        seed_profile(&source, "alpha");
        seed_profile(&source, "beta");
        let path = std::env::temp_dir().join("codex-switch-test-backup.csbak");

        let exported =
            export_profiles_backup_with_home(path.to_str().unwrap(), "secret", &source).unwrap();
        assert_eq!(exported.profiles, vec!["alpha".to_string(), "beta".to_string()]);

        let imported =
            import_profiles_backup_with_home(path.to_str().unwrap(), "secret", false, &target)
                .unwrap();
        assert_eq!(imported.profiles, vec!["alpha".to_string(), "beta".to_string()]);
        assert!(get_backup_root(Some(&target)).join("alpha/auth.json").is_file());
        assert!(get_backup_root(Some(&target)).join("beta/profile.json").is_file());

        let _ = fs::remove_file(&path);
        let _ = fs::remove_dir_all(&source);
        let _ = fs::remove_dir_all(&target);
    }

    #[test]
    fn wrong_password_is_rejected() {
        let source = temp_codex_home("wrong-password-source");
        let target = temp_codex_home("wrong-password-target");
        seed_profile(&source, "alpha");
        let path = std::env::temp_dir().join("codex-switch-test-wrong-password.csbak");
        export_profiles_backup_with_home(path.to_str().unwrap(), "secret", &source).unwrap();

        let error =
            import_profiles_backup_with_home(path.to_str().unwrap(), "wrong", false, &target)
                .unwrap_err();
        assert_eq!(error.error_code, "BACKUP_PASSWORD_INVALID");

        let _ = fs::remove_file(&path);
        let _ = fs::remove_dir_all(&source);
        let _ = fs::remove_dir_all(&target);
    }

    #[test]
    fn import_requires_overwrite_for_existing_profile() {
        let source = temp_codex_home("overwrite-source");
        let target = temp_codex_home("overwrite-target");
        seed_profile(&source, "alpha");
        seed_profile(&target, "alpha");
        let path = std::env::temp_dir().join("codex-switch-test-overwrite.csbak");
        export_profiles_backup_with_home(path.to_str().unwrap(), "secret", &source).unwrap();

        let error =
            import_profiles_backup_with_home(path.to_str().unwrap(), "secret", false, &target)
                .unwrap_err();
        assert_eq!(error.error_code, "BACKUP_PROFILE_EXISTS");

        let imported =
            import_profiles_backup_with_home(path.to_str().unwrap(), "secret", true, &target)
                .unwrap();
        assert_eq!(imported.profiles, vec!["alpha".to_string()]);

        let _ = fs::remove_file(&path);
        let _ = fs::remove_dir_all(&source);
        let _ = fs::remove_dir_all(&target);
    }

    #[test]
    fn backup_size_limit_is_nonzero() {
        assert!(MAX_BACKUP_TOTAL_BYTES > 0);
    }
}
