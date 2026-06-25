use std::collections::BTreeMap;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::fs;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Local, TimeZone};
use serde::Deserialize;
use serde_json::Value;

use crate::models::{
    CodexSessionMessage, CodexSessionMeta, QuotaSummary, QuotaWindow, UsageProfileOption,
    UsageQuerySettings, UsageSessionRow, UsageStatsPayload, UsageStatsResponse, UsageTotals,
    UsageTrendPoint,
};

use super::paths::get_codex_home;
use super::quota_cache::{file_signature, CachedEntry, CachedSnapshot, QuotaCache};
use super::quota_routing::{slot_from_window_minutes, QuotaSlot};
use super::session_files::{collect_jsonl_files, file_modified_ms};

const USAGE_SETTINGS_FILENAME: &str = "usage_settings.json";
const TITLE_MAX_CHARS: usize = 80;
const SUMMARY_MAX_CHARS: usize = 160;
const VSCODE_CONTEXT_PREFIX: &str = "# Context from my IDE setup:";
const CODEX_REQUEST_MARKER: &str = "my request for codex";

#[derive(Clone, Debug)]
pub struct LocalQuotaSnapshot {
    pub quota: QuotaSummary,
    pub source_mtime_ms: Option<u64>,
}

#[derive(Deserialize)]
struct SessionLine {
    #[serde(rename = "type")]
    line_type: String,
    payload: Option<SessionPayload>,
}

#[derive(Deserialize)]
struct SessionPayload {
    #[serde(rename = "type")]
    payload_type: Option<String>,
    rate_limits: Option<SessionRateLimits>,
}

#[derive(Deserialize)]
struct SessionRateLimits {
    limit_id: Option<String>,
    primary: Option<SessionRateLimitWindow>,
    secondary: Option<SessionRateLimitWindow>,
}

#[derive(Deserialize)]
struct SessionRateLimitWindow {
    used_percent: Option<f64>,
    resets_at: Option<i64>,
    window_minutes: Option<i64>,
}

#[derive(Debug, Clone, Default)]
struct CumulativeTokens {
    input: u64,
    cached_input: u64,
    output: u64,
}

#[derive(Debug, Clone, Default)]
struct DeltaTokens {
    input: u64,
    cached_input: u64,
    output: u64,
}

#[derive(Debug, Clone)]
struct UsageEvent {
    profile: String,
    session_id: String,
    model: String,
    timestamp: i64,
    delta: DeltaTokens,
}

#[derive(Debug, Default)]
struct SessionUsageAccumulator {
    profile: String,
    session_id: String,
    model: String,
    started_at: i64,
    input_tokens: u64,
    output_tokens: u64,
    cache_read_tokens: u64,
    total_cost_usd: f64,
}

struct UsageFileSource {
    profile: String,
    path: PathBuf,
}

fn get_sessions_root(codex_home: Option<&Path>) -> PathBuf {
    codex_home
        .map(Path::to_path_buf)
        .unwrap_or_else(get_codex_home)
        .join("sessions")
}

fn get_profile_usage_settings_path(profile: &str, codex_home: Option<&Path>) -> Option<PathBuf> {
    let profile = super::paths::validate_profile_name(profile).ok()?;
    let codex_home = codex_home
        .map(Path::to_path_buf)
        .unwrap_or_else(get_codex_home);
    Some(
        super::paths::get_backup_root(Some(&codex_home))
            .join(profile)
            .join(USAGE_SETTINGS_FILENAME),
    )
}

pub fn load_usage_query_settings(profile: &str, codex_home: Option<&Path>) -> UsageQuerySettings {
    let Some(path) = get_profile_usage_settings_path(profile, codex_home) else {
        return UsageQuerySettings::default().normalized();
    };
    let Ok(raw) = fs::read_to_string(path) else {
        return UsageQuerySettings::default().normalized();
    };
    serde_json::from_str::<UsageQuerySettings>(&raw)
        .unwrap_or_default()
        .normalized()
}

pub fn save_usage_query_settings(
    profile: &str,
    settings: UsageQuerySettings,
    codex_home: Option<&Path>,
) -> crate::errors::AppResult<UsageQuerySettings> {
    let profile = super::paths::validate_profile_name(profile)?;
    let codex_home = codex_home
        .map(Path::to_path_buf)
        .unwrap_or_else(get_codex_home);
    let profile_dir = super::paths::get_backup_root(Some(&codex_home)).join(&profile);
    if !profile_dir.is_dir() {
        return Err(crate::errors::AppError::new(
            "PROFILE_NOT_FOUND",
            format!("Profile not found: {profile}"),
        ));
    }
    let normalized = settings.normalized();
    let serialized = serde_json::to_vec_pretty(&normalized).map_err(|error| {
        crate::errors::AppError::new(
            "USAGE_SETTINGS_SERIALIZE_FAILED",
            format!("Failed to serialize usage settings: {error}"),
        )
    })?;
    fs::write(profile_dir.join(USAGE_SETTINGS_FILENAME), serialized).map_err(|error| {
        crate::errors::AppError::new(
            "USAGE_SETTINGS_WRITE_FAILED",
            format!("Failed to write usage settings: {error}"),
        )
    })?;
    Ok(normalized)
}

fn session_files_descending(codex_home: Option<&Path>) -> Vec<PathBuf> {
    let sessions_root = get_sessions_root(codex_home);
    if !sessions_root.is_dir() {
        return Vec::new();
    }

    let mut files = Vec::new();
    collect_jsonl_files(&sessions_root, &mut files);
    files.sort_by(|left, right| right.as_os_str().cmp(left.as_os_str()));
    files
}

fn format_reset_time(timestamp: i64) -> Option<String> {
    Local
        .timestamp_opt(timestamp, 0)
        .single()
        .map(|datetime| datetime.format("%Y-%m-%d %H:%M").to_string())
}

fn normalize_quota_window(window: QuotaWindow) -> QuotaWindow {
    QuotaWindow {
        remaining_percent: window.remaining_percent.map(|value| value.min(100)),
        refresh_at: window.refresh_at,
        reset_at_timestamp: window.reset_at_timestamp,
    }
}

pub fn normalize_quota_summary(
    quota: Option<QuotaSummary>,
    _plan_name: Option<&str>,
    has_account_identity: bool,
) -> QuotaSummary {
    // `_plan_name` used to gate whether the 5h window was zeroed out
    // for plans labeled "free". Two reasons that gate is gone now:
    //   1. With `quota_routing` correctly bucketing by `window_minutes`,
    //      a Team account whose only enforced window is weekly no
    //      longer leaks into the 5h slot, so there's nothing to zero.
    //   2. `apply_paid_fallback_for_free_plan` flips a stale "free"
    //      claim to `unknown_paid` whenever any window has data, so
    //      the few entries that actually arrive labeled as both
    //      "free" + non-empty 5h are intentional signals worth
    //      surfacing rather than silently masking.
    if !has_account_identity {
        return QuotaSummary::default();
    }

    let quota = quota.unwrap_or_default();
    QuotaSummary {
        five_hour: normalize_quota_window(quota.five_hour),
        weekly: normalize_quota_window(quota.weekly),
    }
}

fn quota_window_from_rate_limit(window: Option<SessionRateLimitWindow>) -> QuotaWindow {
    let Some(window) = window else {
        return QuotaWindow::default();
    };

    let remaining_percent = window
        .used_percent
        .map(|used_percent| (100.0 - used_percent).round().clamp(0.0, 100.0) as u8);
    let refresh_at = window.resets_at.and_then(format_reset_time);
    let reset_at_timestamp = window.resets_at;

    QuotaWindow {
        remaining_percent,
        refresh_at,
        reset_at_timestamp,
    }
}

fn apply_rate_limit_window(
    summary: &mut QuotaSummary,
    window: Option<SessionRateLimitWindow>,
    fallback: QuotaSlot,
) {
    let Some(window) = window else {
        return;
    };

    let slot = slot_from_window_minutes(window.window_minutes, fallback);
    let quota_window = quota_window_from_rate_limit(Some(window));

    match slot {
        QuotaSlot::FiveHour => summary.five_hour = quota_window,
        QuotaSlot::Weekly => summary.weekly = quota_window,
    }
}

struct ParsedQuotaEvent {
    quota: QuotaSummary,
    limit_id: Option<String>,
}

fn is_primary_codex_limit(limit_id: Option<&str>) -> bool {
    limit_id
        .map(str::trim)
        .is_some_and(|value| value.eq_ignore_ascii_case("codex"))
}

fn quota_from_line(line: &str) -> Option<ParsedQuotaEvent> {
    let parsed = serde_json::from_str::<SessionLine>(line).ok()?;
    if parsed.line_type != "event_msg" {
        return None;
    }

    let payload = parsed.payload?;
    if payload.payload_type.as_deref() != Some("token_count") {
        return None;
    }

    let rate_limits = payload.rate_limits?;
    let mut quota = QuotaSummary::default();
    apply_rate_limit_window(&mut quota, rate_limits.primary, QuotaSlot::FiveHour);
    apply_rate_limit_window(&mut quota, rate_limits.secondary, QuotaSlot::Weekly);

    (quota.five_hour.remaining_percent.is_some()
        || quota.five_hour.refresh_at.is_some()
        || quota.weekly.remaining_percent.is_some()
        || quota.weekly.refresh_at.is_some())
    .then_some(ParsedQuotaEvent {
        quota,
        limit_id: rate_limits.limit_id,
    })
}

fn select_latest_quota_from_lines<'a>(
    lines: impl Iterator<Item = &'a str>,
) -> Option<QuotaSummary> {
    let mut latest_quota = None;
    let mut latest_codex_quota = None;

    for line in lines {
        let Some(event) = quota_from_line(line) else {
            continue;
        };

        if is_primary_codex_limit(event.limit_id.as_deref()) {
            latest_codex_quota = Some(event.quota.clone());
        }
        latest_quota = Some(event.quota);
    }

    latest_codex_quota.or(latest_quota)
}

fn load_latest_quota_from_file(path: &Path) -> Option<QuotaSummary> {
    let raw = fs::read_to_string(path).ok()?;
    select_latest_quota_from_lines(raw.lines())
}

fn parse_session_timestamp(value: &Value, fallback: i64) -> i64 {
    value
        .get("timestamp")
        .and_then(Value::as_str)
        .and_then(|raw| DateTime::parse_from_rfc3339(raw).ok())
        .map(|datetime| datetime.timestamp())
        .unwrap_or(fallback)
}

fn parse_timestamp_value_to_seconds(value: &Value) -> Option<i64> {
    if let Some(value) = value.as_i64() {
        return Some(if value > 1_000_000_000_000 {
            value / 1000
        } else {
            value
        });
    }
    value
        .as_str()
        .and_then(|raw| DateTime::parse_from_rfc3339(raw).ok())
        .map(|datetime| datetime.timestamp())
}

fn extract_text(value: &Value) -> String {
    match value {
        Value::String(text) => text.to_string(),
        Value::Array(items) => items
            .iter()
            .map(extract_text)
            .filter(|text| !text.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n"),
        Value::Object(map) => map
            .get("text")
            .or_else(|| map.get("input_text"))
            .or_else(|| map.get("output_text"))
            .or_else(|| map.get("content"))
            .map(extract_text)
            .unwrap_or_default(),
        _ => String::new(),
    }
}

fn truncate_summary(text: &str, max_chars: usize) -> String {
    let trimmed = text.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }
    let mut value = trimmed.chars().take(max_chars).collect::<String>();
    value.push_str("...");
    value
}

fn path_basename(value: &str) -> Option<String> {
    let trimmed = value.trim().trim_end_matches(['/', '\\']);
    trimmed
        .split(['/', '\\'])
        .next_back()
        .filter(|segment| !segment.is_empty())
        .map(str::to_string)
}

fn codex_request_heading_payload(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    if !trimmed.starts_with('#') {
        return None;
    }
    let heading = trimmed.trim_start_matches('#').trim_start();
    let lowered = heading.to_ascii_lowercase();
    if !lowered.starts_with(CODEX_REQUEST_MARKER) {
        return None;
    }
    let suffix = heading[CODEX_REQUEST_MARKER.len()..].trim_start();
    if suffix.is_empty() {
        return Some("");
    }
    let separator = suffix.chars().next()?;
    if !matches!(separator, ':' | '：' | '-' | '—') {
        return None;
    }
    Some(
        suffix
            .trim_start_matches(|c: char| c.is_whitespace() || matches!(c, ':' | '：' | '-' | '—'))
            .trim(),
    )
}

fn extract_codex_prompt_from_ide_context(text: &str) -> Option<String> {
    let normalized = text.replace("\r\n", "\n");
    let lines = normalized.lines().collect::<Vec<_>>();
    let mut prompt: Option<String> = None;
    for (index, line) in lines.iter().enumerate() {
        let Some(inline_prompt) = codex_request_heading_payload(line) else {
            continue;
        };
        if !inline_prompt.is_empty() {
            prompt = Some(inline_prompt.to_string());
            continue;
        }
        let following_prompt = lines[index + 1..].join("\n").trim().to_string();
        prompt = (!following_prompt.is_empty()).then_some(following_prompt);
    }
    prompt
}

fn title_candidate_from_user_message(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty()
        || trimmed.starts_with("# AGENTS.md")
        || trimmed.starts_with("<environment_context>")
    {
        return None;
    }
    if trimmed.starts_with(VSCODE_CONTEXT_PREFIX) {
        return extract_codex_prompt_from_ide_context(trimmed);
    }
    Some(trimmed.to_string())
}

fn infer_session_id_from_filename(path: &Path) -> Option<String> {
    let file_name = path.file_name()?.to_string_lossy();
    let stem = file_name.trim_end_matches(".jsonl");
    stem.rsplit('-')
        .next()
        .filter(|value| value.len() >= 8)
        .map(str::to_string)
        .or_else(|| path.file_stem()?.to_str().map(str::to_string))
}

fn is_subagent_source(source: Option<&Value>) -> bool {
    source
        .and_then(Value::as_object)
        .map(|source| source.contains_key("subagent"))
        .unwrap_or(false)
}

fn normalize_model_name(raw: &str) -> String {
    let mut name = raw.trim().to_lowercase();
    if let Some((_, tail)) = name.rsplit_once('/') {
        name = tail.to_string();
    }
    for suffix_len in [11usize, 9usize] {
        if name.len() > suffix_len {
            let suffix = &name[name.len() - suffix_len..];
            let is_iso = suffix_len == 11
                && suffix.as_bytes().first() == Some(&b'-')
                && suffix[1..5].chars().all(|c| c.is_ascii_digit())
                && suffix.as_bytes().get(5) == Some(&b'-')
                && suffix[6..8].chars().all(|c| c.is_ascii_digit())
                && suffix.as_bytes().get(8) == Some(&b'-')
                && suffix[9..11].chars().all(|c| c.is_ascii_digit());
            let is_compact = suffix_len == 9
                && suffix.as_bytes().first() == Some(&b'-')
                && suffix[1..].chars().all(|c| c.is_ascii_digit());
            if is_iso || is_compact {
                name.truncate(name.len() - suffix_len);
            }
        }
    }
    if name.is_empty() {
        "unknown".to_string()
    } else {
        name
    }
}

fn parse_cumulative_tokens(value: &Value) -> Option<CumulativeTokens> {
    if !value.is_object() {
        return None;
    }
    Some(CumulativeTokens {
        input: value
            .get("input_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        cached_input: value
            .get("cached_input_tokens")
            .or_else(|| value.get("cache_read_input_tokens"))
            .and_then(Value::as_u64)
            .unwrap_or(0),
        output: value
            .get("output_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0),
    })
}

fn token_delta(previous: &Option<CumulativeTokens>, current: &CumulativeTokens) -> DeltaTokens {
    match previous {
        Some(previous) => DeltaTokens {
            input: current.input.saturating_sub(previous.input),
            cached_input: current.cached_input.saturating_sub(previous.cached_input),
            output: current.output.saturating_sub(previous.output),
        },
        None => DeltaTokens {
            input: current.input,
            cached_input: current.cached_input,
            output: current.output,
        },
    }
}

fn cost_for_event(model: &str, delta: &DeltaTokens) -> f64 {
    let model = model.to_lowercase();
    let (input_per_m, output_per_m, cache_per_m) = if model.contains("gpt-5") {
        (1.25, 10.0, 0.125)
    } else if model.contains("gpt-4.1") {
        (2.0, 8.0, 0.5)
    } else if model.contains("gpt-4") || model.contains("o3") {
        (5.0, 15.0, 1.25)
    } else {
        (1.0, 4.0, 0.25)
    };
    let fresh_input = delta.input.saturating_sub(delta.cached_input) as f64;
    (fresh_input * input_per_m
        + delta.output as f64 * output_per_m
        + delta.cached_input as f64 * cache_per_m)
        / 1_000_000.0
}

fn parse_usage_events_from_file(profile: &str, path: &Path) -> Vec<UsageEvent> {
    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(_) => return Vec::new(),
    };
    let fallback_ts = file_modified_ms(path)
        .and_then(|mtime| i64::try_from(mtime / 1000).ok())
        .unwrap_or(0);

    let mut events = Vec::new();
    let mut session_id = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("unknown")
        .to_string();
    let mut model = "unknown".to_string();
    let mut previous_total: Option<CumulativeTokens> = None;

    for line in raw.lines() {
        if !line.contains("token_count")
            && !line.contains("turn_context")
            && !line.contains("session_meta")
        {
            continue;
        }
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        match value.get("type").and_then(Value::as_str) {
            Some("session_meta") => {
                if let Some(next_id) = value
                    .get("payload")
                    .and_then(|payload| {
                        payload
                            .get("session_id")
                            .or_else(|| payload.get("sessionId"))
                            .or_else(|| payload.get("id"))
                    })
                    .and_then(Value::as_str)
                {
                    session_id = next_id.to_string();
                }
            }
            Some("turn_context") => {
                if let Some(next_model) = value
                    .get("payload")
                    .and_then(|payload| {
                        payload
                            .get("model")
                            .or_else(|| payload.get("info").and_then(|info| info.get("model")))
                    })
                    .and_then(Value::as_str)
                {
                    model = normalize_model_name(next_model);
                }
            }
            Some("event_msg") => {
                let Some(payload) = value.get("payload") else {
                    continue;
                };
                if payload.get("type").and_then(Value::as_str) != Some("token_count") {
                    continue;
                }
                let Some(info) = payload.get("info").filter(|info| info.is_object()) else {
                    continue;
                };
                if let Some(next_model) = info
                    .get("model")
                    .or_else(|| info.get("model_name"))
                    .or_else(|| payload.get("model"))
                    .and_then(Value::as_str)
                {
                    model = normalize_model_name(next_model);
                }
                let (tokens, is_total) = if let Some(total) = info.get("total_token_usage") {
                    (parse_cumulative_tokens(total), true)
                } else if let Some(last) = info.get("last_token_usage") {
                    (parse_cumulative_tokens(last), false)
                } else {
                    (None, false)
                };
                let Some(tokens) = tokens else {
                    continue;
                };
                let mut delta = if is_total {
                    let delta = token_delta(&previous_total, &tokens);
                    previous_total = Some(tokens);
                    delta
                } else {
                    DeltaTokens {
                        input: tokens.input,
                        cached_input: tokens.cached_input,
                        output: tokens.output,
                    }
                };
                delta.cached_input = delta.cached_input.min(delta.input);
                if delta.input == 0 && delta.output == 0 && delta.cached_input == 0 {
                    continue;
                }
                events.push(UsageEvent {
                    profile: profile.to_string(),
                    session_id: session_id.clone(),
                    model: model.clone(),
                    timestamp: parse_session_timestamp(&value, fallback_ts),
                    delta,
                });
            }
            _ => {}
        }
    }

    events
}

fn usage_file_timestamp_ms(path: &Path) -> Option<u64> {
    let raw = fs::read_to_string(path).ok()?;
    let mut fallback = None;
    for line in raw.lines().take(500) {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let timestamp = value
            .get("timestamp")
            .and_then(parse_timestamp_value_to_seconds)
            .and_then(|seconds| u64::try_from(seconds).ok())
            .map(|seconds| seconds.saturating_mul(1000))
            .or_else(|| {
                value
                    .get("payload")
                    .and_then(|payload| payload.get("timestamp"))
                    .and_then(parse_timestamp_value_to_seconds)
                    .and_then(|seconds| u64::try_from(seconds).ok())
                    .map(|seconds| seconds.saturating_mul(1000))
            });

        if fallback.is_none() {
            fallback = timestamp;
        }
        if value.get("type").and_then(Value::as_str) == Some("event_msg")
            && value
                .get("payload")
                .and_then(|payload| payload.get("type"))
                .and_then(Value::as_str)
                == Some("token_count")
        {
            return timestamp.or(fallback);
        }
    }
    fallback.or_else(|| file_modified_ms(path))
}

fn infer_live_usage_profile(path: &Path, index: &crate::models::ProfilesIndex) -> Option<String> {
    let timestamp_ms = usage_file_timestamp_ms(path).or_else(|| file_modified_ms(path));
    let Some(timestamp_ms) = timestamp_ms else {
        return index.current_profile.clone();
    };

    index
        .profiles
        .iter()
        .filter_map(|profile| {
            profile
                .auth_mtime_ms
                .filter(|mtime| *mtime <= timestamp_ms)
                .map(|mtime| (mtime, profile.folder_name.clone()))
        })
        .max_by_key(|(mtime, _)| *mtime)
        .map(|(_, profile)| profile)
        .or_else(|| index.current_profile.clone())
}

fn collect_usage_file_sources(
    selected_profiles: &[String],
    codex_home: &Path,
    index: &crate::models::ProfilesIndex,
) -> Vec<UsageFileSource> {
    let backup_root = super::paths::get_backup_root(Some(codex_home));
    let selected = selected_profiles.iter().cloned().collect::<HashSet<_>>();
    let mut seen = HashSet::new();
    let mut sources = Vec::new();

    for profile in selected_profiles {
        let mut files = Vec::new();
        collect_jsonl_files(&backup_root.join(profile).join("sessions"), &mut files);
        for path in files {
            let key = (profile.clone(), path.to_string_lossy().to_string());
            if seen.insert(key) {
                sources.push(UsageFileSource {
                    profile: profile.clone(),
                    path,
                });
            }
        }
    }

    let mut live_files = Vec::new();
    collect_jsonl_files(&codex_home.join("sessions"), &mut live_files);
    for path in live_files {
        let Some(profile) = infer_live_usage_profile(&path, index) else {
            continue;
        };
        if !selected.contains(&profile) {
            continue;
        }
        let key = (profile.clone(), path.to_string_lossy().to_string());
        if seen.insert(key) {
            sources.push(UsageFileSource { profile, path });
        }
    }

    sources.sort_by(|left, right| left.path.cmp(&right.path));
    sources
}

fn read_head_tail_lines(
    path: &Path,
    head_count: usize,
    tail_count: usize,
) -> std::io::Result<(Vec<String>, Vec<String>)> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut head = Vec::with_capacity(head_count);
    let mut tail = VecDeque::with_capacity(tail_count);

    for line in reader.lines() {
        let line = line?;
        if head.len() < head_count {
            head.push(line.clone());
        }
        if tail_count > 0 {
            if tail.len() == tail_count {
                tail.pop_front();
            }
            tail.push_back(line);
        }
    }

    Ok((head, tail.into_iter().collect()))
}

fn parse_session_meta(path: &Path, profile: Option<&str>) -> Option<CodexSessionMeta> {
    let (head, tail) = read_head_tail_lines(path, 10, 30).ok()?;
    let mut session_id: Option<String> = None;
    let mut project_dir: Option<String> = None;
    let mut created_at: Option<i64> = None;
    let mut last_active_at: Option<i64> = None;
    let mut first_user_message: Option<String> = None;
    let mut summary: Option<String> = None;

    for line in &head {
        let Ok(value) = serde_json::from_str::<Value>(line.as_str()) else {
            continue;
        };
        if created_at.is_none() {
            created_at = value
                .get("timestamp")
                .and_then(parse_timestamp_value_to_seconds);
        }
        if value.get("type").and_then(Value::as_str) == Some("session_meta") {
            if let Some(payload) = value.get("payload") {
                if is_subagent_source(payload.get("source")) {
                    return None;
                }
                session_id = session_id.or_else(|| {
                    payload
                        .get("session_id")
                        .or_else(|| payload.get("id"))
                        .and_then(Value::as_str)
                        .map(str::to_string)
                });
                project_dir = project_dir.or_else(|| {
                    payload
                        .get("cwd")
                        .and_then(Value::as_str)
                        .map(str::to_string)
                });
                created_at = created_at.or_else(|| {
                    payload
                        .get("timestamp")
                        .and_then(parse_timestamp_value_to_seconds)
                });
            }
        }
        if first_user_message.is_none()
            && value.get("type").and_then(Value::as_str) == Some("response_item")
        {
            if let Some(payload) = value.get("payload") {
                if payload.get("type").and_then(Value::as_str) == Some("message")
                    && payload.get("role").and_then(Value::as_str) == Some("user")
                {
                    let text = payload.get("content").map(extract_text).unwrap_or_default();
                    first_user_message = title_candidate_from_user_message(&text);
                }
            }
        }
    }

    for line in tail.iter().rev() {
        let Ok(value) = serde_json::from_str::<Value>(line.as_str()) else {
            continue;
        };
        if last_active_at.is_none() {
            last_active_at = value
                .get("timestamp")
                .and_then(parse_timestamp_value_to_seconds);
        }
        if summary.is_none() && value.get("type").and_then(Value::as_str) == Some("response_item") {
            if let Some(payload) = value.get("payload") {
                if payload.get("type").and_then(Value::as_str) == Some("message") {
                    let text = payload.get("content").map(extract_text).unwrap_or_default();
                    if !text.trim().is_empty() {
                        summary = Some(text);
                    }
                }
            }
        }
        if last_active_at.is_some() && summary.is_some() {
            break;
        }
    }

    let session_id = session_id.or_else(|| infer_session_id_from_filename(path))?;
    let title = first_user_message
        .map(|title| truncate_summary(&title, TITLE_MAX_CHARS))
        .or_else(|| {
            project_dir
                .as_deref()
                .and_then(path_basename)
                .map(|value| value.to_string())
        });
    let summary = summary.map(|value| truncate_summary(&value, SUMMARY_MAX_CHARS));
    Some(CodexSessionMeta {
        session_id: session_id.clone(),
        title,
        summary,
        project_dir,
        created_at,
        last_active_at,
        source_path: path.to_string_lossy().to_string(),
        resume_command: format!("codex resume {session_id}"),
        profile: profile.map(str::to_string),
    })
}

pub fn list_codex_sessions(
    codex_home: Option<&Path>,
) -> crate::errors::AppResult<Vec<CodexSessionMeta>> {
    let codex_home = codex_home
        .map(Path::to_path_buf)
        .unwrap_or_else(get_codex_home);
    let index = super::profiles_index::load_profiles_index(Some(&codex_home))?;
    let backup_root = super::paths::get_backup_root(Some(&codex_home));
    let mut files: Vec<(PathBuf, Option<String>)> = Vec::new();
    let mut live_files = Vec::new();
    collect_jsonl_files(&codex_home.join("sessions"), &mut live_files);
    collect_jsonl_files(&codex_home.join("archived_sessions"), &mut live_files);
    files.extend(live_files.into_iter().map(|path| (path, None)));

    for profile in &index.profiles {
        let mut profile_files = Vec::new();
        collect_jsonl_files(
            &backup_root.join(&profile.folder_name).join("sessions"),
            &mut profile_files,
        );
        files.extend(
            profile_files
                .into_iter()
                .map(|path| (path, Some(profile.folder_name.clone()))),
        );
    }

    let mut seen = HashSet::new();
    let mut sessions = Vec::new();
    for (path, profile) in files {
        let key = path.to_string_lossy().to_string();
        if !seen.insert(key) {
            continue;
        }
        if let Some(meta) = parse_session_meta(&path, profile.as_deref()) {
            sessions.push(meta);
        }
    }
    sessions.sort_by(|left, right| {
        let left_ts = left.last_active_at.or(left.created_at).unwrap_or(0);
        let right_ts = right.last_active_at.or(right.created_at).unwrap_or(0);
        right_ts.cmp(&left_ts)
    });
    sessions.truncate(300);
    Ok(sessions)
}

fn session_path_allowed(path: &Path, codex_home: &Path) -> bool {
    let Ok(canonical_path) = path.canonicalize() else {
        return false;
    };
    let candidates = [
        codex_home.join("sessions"),
        codex_home.join("archived_sessions"),
        super::paths::get_backup_root(Some(codex_home)),
    ];
    candidates.iter().any(|root| {
        root.canonicalize()
            .map(|canonical_root| canonical_path.starts_with(canonical_root))
            .unwrap_or(false)
    })
}

pub fn load_codex_session_messages(
    source_path: &str,
    codex_home: Option<&Path>,
) -> crate::errors::AppResult<Vec<CodexSessionMessage>> {
    let codex_home = codex_home
        .map(Path::to_path_buf)
        .unwrap_or_else(get_codex_home);
    let path = PathBuf::from(source_path);
    if !session_path_allowed(&path, &codex_home) {
        return Err(crate::errors::AppError::new(
            "SESSION_PATH_NOT_ALLOWED",
            "Session path is outside managed Codex session directories.",
        ));
    }
    let file = File::open(&path).map_err(|error| {
        crate::errors::AppError::new(
            "SESSION_READ_FAILED",
            format!("Failed to read session file: {error}"),
        )
    })?;
    let mut messages = Vec::new();
    let reader = BufReader::new(file);
    for line in reader.lines() {
        let Ok(line) = line else {
            continue;
        };
        let Ok(value) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if value.get("type").and_then(Value::as_str) != Some("response_item") {
            continue;
        }
        let Some(payload) = value.get("payload") else {
            continue;
        };
        let payload_type = payload.get("type").and_then(Value::as_str).unwrap_or("");
        let (role, content) = match payload_type {
            "message" => {
                let role = payload
                    .get("role")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string();
                let content = payload.get("content").map(extract_text).unwrap_or_default();
                (role, content)
            }
            "function_call" => {
                let name = payload
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown");
                ("assistant".to_string(), format!("[Tool: {name}]"))
            }
            "function_call_output" => {
                let output = payload
                    .get("output")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                ("tool".to_string(), output)
            }
            _ => continue,
        };
        if content.trim().is_empty() {
            continue;
        }
        messages.push(CodexSessionMessage {
            role,
            content,
            ts: value
                .get("timestamp")
                .and_then(parse_timestamp_value_to_seconds),
        });
    }
    Ok(messages)
}

fn default_usage_range() -> (i64, i64) {
    let now = chrono::Utc::now().timestamp();
    (now.saturating_sub(24 * 60 * 60), now)
}

fn bucket_timestamp(timestamp: i64, start_at: i64, end_at: i64) -> i64 {
    let span = end_at.saturating_sub(start_at);
    let width = if span <= 24 * 60 * 60 {
        60 * 60
    } else {
        24 * 60 * 60
    };
    timestamp - timestamp.rem_euclid(width)
}

fn push_totals(totals: &mut UsageTotals, delta: &DeltaTokens, model: &str) {
    let fresh_input = delta.input.saturating_sub(delta.cached_input);
    totals.request_count += 1;
    totals.input_tokens += fresh_input;
    totals.output_tokens += delta.output;
    totals.cache_read_tokens += delta.cached_input;
    totals.real_total_tokens += fresh_input + delta.output + delta.cached_input;
    totals.total_cost_usd += cost_for_event(model, delta);
}

fn finish_totals(totals: &mut UsageTotals) {
    let cacheable = totals.input_tokens + totals.cache_read_tokens + totals.cache_creation_tokens;
    totals.cache_hit_rate = if cacheable == 0 {
        0.0
    } else {
        totals.cache_read_tokens as f64 / cacheable as f64
    };
}

pub fn load_usage_stats(
    payload: UsageStatsPayload,
    codex_home: Option<&Path>,
) -> crate::errors::AppResult<UsageStatsResponse> {
    let codex_home = codex_home
        .map(Path::to_path_buf)
        .unwrap_or_else(get_codex_home);
    let index = super::profiles_index::load_profiles_index(Some(&codex_home))?;
    let profiles: Vec<UsageProfileOption> = index
        .profiles
        .iter()
        .map(|profile| UsageProfileOption {
            folder_name: profile.folder_name.clone(),
            display_title: profile
                .account_label
                .clone()
                .unwrap_or_else(|| profile.folder_name.clone()),
        })
        .collect();
    let selected_profile = payload.profile.and_then(|profile| {
        profiles
            .iter()
            .any(|option| option.folder_name == profile)
            .then_some(profile)
    });
    let (default_start, default_end) = default_usage_range();
    let start_at = payload.start_at.unwrap_or(default_start);
    let end_at = payload.end_at.unwrap_or(default_end).max(start_at);

    let selected_names: Vec<String> = selected_profile
        .clone()
        .map(|profile| vec![profile])
        .unwrap_or_else(|| {
            profiles
                .iter()
                .map(|profile| profile.folder_name.clone())
                .collect()
        });

    let mut totals = UsageTotals::default();
    let mut trend_map: BTreeMap<i64, UsageTrendPoint> = BTreeMap::new();
    let mut session_map: BTreeMap<(String, String), SessionUsageAccumulator> = BTreeMap::new();

    for source in collect_usage_file_sources(&selected_names, &codex_home, &index) {
        for event in parse_usage_events_from_file(&source.profile, &source.path) {
            if event.timestamp < start_at || event.timestamp > end_at {
                continue;
            }
            push_totals(&mut totals, &event.delta, &event.model);

            let fresh_input = event.delta.input.saturating_sub(event.delta.cached_input);
            let bucket = bucket_timestamp(event.timestamp, start_at, end_at);
            let trend = trend_map.entry(bucket).or_insert_with(|| UsageTrendPoint {
                bucket: Local
                    .timestamp_opt(bucket, 0)
                    .single()
                    .map(|datetime| datetime.format("%Y-%m-%d %H:%M").to_string())
                    .unwrap_or_else(|| bucket.to_string()),
                timestamp: bucket,
                input_tokens: 0,
                output_tokens: 0,
                cache_read_tokens: 0,
                cache_creation_tokens: 0,
                real_total_tokens: 0,
                total_cost_usd: 0.0,
            });
            trend.input_tokens += fresh_input;
            trend.output_tokens += event.delta.output;
            trend.cache_read_tokens += event.delta.cached_input;
            trend.real_total_tokens += fresh_input + event.delta.output + event.delta.cached_input;
            let event_cost = cost_for_event(&event.model, &event.delta);
            trend.total_cost_usd += event_cost;

            let session = session_map
                .entry((event.profile.clone(), event.session_id.clone()))
                .or_insert_with(|| SessionUsageAccumulator {
                    profile: event.profile.clone(),
                    session_id: event.session_id.clone(),
                    model: event.model.clone(),
                    started_at: event.timestamp,
                    ..SessionUsageAccumulator::default()
                });
            session.started_at = session.started_at.min(event.timestamp);
            session.model = event.model.clone();
            session.input_tokens += fresh_input;
            session.output_tokens += event.delta.output;
            session.cache_read_tokens += event.delta.cached_input;
            session.total_cost_usd += event_cost;
        }
    }

    finish_totals(&mut totals);
    let mut sessions: Vec<UsageSessionRow> = session_map
        .into_values()
        .map(|session| {
            let real_total_tokens =
                session.input_tokens + session.output_tokens + session.cache_read_tokens;
            UsageSessionRow {
                profile: session.profile,
                session_id: session.session_id,
                model: session.model,
                started_at: session.started_at,
                input_tokens: session.input_tokens,
                output_tokens: session.output_tokens,
                cache_read_tokens: session.cache_read_tokens,
                cache_creation_tokens: 0,
                real_total_tokens,
                total_cost_usd: session.total_cost_usd,
            }
        })
        .collect();
    sessions.sort_by(|left, right| right.started_at.cmp(&left.started_at));
    sessions.truncate(100);

    Ok(UsageStatsResponse {
        profiles,
        selected_profile,
        start_at,
        end_at,
        totals,
        trends: trend_map.into_values().collect(),
        sessions,
    })
}

#[allow(dead_code)]
pub fn load_latest_local_quota(codex_home: Option<&Path>) -> Option<QuotaSummary> {
    load_latest_local_quota_snapshot(codex_home).map(|snapshot| snapshot.quota)
}

pub fn load_latest_local_quota_snapshot(codex_home: Option<&Path>) -> Option<LocalQuotaSnapshot> {
    load_latest_local_quota_snapshot_since(codex_home, None)
}

pub fn load_latest_local_quota_snapshot_since(
    codex_home: Option<&Path>,
    min_source_mtime_ms: Option<u64>,
) -> Option<LocalQuotaSnapshot> {
    let mut cache = QuotaCache::load(codex_home);
    let scan_paths: Vec<PathBuf> = session_files_descending(codex_home)
        .into_iter()
        .take(32)
        .collect();

    // Tier-1 fast path: if the previously winning file still exists,
    // is still the lex-largest jsonl in `sessions/`, and its
    // `(mtime, size)` signature is unchanged, we already know the
    // answer — no JSONL reads required. The dashboard's 15-second
    // ticker hits this >99% of the time on an idle session corpus
    // (a 1.1 GB / 232-file corpus parses in 0.5–5 s in the slow path
    // on the maintainer's machine, vs. ~10 ms here).
    if let Some(snapshot) = try_fast_path(&cache, &scan_paths, min_source_mtime_ms) {
        return Some(snapshot);
    }

    let mut next_last_snapshot: Option<CachedSnapshot> = None;
    let mut hit: Option<LocalQuotaSnapshot> = None;
    for path in &scan_paths {
        let signature = file_signature(path);
        let source_mtime_ms = if signature.0 == 0 {
            file_modified_ms(path)
        } else {
            Some(signature.0)
        };
        if min_source_mtime_ms.is_some_and(|min_mtime| source_mtime_ms.unwrap_or(0) < min_mtime) {
            continue;
        }

        // Per-file cache: skip parsing files that haven't moved. An
        // entry whose `quota` is `None` means "previously parsed, no
        // token_count event in this file" — we don't need to re-read.
        let quota_for_path = match cache.lookup(path, signature) {
            Some(entry) => entry.quota.clone(),
            None => {
                let parsed = load_latest_quota_from_file(path);
                // Skip caching when stat failed (`signature == (0, 0)`):
                // a stored `(0, 0)` entry would be a false hit on the
                // next tick for any other transiently-inaccessible file
                // and silently suppress its parse. The slow path will
                // re-attempt this path on the next tick when stat works
                // again.
                if signature != (0, 0) {
                    cache.upsert_entry(
                        path.clone(),
                        CachedEntry {
                            mtime_ms: signature.0,
                            size: signature.1,
                            quota: parsed.clone(),
                        },
                    );
                }
                parsed
            }
        };

        if let Some(quota) = quota_for_path {
            next_last_snapshot = Some(CachedSnapshot {
                path: path.clone(),
                mtime_ms: signature.0,
                size: signature.1,
                quota: quota.clone(),
                source_mtime_ms,
            });
            hit = Some(LocalQuotaSnapshot {
                quota,
                source_mtime_ms,
            });
            break;
        }
    }

    match next_last_snapshot {
        Some(snapshot) => cache.set_last_snapshot(snapshot),
        None => cache.clear_last_snapshot(),
    }
    cache.save(codex_home);

    hit
}

fn try_fast_path(
    cache: &QuotaCache,
    scan_paths: &[PathBuf],
    min_source_mtime_ms: Option<u64>,
) -> Option<LocalQuotaSnapshot> {
    let last = cache.last_snapshot.as_ref()?;
    // Only short-circuit when the cached file is still the
    // lex-largest entry — otherwise a brand-new session would be
    // ignored until the cache happened to be invalidated by some
    // other change.
    let newest = scan_paths.first()?;
    if newest != &last.path {
        return None;
    }
    let signature = file_signature(&last.path);
    if signature != (last.mtime_ms, last.size) {
        return None;
    }
    if min_source_mtime_ms.is_some_and(|min_mtime| last.source_mtime_ms.unwrap_or(0) < min_mtime) {
        return None;
    }
    Some(LocalQuotaSnapshot {
        quota: last.quota.clone(),
        source_mtime_ms: last.source_mtime_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token_count_line(
        limit_id: Option<&str>,
        primary_used_percent: f64,
        secondary_used_percent: f64,
    ) -> String {
        let limit_id_field = limit_id
            .map(|value| format!(r#""limit_id":"{value}","#))
            .unwrap_or_default();

        format!(
            r#"{{"type":"event_msg","payload":{{"type":"token_count","rate_limits":{{{limit_id_field}"primary":{{"used_percent":{primary_used_percent},"resets_at":1730000000,"window_minutes":300}},"secondary":{{"used_percent":{secondary_used_percent},"resets_at":1730600000,"window_minutes":10080}}}}}}}}"#
        )
    }

    #[test]
    fn prefers_main_codex_limit_over_later_model_specific_limit() {
        let lines = [
            token_count_line(Some("codex"), 11.0, 12.0),
            token_count_line(Some("codex_bengalfox"), 0.0, 0.0),
        ];

        let quota = select_latest_quota_from_lines(lines.iter().map(String::as_str))
            .expect("expected quota to be parsed");

        assert_eq!(quota.five_hour.remaining_percent, Some(89));
        assert_eq!(quota.weekly.remaining_percent, Some(88));
    }

    #[test]
    fn falls_back_to_latest_available_limit_when_main_codex_is_absent() {
        let lines = [
            token_count_line(Some("codex_bengalfox"), 25.0, 30.0),
            token_count_line(Some("codex_koala"), 5.0, 6.0),
        ];

        let quota = select_latest_quota_from_lines(lines.iter().map(String::as_str))
            .expect("expected quota to be parsed");

        assert_eq!(quota.five_hour.remaining_percent, Some(95));
        assert_eq!(quota.weekly.remaining_percent, Some(94));
    }

    #[test]
    fn live_usage_attribution_prefers_token_count_timestamp() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "codex-switch-live-usage-attribution-{unique}.jsonl"
        ));
        fs::write(
            &path,
            [
                r#"{"timestamp":"2026-06-25T09:02:03Z","type":"session_meta","payload":{"id":"a"}}"#,
                r#"{"timestamp":"2026-06-25T09:23:31Z","type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":10,"cached_input_tokens":0,"output_tokens":1}}}}"#,
            ]
            .join("\n"),
        )
        .unwrap();

        let index = crate::models::ProfilesIndex {
            current_profile: Some("fallback".to_string()),
            profiles: vec![
                crate::models::ProfileIndexEntry {
                    folder_name: "charlie".to_string(),
                    auth_mtime_ms: Some(1_782_375_554_611),
                    ..Default::default()
                },
                crate::models::ProfileIndexEntry {
                    folder_name: "hester".to_string(),
                    auth_mtime_ms: Some(1_782_378_356_078),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        assert_eq!(
            infer_live_usage_profile(&path, &index).as_deref(),
            Some("hester")
        );

        let _ = fs::remove_file(path);
    }

    /// Cache integration tests — guard the fast-path / per-file-cache
    /// invariants of `load_latest_local_quota_snapshot_since`. These
    /// regressions would silently re-introduce the multi-second
    /// dashboard stalls the cache exists to prevent, *without*
    /// breaking any of the parser-level assertions above, so they
    /// need their own coverage.
    mod cache_integration {
        use super::super::*;
        use std::fs;
        use std::path::PathBuf;
        use std::time::{SystemTime, UNIX_EPOCH};

        fn temp_codex_home(name: &str) -> PathBuf {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let pid = std::process::id();
            let path = std::env::temp_dir()
                .join(format!("codex-switch-session-usage-{name}-{pid}-{unique}"));
            // Cache helpers resolve through `get_runtime_dir`, which
            // hardcodes `account_backup/<platform>/`. Pre-create both
            // so the test setup works on either CI host.
            fs::create_dir_all(path.join("account_backup").join("windows")).unwrap();
            fs::create_dir_all(path.join("account_backup").join("macos")).unwrap();
            path
        }

        fn write_jsonl(codex_home: &Path, rel: &str, body: &str) {
            let path = codex_home.join("sessions").join(rel);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, body).unwrap();
        }

        const QUOTA_LINE: &str = r#"{"type":"event_msg","payload":{"type":"token_count","rate_limits":{"limit_id":"codex","primary":{"used_percent":11,"resets_at":1730000000,"window_minutes":300},"secondary":{"used_percent":12,"resets_at":1730600000,"window_minutes":10080}}}}"#;
        const QUOTA_LINE_DIFFERENT: &str = r#"{"type":"event_msg","payload":{"type":"token_count","rate_limits":{"limit_id":"codex","primary":{"used_percent":50,"resets_at":1730000000,"window_minutes":300},"secondary":{"used_percent":60,"resets_at":1730600000,"window_minutes":10080}}}}"#;

        #[test]
        fn fast_path_returns_seeded_snapshot_when_signature_matches() {
            // Seed the cache with a DIFFERENT quota than what the
            // file would yield if parsed. If the fast path works, the
            // seeded value comes back; if a regression makes the
            // function re-parse, the file's real quota (89) wins
            // instead of the seeded 99.
            let codex_home = temp_codex_home("fast-path-seeded");
            let rel = "2026/05/10/rollout-A.jsonl";
            write_jsonl(&codex_home, rel, QUOTA_LINE);
            let path = codex_home.join("sessions").join(rel);
            let signature = file_signature(&path);

            let mut cache = QuotaCache::default();
            cache.set_last_snapshot(CachedSnapshot {
                path: path.clone(),
                mtime_ms: signature.0,
                size: signature.1,
                quota: QuotaSummary {
                    five_hour: QuotaWindow {
                        remaining_percent: Some(99),
                        refresh_at: None,
                        ..QuotaWindow::default()
                    },
                    weekly: QuotaWindow::default(),
                },
                source_mtime_ms: Some(signature.0),
            });
            cache.save(Some(&codex_home));

            let result = load_latest_local_quota_snapshot(Some(&codex_home))
                .expect("fast path returns the seeded snapshot");
            assert_eq!(
                result.quota.five_hour.remaining_percent,
                Some(99),
                "fast path should return cached snapshot rather than re-parsing"
            );

            let _ = fs::remove_dir_all(&codex_home);
        }

        #[test]
        fn fast_path_falls_through_when_a_newer_jsonl_appears() {
            let codex_home = temp_codex_home("fast-path-new-file");
            write_jsonl(&codex_home, "2026/05/10/rollout-A.jsonl", QUOTA_LINE);

            let first = load_latest_local_quota_snapshot(Some(&codex_home))
                .expect("first call returns quota");
            assert_eq!(first.quota.five_hour.remaining_percent, Some(89));

            // Add a lex-larger file with a different quota — the fast
            // path's `newest != last.path` guard must reject the
            // cached snapshot and pick up the new file.
            write_jsonl(
                &codex_home,
                "2026/05/10/rollout-Z.jsonl",
                QUOTA_LINE_DIFFERENT,
            );

            let second = load_latest_local_quota_snapshot(Some(&codex_home))
                .expect("second call returns quota from the new file");
            assert_eq!(
                second.quota.five_hour.remaining_percent,
                Some(50),
                "newest path should win over the cached snapshot"
            );

            let _ = fs::remove_dir_all(&codex_home);
        }

        #[test]
        fn fast_path_falls_through_when_winning_file_signature_changes() {
            let codex_home = temp_codex_home("fast-path-sig-change");
            write_jsonl(&codex_home, "2026/05/10/rollout-A.jsonl", QUOTA_LINE);

            let first = load_latest_local_quota_snapshot(Some(&codex_home))
                .expect("first call returns quota");
            assert_eq!(first.quota.five_hour.remaining_percent, Some(89));

            // Append more data — different content + larger size.
            // Either size or mtime change is enough to invalidate the
            // fast-path signature check.
            let path = codex_home.join("sessions/2026/05/10/rollout-A.jsonl");
            let mut existing = fs::read_to_string(&path).unwrap();
            existing.push('\n');
            existing.push_str(QUOTA_LINE_DIFFERENT);
            fs::write(&path, existing).unwrap();

            let second = load_latest_local_quota_snapshot(Some(&codex_home))
                .expect("second call returns quota");
            assert_eq!(
                second.quota.five_hour.remaining_percent,
                Some(50),
                "signature drift should force re-parse and pick up the newer event"
            );

            let _ = fs::remove_dir_all(&codex_home);
        }

        #[test]
        fn per_file_cache_remembers_files_with_no_quota_event() {
            let codex_home = temp_codex_home("per-file-skip");
            // Older file: no token_count event — `load_latest_quota_from_file`
            // returns `None`. Newer file: has a quota.
            write_jsonl(
                &codex_home,
                "2026/05/10/rollout-A.jsonl",
                "{\"type\":\"event_msg\"}\n",
            );
            write_jsonl(&codex_home, "2026/05/10/rollout-Z.jsonl", QUOTA_LINE);

            let first = load_latest_local_quota_snapshot(Some(&codex_home))
                .expect("first call returns quota from rollout-Z");
            assert_eq!(first.quota.five_hour.remaining_percent, Some(89));

            // Delete the winning file so the slow path has to walk to
            // the older one. If the per-file `quota: None` cache entry
            // is honored, we get `None` without re-reading the empty
            // file.
            fs::remove_file(codex_home.join("sessions/2026/05/10/rollout-Z.jsonl")).unwrap();

            let second = load_latest_local_quota_snapshot(Some(&codex_home));
            assert!(
                second.is_none(),
                "older file's cached `None` should short-circuit; got {second:?}"
            );

            let _ = fs::remove_dir_all(&codex_home);
        }
    }
}
