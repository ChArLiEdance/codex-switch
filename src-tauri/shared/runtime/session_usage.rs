use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Local, TimeZone};
use serde::Deserialize;
use serde_json::Value;

use crate::models::{
    QuotaSummary, QuotaWindow, UsageProfileOption, UsageQuerySettings, UsageSessionRow,
    UsageStatsPayload, UsageStatsResponse, UsageTotals, UsageTrendPoint,
};

use super::paths::get_codex_home;
use super::quota_cache::{file_signature, CachedEntry, CachedSnapshot, QuotaCache};
use super::quota_routing::{slot_from_window_minutes, QuotaSlot};
use super::session_files::{collect_jsonl_files, file_modified_ms};

const USAGE_SETTINGS_FILENAME: &str = "usage_settings.json";

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

fn collect_usage_files_for_profile(
    profile: &str,
    codex_home: &Path,
    current: Option<&str>,
) -> Vec<PathBuf> {
    let backup_root = super::paths::get_backup_root(Some(codex_home));
    let mut files = Vec::new();
    if current == Some(profile) {
        collect_jsonl_files(&codex_home.join("sessions"), &mut files);
    }
    collect_jsonl_files(&backup_root.join(profile).join("sessions"), &mut files);
    files.sort();
    files.dedup();
    files
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

    for profile in selected_names {
        for file in
            collect_usage_files_for_profile(&profile, &codex_home, index.current_profile.as_deref())
        {
            for event in parse_usage_events_from_file(&profile, &file) {
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
                trend.real_total_tokens +=
                    fresh_input + event.delta.output + event.delta.cached_input;
                trend.total_cost_usd += cost_for_event(&event.model, &event.delta);

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
            }
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
                total_cost_usd: 0.0,
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
