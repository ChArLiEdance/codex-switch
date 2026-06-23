use crate::profile::ProfileMetadata;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileStoreError {
    Io(String),
    Json(String),
    Validation(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileStoreDocument {
    pub schema_version: u32,
    pub profiles: Vec<ProfileMetadata>,
}

impl Default for ProfileStoreDocument {
    fn default() -> Self {
        Self {
            schema_version: 1,
            profiles: Vec::new(),
        }
    }
}

pub struct ProfileRepository {
    path: PathBuf,
}

impl ProfileRepository {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(&self) -> Result<ProfileStoreDocument, ProfileStoreError> {
        if !self.path.exists() {
            return Ok(ProfileStoreDocument::default());
        }

        let content = fs::read_to_string(&self.path)
            .map_err(|error| ProfileStoreError::Io(error.to_string()))?;
        serde_json::from_str(&content).map_err(|error| ProfileStoreError::Json(error.to_string()))
    }

    pub fn save(&self, document: &ProfileStoreDocument) -> Result<(), ProfileStoreError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|error| ProfileStoreError::Io(error.to_string()))?;
        }

        let content = serde_json::to_string_pretty(document)
            .map_err(|error| ProfileStoreError::Json(error.to_string()))?;
        let temporary_path = self.path.with_extension("json.tmp");
        fs::write(&temporary_path, content).map_err(|error| ProfileStoreError::Io(error.to_string()))?;
        fs::rename(&temporary_path, &self.path)
            .map_err(|error| ProfileStoreError::Io(error.to_string()))
    }

    pub fn list_profiles(&self) -> Result<Vec<ProfileMetadata>, ProfileStoreError> {
        Ok(self.load()?.profiles)
    }

    pub fn upsert_profile(&self, profile: ProfileMetadata) -> Result<(), ProfileStoreError> {
        profile
            .validate()
            .map_err(|error| ProfileStoreError::Validation(format!("{error:?}")))?;
        let mut document = self.load()?;
        document.profiles.retain(|existing| existing.id != profile.id);
        if profile.default_profile {
            for existing in &mut document.profiles {
                existing.default_profile = false;
            }
        }
        document.profiles.push(profile);
        document.profiles.sort_by(|left, right| left.name.cmp(&right.name));
        self.save(&document)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::{EnvironmentProfileState, TargetEnvironment};

    fn test_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "codex-switch-profile-store-{name}-{}.json",
            std::process::id()
        ))
    }

    fn profile(id: &str, default_profile: bool) -> ProfileMetadata {
        ProfileMetadata {
            id: id.to_string(),
            name: id.to_string(),
            account_hint: "u***@example.com".to_string(),
            tags: Vec::new(),
            note: String::new(),
            default_profile,
            last_used_at: None,
            environments: vec![EnvironmentProfileState::available(
                TargetEnvironment::Cli,
                format!("profile:{id}:environment:cli"),
                "1000".to_string(),
            )],
        }
    }

    #[test]
    fn missing_store_loads_as_empty_document() {
        let path = test_path("missing");
        let _ = fs::remove_file(&path);
        let repository = ProfileRepository::new(path);

        assert!(repository.list_profiles().expect("list profiles").is_empty());
    }

    #[test]
    fn upsert_profile_persists_metadata_without_secrets() {
        let path = test_path("upsert");
        let _ = fs::remove_file(&path);
        let repository = ProfileRepository::new(path.clone());

        repository
            .upsert_profile(profile("profile-1", true))
            .expect("save profile");
        let content = fs::read_to_string(&path).expect("read metadata");

        assert!(content.contains("profile-1"));
        assert!(!content.contains("access_token"));
        assert_eq!(repository.list_profiles().expect("list profiles").len(), 1);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn new_default_profile_clears_existing_default() {
        let path = test_path("default");
        let _ = fs::remove_file(&path);
        let repository = ProfileRepository::new(path.clone());

        repository
            .upsert_profile(profile("profile-1", true))
            .expect("save first");
        repository
            .upsert_profile(profile("profile-2", true))
            .expect("save second");

        let profiles = repository.list_profiles().expect("list profiles");
        assert_eq!(profiles.iter().filter(|profile| profile.default_profile).count(), 1);
        assert!(profiles
            .iter()
            .any(|profile| profile.id == "profile-2" && profile.default_profile));
        let _ = fs::remove_file(path);
    }
}

