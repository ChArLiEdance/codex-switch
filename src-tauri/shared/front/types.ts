export interface QuotaWindow {
  remaining_percent: number | null;
  refresh_at: string | null;
  reset_at_timestamp: number | null;
}

export interface QuotaSummary {
  five_hour: QuotaWindow;
  weekly: QuotaWindow;
}

export interface ProfileCard {
  folder_name: string;
  display_title: string;
  account_label: string | null;
  status: "current" | "available" | "missing_auth";
  auth_present: boolean;
  has_account_identity: boolean;
  plan_name: string | null;
  subscription_days_left: number | null;
  openai_base_url: string | null;
  quota: QuotaSummary;
  last_plan_check_ms: number | null;
}

export interface CurrentCard {
  folder_name: string;
  display_title: string;
  account_label: string | null;
  has_account_identity: boolean;
  plan_name: string | null;
  subscription_days_left: number | null;
  profile_folder_path: string;
  last_plan_check_ms: number | null;
}

export interface PagingInfo {
  page: number;
  page_size: number;
  total_profiles: number;
  total_pages: number;
  has_previous: boolean;
  has_next: boolean;
}

export interface DashboardViewModel {
  paging: PagingInfo;
  profiles: ProfileCard[];
  current_card: CurrentCard | null;
  current_quota_card: QuotaSummary | null;
}

export interface ProfilesSnapshotResponse {
  page_size: number;
  profiles: ProfileCard[];
  current_card: CurrentCard | null;
  current_quota_card: QuotaSummary | null;
  /** Label of the live `~/.codex` account when it belongs to no saved card
   *  (drift to an unmanaged account); `null` in the normal case. */
  unmanaged_live_account: string | null;
}

export interface CurrentQuotaResponse {
  profile: string | null;
  quota: QuotaSummary | null;
}

export interface SwitchResponse {
  ok: boolean;
  profile: string;
  message: string;
  warnings: string[];
}

export interface SwitchRestartTargets {
  cli: boolean;
  vscode: boolean;
  codex_desktop: boolean;
}

export type CloseBehavior = "ask" | "hide" | "quit";

export interface TrayProfileEntry {
  folder_name: string;
  display_title: string;
  nickname: string;
  plan_name: string | null;
  quota: QuotaSummary;
  status: string;
  auth_present: boolean;
}

export interface TrayStatePayload {
  locale: string;
  current_profile: string | null;
  current_title: string | null;
  current_quota: QuotaSummary | null;
  profiles: TrayProfileEntry[];
  restart_targets: SwitchRestartTargets;
}

export interface ActionResponse {
  ok: boolean;
  message: string;
  path: string | null;
}

export interface UpdateCheckResponse {
  ok: boolean;
  current_version: string;
  latest_version: string | null;
  has_update: boolean;
  release_url: string | null;
  notes: string | null;
  checked_url: string;
}

export interface CommandError {
  error_code?: string;
  message?: string;
}

export type CodexCliSource = "user_override" | "install_state" | "discovery" | "none";

export interface CodexCliStatus {
  resolved_path: string | null;
  source: CodexCliSource;
  suggested_paths: string[];
}

export interface CodexCliCandidate {
  /** Absolute path to the verified-runnable codex binary. */
  path: string;
  /** `codex --version` line (e.g. "codex-cli 0.133.0"), or null if it ran but printed nothing parseable. */
  version: string | null;
}

export interface CodexCliRedetectResult {
  /** Candidates verified runnable by the forced scan, deduped, best-first. */
  candidates: CodexCliCandidate[];
  /** Refreshed status snapshot so the Settings row can update in step. */
  status: CodexCliStatus;
}

export type ShellRoute = "dashboard" | "profiles" | "settings" | "guide" | "skills" | "prompts" | "history";

export interface UsageQuerySettings {
  enabled: boolean;
  timeout_seconds: number;
  auto_query_interval_minutes: number;
}

export interface UsageStatsPayload {
  profile: string | null;
  start_at: number | null;
  end_at: number | null;
}

export type UsageStatsRangePreset = "today" | "1d" | "7d" | "14d" | "30d" | "custom";

export type UsageStatsRefreshSeconds = 0 | 5 | 10 | 30 | 60;

export interface UsageTotals {
  request_count: number;
  input_tokens: number;
  output_tokens: number;
  cache_read_tokens: number;
  cache_creation_tokens: number;
  real_total_tokens: number;
  total_cost_usd: number;
  cache_hit_rate: number;
}

export interface UsageTrendPoint {
  bucket: string;
  timestamp: number;
  input_tokens: number;
  output_tokens: number;
  cache_read_tokens: number;
  cache_creation_tokens: number;
  real_total_tokens: number;
  total_cost_usd: number;
}

export interface UsageSessionRow {
  profile: string;
  session_id: string;
  model: string;
  started_at: number;
  input_tokens: number;
  output_tokens: number;
  cache_read_tokens: number;
  cache_creation_tokens: number;
  real_total_tokens: number;
  total_cost_usd: number;
}

export interface UsageProfileOption {
  folder_name: string;
  display_title: string;
}

export interface UsageStatsResponse {
  profiles: UsageProfileOption[];
  selected_profile: string | null;
  start_at: number;
  end_at: number;
  totals: UsageTotals;
  trends: UsageTrendPoint[];
  sessions: UsageSessionRow[];
}

export interface CodexSessionMeta {
  session_id: string;
  title: string | null;
  summary: string | null;
  project_dir: string | null;
  created_at: number | null;
  last_active_at: number | null;
  source_path: string;
  resume_command: string;
  profile: string | null;
}

export interface CodexSessionMessage {
  role: string;
  content: string;
  ts: number | null;
}
