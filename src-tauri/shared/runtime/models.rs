use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct QuotaWindow {
    pub remaining_percent: Option<u8>,
    pub refresh_at: Option<String>,
    pub reset_at_timestamp: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct QuotaSummary {
    pub five_hour: QuotaWindow,
    pub weekly: QuotaWindow,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ProfileMetadata {
    pub folder_name: Option<String>,
    pub account_label: Option<String>,
    pub plan_name: Option<String>,
    pub subscription_expires_at: Option<String>,
    pub openai_base_url: Option<String>,
    pub quota: QuotaSummary,
    pub quota_updated_at_ms: Option<u64>,
    /// Wall-clock millis at the most recent confirmed plan check (any
    /// path that proved plan info was current: API plan_type override,
    /// fresh id_token claim, login). Independent of
    /// `quota_updated_at_ms` because plan changes much less frequently
    /// than usage and the UI surfaces plan freshness on its own. `None`
    /// for legacy profile.json that predates the field.
    pub last_plan_check_ms: Option<u64>,
}

impl ProfileMetadata {
    pub fn with_folder_name(folder_name: &str) -> Self {
        Self {
            folder_name: Some(folder_name.to_string()),
            ..Self::default()
        }
    }

    pub fn validate(self) -> Option<Self> {
        let five_hour_ok = self
            .quota
            .five_hour
            .remaining_percent
            .map_or(true, |value| value <= 100);
        let weekly_ok = self
            .quota
            .weekly
            .remaining_percent
            .map_or(true, |value| value <= 100);

        if five_hour_ok && weekly_ok {
            Some(self)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileCard {
    pub folder_name: String,
    pub display_title: String,
    pub account_label: Option<String>,
    pub status: String,
    pub auth_present: bool,
    pub has_account_identity: bool,
    pub plan_name: Option<String>,
    pub subscription_days_left: Option<i64>,
    pub openai_base_url: Option<String>,
    pub quota: QuotaSummary,
    /// Surfaces plan freshness to the front-end so the dashboard can
    /// render a hover-time tooltip and a stale indicator without
    /// re-fetching per-profile metadata. `None` for legacy entries
    /// that predate the field.
    pub last_plan_check_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentCard {
    pub folder_name: String,
    pub display_title: String,
    pub account_label: Option<String>,
    pub has_account_identity: bool,
    pub plan_name: Option<String>,
    pub subscription_days_left: Option<i64>,
    pub profile_folder_path: String,
    pub last_plan_check_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ProfileIndexEntry {
    pub folder_name: String,
    pub account_label: Option<String>,
    pub has_account_identity: bool,
    pub plan_name: Option<String>,
    pub subscription_expires_at: Option<String>,
    pub openai_base_url: Option<String>,
    pub auth_present: bool,
    pub stored_quota: QuotaSummary,
    pub stored_quota_updated_at_ms: Option<u64>,
    /// Mirrors `ProfileMetadata::last_plan_check_ms` after the index
    /// rolls up profile.json. Lets the dashboard show plan freshness
    /// without re-reading per-profile metadata.
    pub last_plan_check_ms: Option<u64>,
    pub auth_mtime_ms: Option<u64>,
    pub auth_size: Option<u64>,
    pub profile_mtime_ms: Option<u64>,
    pub profile_size: Option<u64>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ProfilesIndex {
    pub schema_version: u32,
    pub updated_at: String,
    pub current_profile: Option<String>,
    pub profiles: Vec<ProfileIndexEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfilesSnapshotResponse {
    pub page_size: u32,
    pub profiles: Vec<ProfileCard>,
    pub current_card: Option<CurrentCard>,
    pub current_quota_card: Option<QuotaSummary>,
    /// Set when the live `~/.codex` account has a resolvable identity that no
    /// managed profile owns (drift to an unmanaged account) — carries a label
    /// for the dashboard prompt. `None` in the normal case.
    pub unmanaged_live_account: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentQuotaResponse {
    pub profile: Option<String>,
    pub quota: Option<QuotaSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfilePayload {
    pub profile: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddProfilePayload {
    pub folder_name: String,
    pub openai_base_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameProfilePayload {
    pub profile: String,
    pub new_folder_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProfileBaseUrlPayload {
    pub profile: String,
    pub openai_base_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCheckPayload {
    pub update_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenUrlPayload {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCheckResponse {
    pub ok: bool,
    pub current_version: String,
    pub latest_version: Option<String>,
    pub has_update: bool,
    pub release_url: Option<String>,
    pub notes: Option<String>,
    pub checked_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwitchResponse {
    pub ok: bool,
    pub profile: String,
    pub message: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResponse {
    pub ok: bool,
    pub message: String,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexCliStatus {
    /// Currently resolved real-codex path, or None when nothing was
    /// found. Front-end uses this to decide whether the "已定位"
    /// indicator shows up and what to prefill the input with.
    pub resolved_path: Option<String>,
    /// `"user_override" | "install_state" | "discovery" | "none"`
    /// — frontend i18n maps this to a label so the user can tell
    /// whether they're looking at their manual override or the
    /// auto-discovered path.
    pub source: String,
    /// Common platform-specific install locations that exist on disk
    /// right now. Frontend renders these as click-to-fill chips.
    pub suggested_paths: Vec<String>,
}

/// A codex CLI candidate confirmed runnable by the re-detection scan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodexCliCandidate {
    /// Absolute path to the verified-runnable codex binary.
    pub path: String,
    /// Version line from `codex --version` (e.g. "codex-cli 0.133.0"), or
    /// None if the binary ran successfully but printed nothing parseable.
    /// Shown next to the path so the user can tell several installs apart.
    pub version: Option<String>,
}

/// Result of a forced re-detection scan triggered by the Settings
/// "auto-detect" button. Unlike `get_codex_cli_status` (which honours
/// the cached/override path), this rescans from scratch across every
/// known source and keeps only the candidates that pass a
/// `codex --version` runnable probe.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexCliRedetectResult {
    /// Verified-runnable candidates, deduped and best-first. The
    /// front-end auto-applies a lone hit and lets the user pick (with
    /// versions shown) when there are several.
    pub candidates: Vec<CodexCliCandidate>,
    /// Refreshed status snapshot so the Settings row and the dialog can
    /// update in lock-step after the scan.
    pub status: CodexCliStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetCodexCliPathPayload {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct UsageQuerySettings {
    pub enabled: bool,
    pub timeout_seconds: u32,
    /// Minutes. `0` means no automatic usage query.
    pub auto_query_interval_minutes: u32,
}

impl UsageQuerySettings {
    pub fn normalized(self) -> Self {
        Self {
            enabled: self.enabled,
            timeout_seconds: self.timeout_seconds.clamp(1, 120),
            auto_query_interval_minutes: self.auto_query_interval_minutes.min(1440),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageQuerySettingsPayload {
    pub profile: String,
    pub settings: UsageQuerySettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageStatsPayload {
    pub profile: Option<String>,
    pub start_at: Option<i64>,
    pub end_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct UsageTotals {
    pub request_count: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_creation_tokens: u64,
    pub real_total_tokens: u64,
    pub total_cost_usd: f64,
    pub cache_hit_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageTrendPoint {
    pub bucket: String,
    pub timestamp: i64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_creation_tokens: u64,
    pub real_total_tokens: u64,
    pub total_cost_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageSessionRow {
    pub profile: String,
    pub session_id: String,
    pub model: String,
    pub started_at: i64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_creation_tokens: u64,
    pub real_total_tokens: u64,
    pub total_cost_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageProfileOption {
    pub folder_name: String,
    pub display_title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageStatsResponse {
    pub profiles: Vec<UsageProfileOption>,
    pub selected_profile: Option<String>,
    pub start_at: i64,
    pub end_at: i64,
    pub totals: UsageTotals,
    pub trends: Vec<UsageTrendPoint>,
    pub sessions: Vec<UsageSessionRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexSessionMeta {
    pub session_id: String,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub project_dir: Option<String>,
    pub created_at: Option<i64>,
    pub last_active_at: Option<i64>,
    pub source_path: String,
    pub resume_command: String,
    pub profile: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexSessionMessage {
    pub role: String,
    pub content: String,
    pub ts: Option<i64>,
}
