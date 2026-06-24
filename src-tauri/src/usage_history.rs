use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageTokenTotals {
    pub input_tokens: u64,
    pub cached_input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
}

impl UsageTokenTotals {
    fn add_delta(&mut self, delta: &UsageTokenTotals) {
        self.input_tokens = self.input_tokens.saturating_add(delta.input_tokens);
        self.cached_input_tokens = self
            .cached_input_tokens
            .saturating_add(delta.cached_input_tokens);
        self.output_tokens = self.output_tokens.saturating_add(delta.output_tokens);
        self.total_tokens = self
            .input_tokens
            .saturating_add(self.cached_input_tokens)
            .saturating_add(self.output_tokens);
    }

    fn is_zero(&self) -> bool {
        self.input_tokens == 0 && self.cached_input_tokens == 0 && self.output_tokens == 0
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageQuotaWindow {
    pub remaining_percent: Option<u8>,
    pub reset_at: Option<String>,
    pub window_minutes: Option<i64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageQuotaSummary {
    pub five_hour: UsageQuotaWindow,
    pub weekly: UsageQuotaWindow,
    pub source_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageSessionSummary {
    pub session_id: Option<String>,
    pub source_path: String,
    pub modified_at: Option<String>,
    pub latest_event_at: Option<String>,
    pub model: String,
    pub token_events: u32,
    pub tokens: UsageTokenTotals,
    pub quota: Option<UsageQuotaSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageHistoryReport {
    pub scanned_at: String,
    pub codex_home: String,
    pub sessions_root: String,
    pub archived_sessions_root: String,
    pub files_scanned: u32,
    pub parse_errors: Vec<String>,
    pub totals: UsageTokenTotals,
    pub latest_quota: Option<UsageQuotaSummary>,
    pub sessions: Vec<UsageSessionSummary>,
}

#[derive(Debug, Clone, Default)]
struct FileParseState {
    session_id: Option<String>,
    model: String,
    previous_total: Option<UsageTokenTotals>,
    token_events: u32,
    tokens: UsageTokenTotals,
    quota: Option<UsageQuotaSummary>,
    latest_event_at: Option<String>,
}

pub fn load_usage_history() -> UsageHistoryReport {
    let codex_home = default_codex_home();
    load_usage_history_from_codex_home(&codex_home)
}

pub fn load_usage_history_from_codex_home(codex_home: &Path) -> UsageHistoryReport {
    let sessions_root = codex_home.join("sessions");
    let archived_sessions_root = codex_home.join("archived_sessions");
    let mut files = Vec::new();
    collect_jsonl_recursive(&sessions_root, &mut files, 0, 4);
    collect_jsonl_recursive(&archived_sessions_root, &mut files, 0, 1);
    files.sort_by(|left, right| right.as_os_str().cmp(left.as_os_str()));

    let mut sessions = Vec::new();
    let mut parse_errors = Vec::new();
    let mut totals = UsageTokenTotals::default();
    let mut latest_quota = None;

    for file in &files {
        match parse_usage_session_file(file) {
            Ok(Some(session)) => {
                totals.add_delta(&session.tokens);
                if session.quota.is_some() {
                    latest_quota = session.quota.clone();
                }
                sessions.push(session);
            }
            Ok(None) => {}
            Err(error) => parse_errors.push(format!("{}: {error}", file.display())),
        }
    }

    sessions.sort_by(|left, right| {
        right
            .latest_event_at
            .cmp(&left.latest_event_at)
            .then_with(|| right.modified_at.cmp(&left.modified_at))
    });

    UsageHistoryReport {
        scanned_at: unix_timestamp_string(),
        codex_home: codex_home.to_string_lossy().into_owned(),
        sessions_root: sessions_root.to_string_lossy().into_owned(),
        archived_sessions_root: archived_sessions_root.to_string_lossy().into_owned(),
        files_scanned: files.len() as u32,
        parse_errors,
        totals,
        latest_quota,
        sessions,
    }
}

fn parse_usage_session_file(path: &Path) -> Result<Option<UsageSessionSummary>, String> {
    let file = fs::File::open(path).map_err(|error| error.to_string())?;
    let reader = BufReader::new(file);
    let mut state = FileParseState {
        model: "unknown".to_string(),
        ..FileParseState::default()
    };

    for line_result in reader.lines() {
        let line = line_result.map_err(|error| error.to_string())?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if !line.contains("\"session_meta\"")
            && !line.contains("\"turn_context\"")
            && !line.contains("\"event_msg\"")
        {
            continue;
        }

        let value: Value = match serde_json::from_str(line) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let line_type = value
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default();
        match line_type {
            "session_meta" => {
                if state.session_id.is_none() {
                    state.session_id = value
                        .get("payload")
                        .and_then(|payload| {
                            payload
                                .get("session_id")
                                .or_else(|| payload.get("sessionId"))
                                .or_else(|| payload.get("id"))
                        })
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned);
                }
            }
            "turn_context" => {
                if let Some(model) = value
                    .get("payload")
                    .and_then(|payload| {
                        payload
                            .get("model")
                            .or_else(|| payload.get("info").and_then(|info| info.get("model")))
                    })
                    .and_then(Value::as_str)
                {
                    state.model = normalize_model(model);
                }
            }
            "event_msg" => parse_event_msg(&value, path, &mut state),
            _ => {}
        }
    }

    if state.token_events == 0 && state.quota.is_none() {
        return Ok(None);
    }

    Ok(Some(UsageSessionSummary {
        session_id: state.session_id,
        source_path: path.to_string_lossy().into_owned(),
        modified_at: file_modified_timestamp(path),
        latest_event_at: state.latest_event_at,
        model: state.model,
        token_events: state.token_events,
        tokens: state.tokens,
        quota: state.quota,
    }))
}

fn parse_event_msg(value: &Value, path: &Path, state: &mut FileParseState) {
    let Some(payload) = value.get("payload") else {
        return;
    };
    if payload.get("type").and_then(Value::as_str) != Some("token_count") {
        return;
    }

    if let Some(timestamp) = value.get("timestamp").and_then(Value::as_str) {
        state.latest_event_at = Some(timestamp.to_string());
    }

    if let Some(model) = payload
        .get("info")
        .and_then(|info| {
            info.get("model")
                .or_else(|| info.get("model_name"))
                .or_else(|| payload.get("model"))
        })
        .and_then(Value::as_str)
    {
        state.model = normalize_model(model);
    }

    if let Some(quota) = quota_from_payload(payload, path) {
        state.quota = Some(quota);
    }

    let Some(info) = payload.get("info") else {
        return;
    };
    let (tokens, cumulative) = if let Some(total) = info.get("total_token_usage") {
        (parse_token_totals(total), true)
    } else if let Some(last) = info.get("last_token_usage") {
        (parse_token_totals(last), false)
    } else {
        (None, false)
    };
    let Some(tokens) = tokens else {
        return;
    };

    let mut delta = if cumulative {
        compute_delta(&state.previous_total, &tokens)
    } else {
        tokens.clone()
    };
    delta.cached_input_tokens = delta.cached_input_tokens.min(delta.input_tokens);
    delta.total_tokens = delta
        .input_tokens
        .saturating_add(delta.cached_input_tokens)
        .saturating_add(delta.output_tokens);

    if cumulative {
        state.previous_total = Some(tokens);
    }
    if delta.is_zero() {
        return;
    }

    state.token_events = state.token_events.saturating_add(1);
    state.tokens.add_delta(&delta);
}

fn parse_token_totals(value: &Value) -> Option<UsageTokenTotals> {
    value.is_object().then(|| {
        let input_tokens = value
            .get("input_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let cached_input_tokens = value
            .get("cached_input_tokens")
            .or_else(|| value.get("cache_read_input_tokens"))
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let output_tokens = value
            .get("output_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        UsageTokenTotals {
            input_tokens,
            cached_input_tokens,
            output_tokens,
            total_tokens: input_tokens
                .saturating_add(cached_input_tokens)
                .saturating_add(output_tokens),
        }
    })
}

fn compute_delta(
    previous: &Option<UsageTokenTotals>,
    current: &UsageTokenTotals,
) -> UsageTokenTotals {
    match previous {
        None => current.clone(),
        Some(previous) => UsageTokenTotals {
            input_tokens: current.input_tokens.saturating_sub(previous.input_tokens),
            cached_input_tokens: current
                .cached_input_tokens
                .saturating_sub(previous.cached_input_tokens),
            output_tokens: current.output_tokens.saturating_sub(previous.output_tokens),
            total_tokens: 0,
        },
    }
}

fn quota_from_payload(payload: &Value, source_path: &Path) -> Option<UsageQuotaSummary> {
    let rate_limits = payload.get("rate_limits")?;
    let mut quota = UsageQuotaSummary {
        source_path: Some(source_path.to_string_lossy().into_owned()),
        ..UsageQuotaSummary::default()
    };
    apply_quota_window(&mut quota, rate_limits.get("primary"), QuotaSlot::FiveHour);
    apply_quota_window(&mut quota, rate_limits.get("secondary"), QuotaSlot::Weekly);

    let has_data = quota.five_hour.remaining_percent.is_some()
        || quota.five_hour.reset_at.is_some()
        || quota.weekly.remaining_percent.is_some()
        || quota.weekly.reset_at.is_some();
    has_data.then_some(quota)
}

#[derive(Debug, Clone, Copy)]
enum QuotaSlot {
    FiveHour,
    Weekly,
}

fn apply_quota_window(
    summary: &mut UsageQuotaSummary,
    window: Option<&Value>,
    fallback: QuotaSlot,
) {
    let Some(window) = window else {
        return;
    };
    let slot = match window.get("window_minutes").and_then(Value::as_i64) {
        Some(minutes) if minutes > 300 => QuotaSlot::Weekly,
        Some(_) => QuotaSlot::FiveHour,
        None => fallback,
    };
    let used_percent = window.get("used_percent").and_then(Value::as_f64);
    let quota_window = UsageQuotaWindow {
        remaining_percent: used_percent.map(|used| (100.0 - used).round().clamp(0.0, 100.0) as u8),
        reset_at: window
            .get("resets_at")
            .and_then(Value::as_i64)
            .map(|value| value.to_string()),
        window_minutes: window.get("window_minutes").and_then(Value::as_i64),
    };
    match slot {
        QuotaSlot::FiveHour => summary.five_hour = quota_window,
        QuotaSlot::Weekly => summary.weekly = quota_window,
    }
}

fn normalize_model(raw: &str) -> String {
    let mut name = raw.trim().to_lowercase();
    if let Some((_, suffix)) = name.rsplit_once('/') {
        name = suffix.to_string();
    }
    if name.len() > 11 {
        let suffix = &name[name.len() - 11..];
        if suffix.as_bytes().first() == Some(&b'-')
            && suffix[1..5].chars().all(|char| char.is_ascii_digit())
            && suffix.as_bytes().get(5) == Some(&b'-')
            && suffix[6..8].chars().all(|char| char.is_ascii_digit())
            && suffix.as_bytes().get(8) == Some(&b'-')
            && suffix[9..11].chars().all(|char| char.is_ascii_digit())
        {
            name.truncate(name.len() - 11);
        }
    }
    if let Some((prefix, suffix)) = name.rsplit_once('-') {
        if suffix.len() == 8 && suffix.chars().all(|char| char.is_ascii_digit()) {
            name = prefix.to_string();
        }
    }
    name
}

fn collect_jsonl_recursive(dir: &Path, files: &mut Vec<PathBuf>, depth: u32, max_depth: u32) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() && depth < max_depth {
            collect_jsonl_recursive(&path, files, depth + 1, max_depth);
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("jsonl") {
            files.push(path);
        }
    }
}

fn default_codex_home() -> PathBuf {
    std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".codex")))
        .unwrap_or_else(|| PathBuf::from(".codex"))
}

fn file_modified_timestamp(path: &Path) -> Option<String> {
    fs::metadata(path)
        .ok()
        .and_then(|metadata| metadata.modified().ok())
        .and_then(system_time_to_unix_string)
}

fn unix_timestamp_string() -> String {
    system_time_to_unix_string(SystemTime::now()).unwrap_or_else(|| "0".to_string())
}

fn system_time_to_unix_string(time: SystemTime) -> Option<String> {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_secs().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "codex-switch-usage-history-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create temp root");
        path
    }

    #[test]
    fn usage_history_extracts_codex_token_deltas_without_message_content() {
        let root = temp_root("tokens");
        let session_dir = root.join("sessions/2026/06/24");
        fs::create_dir_all(&session_dir).expect("session dir");
        fs::write(
            session_dir.join("session.jsonl"),
            r#"{"type":"session_meta","payload":{"session_id":"session-1"}}
{"type":"turn_context","payload":{"model":"openai/gpt-5-2026-06-01"}}
{"timestamp":"2026-06-24T01:00:00Z","type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":10,"cached_input_tokens":2,"output_tokens":5}}}}
{"timestamp":"2026-06-24T01:01:00Z","type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":18,"cached_input_tokens":4,"output_tokens":9}}}}
"#,
        )
        .expect("write session");

        let report = load_usage_history_from_codex_home(&root);

        assert_eq!(report.files_scanned, 1);
        assert_eq!(report.sessions.len(), 1);
        assert_eq!(report.sessions[0].session_id, Some("session-1".to_string()));
        assert_eq!(report.sessions[0].model, "gpt-5");
        assert_eq!(report.sessions[0].token_events, 2);
        assert_eq!(report.sessions[0].tokens.input_tokens, 18);
        assert_eq!(report.sessions[0].tokens.cached_input_tokens, 4);
        assert_eq!(report.sessions[0].tokens.output_tokens, 9);
        assert_eq!(report.totals.total_tokens, 31);
        let encoded = serde_json::to_string(&report).expect("json");
        assert!(!encoded.contains("prompt"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn usage_history_extracts_latest_quota_windows() {
        let root = temp_root("quota");
        let session_dir = root.join("sessions/2026/06/24");
        fs::create_dir_all(&session_dir).expect("session dir");
        fs::write(
            session_dir.join("quota.jsonl"),
            r#"{"timestamp":"2026-06-24T01:00:00Z","type":"event_msg","payload":{"type":"token_count","rate_limits":{"limit_id":"codex","primary":{"used_percent":25,"resets_at":1800000000,"window_minutes":300},"secondary":{"used_percent":40,"resets_at":1800500000,"window_minutes":10080}},"info":{"last_token_usage":{"input_tokens":1,"cached_input_tokens":0,"output_tokens":2}}}}"#,
        )
        .expect("write quota");

        let report = load_usage_history_from_codex_home(&root);
        let quota = report.latest_quota.expect("quota");

        assert_eq!(quota.five_hour.remaining_percent, Some(75));
        assert_eq!(quota.weekly.remaining_percent, Some(60));
        assert_eq!(quota.five_hour.reset_at, Some("1800000000".to_string()));
        let _ = fs::remove_dir_all(root);
    }
}
