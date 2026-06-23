use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetEnvironment {
    Cli,
    Vscode,
    Desktop,
}

impl TargetEnvironment {
    pub fn key(self) -> &'static str {
        match self {
            TargetEnvironment::Cli => "cli",
            TargetEnvironment::Vscode => "vscode",
            TargetEnvironment::Desktop => "desktop",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileAuthStatus {
    Available,
    PossiblyExpired,
    Expired,
    NotDetected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentProfileState {
    pub environment: TargetEnvironment,
    pub status: ProfileAuthStatus,
    pub secret_ref: Option<String>,
    pub completeness_reason: String,
    pub captured_at: Option<String>,
}

impl EnvironmentProfileState {
    pub fn available(
        environment: TargetEnvironment,
        secret_ref: String,
        captured_at: String,
    ) -> Self {
        Self {
            environment,
            status: ProfileAuthStatus::Available,
            secret_ref: Some(secret_ref),
            completeness_reason: "Captured from current authorized local state".to_string(),
            captured_at: Some(captured_at),
        }
    }

    pub fn missing(environment: TargetEnvironment, reason: impl Into<String>) -> Self {
        Self {
            environment,
            status: ProfileAuthStatus::NotDetected,
            secret_ref: None,
            completeness_reason: reason.into(),
            captured_at: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileMetadata {
    pub id: String,
    pub name: String,
    pub account_hint: String,
    pub tags: Vec<String>,
    pub note: String,
    pub default_profile: bool,
    pub last_used_at: Option<String>,
    pub environments: Vec<EnvironmentProfileState>,
}

impl ProfileMetadata {
    pub fn validate(&self) -> Result<(), ProfileValidationError> {
        if self.id.trim().is_empty() {
            return Err(ProfileValidationError::MissingId);
        }
        if self.name.trim().is_empty() {
            return Err(ProfileValidationError::MissingName);
        }
        if looks_like_full_email(&self.account_hint) {
            return Err(ProfileValidationError::UnredactedAccountHint);
        }
        for environment in &self.environments {
            if environment.status == ProfileAuthStatus::Available
                && environment.secret_ref.is_none()
            {
                return Err(
                    ProfileValidationError::AvailableEnvironmentMissingSecretRef(
                        environment.environment,
                    ),
                );
            }
        }
        Ok(())
    }

    pub fn supports(&self, environment: TargetEnvironment) -> bool {
        self.environments.iter().any(|state| {
            state.environment == environment && state.status == ProfileAuthStatus::Available
        })
    }

    pub fn complete_for_all_targets(&self) -> bool {
        self.supports(TargetEnvironment::Cli)
            && self.supports(TargetEnvironment::Vscode)
            && self.supports(TargetEnvironment::Desktop)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileValidationError {
    MissingId,
    MissingName,
    UnredactedAccountHint,
    AvailableEnvironmentMissingSecretRef(TargetEnvironment),
}

fn looks_like_full_email(value: &str) -> bool {
    let trimmed = value.trim();
    trimmed.contains('@') && !trimmed.contains("***") && !trimmed.contains('*')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_profile() -> ProfileMetadata {
        ProfileMetadata {
            id: "profile-1".to_string(),
            name: "Work".to_string(),
            account_hint: "c***@example.com".to_string(),
            tags: vec!["default".to_string()],
            note: "Local metadata only".to_string(),
            default_profile: true,
            last_used_at: None,
            environments: vec![
                EnvironmentProfileState::available(
                    TargetEnvironment::Cli,
                    "codex-switch:profile-1:cli".to_string(),
                    "1000".to_string(),
                ),
                EnvironmentProfileState::missing(TargetEnvironment::Vscode, "Not imported"),
                EnvironmentProfileState::missing(TargetEnvironment::Desktop, "Not imported"),
            ],
        }
    }

    #[test]
    fn validates_redacted_profile_metadata() {
        let profile = sample_profile();

        assert_eq!(profile.validate(), Ok(()));
        assert!(profile.supports(TargetEnvironment::Cli));
        assert!(!profile.complete_for_all_targets());
    }

    #[test]
    fn rejects_unredacted_email_hint() {
        let mut profile = sample_profile();
        profile.account_hint = "charlie@example.com".to_string();

        assert_eq!(
            profile.validate(),
            Err(ProfileValidationError::UnredactedAccountHint)
        );
    }

    #[test]
    fn rejects_available_environment_without_secret_reference() {
        let mut profile = sample_profile();
        profile.environments[0].secret_ref = None;

        assert_eq!(
            profile.validate(),
            Err(
                ProfileValidationError::AvailableEnvironmentMissingSecretRef(
                    TargetEnvironment::Cli
                )
            )
        );
    }
}
