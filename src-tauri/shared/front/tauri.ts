import { invoke } from "@tauri-apps/api/core";

import type {
  ActionResponse,
  CodexCliRedetectResult,
  CodexCliStatus,
  CodexSessionMessage,
  CodexSessionMeta,
  CommandError,
  CurrentCard,
  CurrentQuotaResponse,
  ProfileCard,
  ProfilesSnapshotResponse,
  QuotaSummary,
  SwitchRestartTargets,
  SwitchResponse,
  TrayStatePayload,
  UpdateCheckResponse,
  UsageQuerySettings,
  UsageStatsPayload,
  UsageStatsResponse,
} from "@front-shared/types";

type NativeCommandError = Error & {
  code?: string;
};

type RuntimeWindow = typeof globalThis & {
  __TAURI_INTERNALS__?: unknown;
  __TAURI__?: unknown;
};

const hasTauriRuntime = Boolean(
  (globalThis as RuntimeWindow).__TAURI_INTERNALS__ || (globalThis as RuntimeWindow).__TAURI__,
);
const usePreviewMocks = __CODEX_PREVIEW_MOCKS__ || !hasTauriRuntime;

function clone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

function quota(
  fiveHourPercent: number | null,
  fiveHourRefresh: string | null,
  weeklyPercent: number | null,
  weeklyRefresh: string | null,
): QuotaSummary {
  return {
    five_hour: {
      remaining_percent: fiveHourPercent,
      refresh_at: fiveHourRefresh,
      reset_at_timestamp: null,
    },
    weekly: {
      remaining_percent: weeklyPercent,
      refresh_at: weeklyRefresh,
      reset_at_timestamp: null,
    },
  };
}

const previewProfiles: ProfileCard[] = [
  {
    folder_name: "workspace-alpha",
    display_title: "Workspace Alpha",
    account_label: "Workspace Alpha",
    status: "available",
    auth_present: true,
    has_account_identity: true,
    plan_name: "Pro plan",
    subscription_days_left: 18,
    openai_base_url: null,
    quota: quota(84, "3小时后刷新", 61, "2天4小时后刷新"),
    last_plan_check_ms: Date.now() - 30 * 60 * 1000,
  },
  {
    folder_name: "workspace-beta",
    display_title: "Workspace Beta",
    account_label: "Workspace Beta",
    status: "available",
    auth_present: true,
    has_account_identity: true,
    plan_name: "Plus plan",
    subscription_days_left: 12,
    openai_base_url: null,
    quota: quota(58, "1小时后刷新", 42, "4天6小时后刷新"),
    last_plan_check_ms: Date.now() - 4 * 60 * 60 * 1000,
  },
  {
    folder_name: "workspace-gamma",
    display_title: "Metadata Not Configured",
    account_label: null,
    status: "missing_auth",
    auth_present: false,
    has_account_identity: false,
    plan_name: null,
    subscription_days_left: null,
    openai_base_url: null,
    quota: quota(null, "6小时后刷新", null, "7天2小时后刷新"),
    last_plan_check_ms: null,
  },
  {
    folder_name: "workspace-delta",
    display_title: "Workspace Delta",
    account_label: "Workspace Delta",
    status: "available",
    auth_present: true,
    has_account_identity: true,
    plan_name: "Custom endpoint",
    subscription_days_left: 31,
    openai_base_url: "https://example.com/v1",
    quota: quota(73, "2小时后刷新", 88, "1天9小时后刷新"),
    last_plan_check_ms: Date.now() - 8 * 24 * 60 * 60 * 1000,
  },
  {
    folder_name: "workspace-epsilon",
    display_title: "Workspace Epsilon",
    account_label: "Workspace Epsilon",
    status: "available",
    auth_present: true,
    has_account_identity: true,
    plan_name: "Pro plan",
    subscription_days_left: 18,
    openai_base_url: null,
    quota: quota(84, "3小时后刷新", 61, "2天4小时后刷新"),
    last_plan_check_ms: Date.now() - 2 * 24 * 60 * 60 * 1000,
  },
  {
    folder_name: "workspace-zeta",
    display_title: "Workspace Zeta",
    account_label: "Workspace Zeta",
    status: "available",
    auth_present: true,
    has_account_identity: true,
    plan_name: "Custom endpoint",
    subscription_days_left: 31,
    openai_base_url: "https://example.com/v1",
    quota: quota(73, "2小时后刷新", 88, "1天9小时后刷新"),
    last_plan_check_ms: null,
  },
];

let previewCurrentCard: CurrentCard = {
  folder_name: "workspace-alpha",
  display_title: "Workspace Alpha",
  account_label: "Workspace Alpha",
  has_account_identity: true,
  plan_name: "Pro plan",
  subscription_days_left: 18,
  profile_folder_path: "/mock/workspace-alpha",
  last_plan_check_ms: Date.now() - 30 * 60 * 1000,
};

let previewCurrentQuota: QuotaSummary = quota(84, "3小时后刷新", 61, "2天4小时后刷新");

let previewSnapshot: ProfilesSnapshotResponse = {
  page_size: 8,
  profiles: clone(previewProfiles),
  current_card: clone(previewCurrentCard),
  current_quota_card: clone(previewCurrentQuota),
  unmanaged_live_account: null,
};

function mockAction(message: string, path: string | null = null): Promise<ActionResponse> {
  return Promise.resolve({
    ok: true,
    message,
    path,
  });
}

const previewUsageSettings = new Map<string, UsageQuerySettings>();

const previewCodexSessions: CodexSessionMeta[] = [
  {
    session_id: "019ef3ca-10c4-7973-a7b8-bfd082573ca4",
    title: "<codex_internal_context source=\"goal\"> Continue working on Codex Switch",
    summary: "Refine the desktop app session manager and account switching UI.",
    project_dir: "/Users/charlie/Documents/CharlieCode/codex_switch",
    created_at: Math.floor(Date.now() / 1000) - 240,
    last_active_at: Math.floor(Date.now() / 1000) - 42,
    source_path: "/preview/sessions/019ef3ca-10c4-7973-a7b8-bfd082573ca4.jsonl",
    resume_command: "codex resume 019ef3ca-10c4-7973-a7b8-bfd082573ca4",
    profile: "workspace-alpha",
  },
  {
    session_id: "01-preview-usage-stats",
    title: "Use CCSwitch layout for session history",
    summary: "Build a split session list and detail reader with local Codex JSONL data.",
    project_dir: "/Users/charlie/Documents/CharlieCode",
    created_at: Math.floor(Date.now() / 1000) - 14_400,
    last_active_at: Math.floor(Date.now() / 1000) - 13_800,
    source_path: "/preview/sessions/01-preview-usage-stats.jsonl",
    resume_command: "codex resume 01-preview-usage-stats",
    profile: "workspace-beta",
  },
];

const previewCodexMessages: CodexSessionMessage[] = [
  {
    role: "developer",
    content: "<permissions instructions>\nFilesystem sandboxing defines which files can be read or written.\nNetwork access is enabled.",
    ts: Math.floor(Date.now() / 1000) - 220,
  },
  {
    role: "user",
    content: "会话记录参考ccswitch做成这个样子的",
    ts: Math.floor(Date.now() / 1000) - 180,
  },
  {
    role: "assistant",
    content: "我会把会话记录做成左侧列表、右侧详情，并从本地 Codex 会话 JSONL 中读取记录。",
    ts: Math.floor(Date.now() / 1000) - 120,
  },
];

function defaultUsageSettings(): UsageQuerySettings {
  return {
    enabled: false,
    timeout_seconds: 10,
    auto_query_interval_minutes: 5,
  };
}

function previewCost(model: string, input: number, output: number, cacheRead: number): number {
  const normalized = model.toLowerCase();
  let inputPerM = 1;
  let outputPerM = 4;
  let cachePerM = 0.25;
  if (normalized.startsWith("gpt-5.5")) {
    inputPerM = 5;
    outputPerM = 30;
    cachePerM = 0.5;
  } else if (normalized.startsWith("gpt-5.4")) {
    inputPerM = 2.5;
    outputPerM = 15;
    cachePerM = 0.25;
  } else if (normalized.startsWith("gpt-5.3-codex") || normalized.startsWith("gpt-5.2")) {
    inputPerM = 1.75;
    outputPerM = 14;
    cachePerM = 0.175;
  } else if (normalized.startsWith("gpt-5.1") || normalized.startsWith("gpt-5")) {
    inputPerM = 1.25;
    outputPerM = 10;
    cachePerM = 0.125;
  } else if (normalized.startsWith("gpt-4.1") || normalized.startsWith("o3")) {
    inputPerM = 2;
    outputPerM = 8;
    cachePerM = 0.5;
  }
  return ((Math.max(0, input - cacheRead) * inputPerM) + (output * outputPerM) + (cacheRead * cachePerM)) / 1_000_000;
}

function makePreviewUsageStats(payload: UsageStatsPayload | undefined): UsageStatsResponse {
  const now = Math.floor(Date.now() / 1000);
  const start = payload?.start_at ?? now - 24 * 60 * 60;
  const end = payload?.end_at ?? now;
  const profiles = previewSnapshot.profiles.map((profile) => ({
    folder_name: profile.folder_name,
    display_title: profile.account_label ?? profile.display_title ?? profile.folder_name,
  }));
  const selected = payload?.profile ?? null;
  const scale = selected ? 1 : Math.max(1, profiles.length);
  const trends = Array.from({ length: 8 }, (_, index) => {
    const timestamp = start + Math.floor(((end - start) * index) / 7);
    const input = index < 4 ? 0 : (index - 3) * 420000 * scale;
    const output = index < 4 ? 0 : (index - 3) * 12000 * scale;
    const cache = index < 4 ? 0 : (index - 3) * 5100000 * scale;
    const model = index % 2 === 0 ? "gpt-5.4" : "gpt-5.1-codex";
    return {
      bucket: new Date(timestamp * 1000).toLocaleString(),
      timestamp,
      input_tokens: input,
      output_tokens: output,
      cache_read_tokens: cache,
      cache_creation_tokens: 0,
      real_total_tokens: input + output + cache,
      total_cost_usd: previewCost(model, input, output, cache),
    };
  });
  const totals = trends.reduce(
    (acc, point) => ({
      request_count: acc.request_count + (point.real_total_tokens > 0 ? 28 : 0),
      input_tokens: acc.input_tokens + point.input_tokens,
      output_tokens: acc.output_tokens + point.output_tokens,
      cache_read_tokens: acc.cache_read_tokens + point.cache_read_tokens,
      cache_creation_tokens: 0,
      real_total_tokens: acc.real_total_tokens + point.real_total_tokens,
      total_cost_usd: acc.total_cost_usd + point.total_cost_usd,
      cache_hit_rate: 0,
    }),
    {
      request_count: 0,
      input_tokens: 0,
      output_tokens: 0,
      cache_read_tokens: 0,
      cache_creation_tokens: 0,
      real_total_tokens: 0,
      total_cost_usd: 0,
      cache_hit_rate: 0,
    },
  );
  const cacheable = totals.input_tokens + totals.cache_read_tokens;
  totals.cache_hit_rate = cacheable > 0 ? totals.cache_read_tokens / cacheable : 0;
  return {
    profiles,
    selected_profile: selected,
    start_at: start,
    end_at: end,
    totals,
    trends,
    sessions: profiles.slice(0, selected ? 1 : 4).map((profile, index) => ({
      profile: profile.folder_name,
      session_id: `preview-session-${index + 1}`,
      model: index % 2 === 0 ? "gpt-5" : "gpt-4.1",
      started_at: end - index * 3600,
      input_tokens: 64000 * (index + 1),
      output_tokens: 3200 * (index + 1),
      cache_read_tokens: 880000 * (index + 1),
      cache_creation_tokens: 0,
      real_total_tokens: 947200 * (index + 1),
      total_cost_usd: 0.42 * (index + 1),
    })),
  };
}

function refreshPreviewSnapshot(): void {
  previewSnapshot = {
    page_size: 8,
    profiles: clone(previewSnapshot.profiles),
    current_card: clone(previewCurrentCard),
    current_quota_card: clone(previewCurrentQuota),
    unmanaged_live_account: null,
  };
}

function toError(error: unknown): Error {
  if (typeof error === "string") {
    return new Error(error);
  }

  if (error && typeof error === "object") {
    const payload = error as CommandError;
    if (payload.message || payload.error_code) {
      const nextError = new Error(payload.message || "Unknown native command error.") as NativeCommandError;
      nextError.code = payload.error_code;
      return nextError;
    }
  }

  return new Error("Unknown native command error.");
}

async function invokeCommand<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (usePreviewMocks) {
    switch (command) {
      case "get_profiles_snapshot":
        return clone(previewSnapshot) as T;
      case "get_current_live_quota":
      case "refresh_active_profile_quota_silent":
        return {
          profile: previewCurrentCard.folder_name,
          quota: clone(previewCurrentQuota),
        } as T;
      case "refresh_all_oauth_profile_plans_silent":
        return 0 as T;
      case "get_usage_stats":
        return makePreviewUsageStats(args?.payload as UsageStatsPayload | undefined) as T;
      case "list_codex_sessions":
        return clone(previewCodexSessions) as T;
      case "get_codex_session_messages":
        return clone(previewCodexMessages) as T;
      case "get_usage_query_settings": {
        const profile = (args?.profile as string | undefined) ?? "";
        return clone(previewUsageSettings.get(profile) ?? defaultUsageSettings()) as T;
      }
      case "save_usage_query_settings": {
        const payload = args?.payload as { profile?: string; settings?: UsageQuerySettings } | undefined;
        if (payload?.profile && payload.settings) {
          previewUsageSettings.set(payload.profile, clone(payload.settings));
          return clone(payload.settings) as T;
        }
        return defaultUsageSettings() as T;
      }
      case "switch_profile": {
        const profile = (args?.payload as { profile?: string } | undefined)?.profile ?? previewCurrentCard.folder_name;
        const next = previewSnapshot.profiles.find((entry) => entry.folder_name === profile);
        if (next) {
          previewCurrentCard = {
            folder_name: next.folder_name,
            display_title: next.display_title,
            account_label: next.account_label,
            has_account_identity: next.has_account_identity,
            plan_name: next.plan_name,
            subscription_days_left: next.subscription_days_left,
            profile_folder_path: `C:/mock/${next.folder_name}`,
            last_plan_check_ms: next.last_plan_check_ms,
          };
          previewCurrentQuota = clone(next.quota);
          refreshPreviewSnapshot();
        }
        return {
          ok: true,
          profile,
          message: "Switched in preview mode",
          warnings: [],
        } as T;
      }
      case "rename_profile": {
        const payload = args?.payload as { profile?: string; new_folder_name?: string } | undefined;
        if (payload?.profile && payload.new_folder_name) {
          const sourceProfile = payload.profile;
          const nextFolderName = payload.new_folder_name;
          previewSnapshot.profiles = previewSnapshot.profiles.map((entry) =>
            entry.folder_name === sourceProfile
              ? {
                  ...entry,
                  folder_name: nextFolderName,
                  display_title: nextFolderName,
                  account_label: nextFolderName,
                }
              : entry,
          );
          if (previewCurrentCard.folder_name === sourceProfile) {
            previewCurrentCard = {
              ...previewCurrentCard,
              folder_name: nextFolderName,
              display_title: nextFolderName,
              account_label: nextFolderName,
            };
          }
          refreshPreviewSnapshot();
        }
        return mockAction("Renamed in preview mode") as Promise<T>;
      }
      case "add_profile": {
        const payload = args?.payload as { folder_name?: string; openai_base_url?: string | null } | undefined;
        if (payload?.folder_name) {
          previewSnapshot.profiles.push({
            folder_name: payload.folder_name,
            display_title: payload.folder_name,
            account_label: payload.folder_name,
            status: "available",
            auth_present: true,
            has_account_identity: true,
            plan_name: "Pro plan",
            subscription_days_left: 30,
            openai_base_url: payload.openai_base_url ?? null,
            quota: quota(52, "5小时后刷新", 67, "3天后刷新"),
            last_plan_check_ms: Date.now(),
          });
          refreshPreviewSnapshot();
        }
        return mockAction("Added in preview mode") as Promise<T>;
      }
      case "update_profile_base_url": {
        const payload = args?.payload as { profile?: string; openai_base_url?: string } | undefined;
        if (payload?.profile) {
          previewSnapshot.profiles = previewSnapshot.profiles.map((entry) =>
            entry.folder_name === payload.profile
              ? { ...entry, openai_base_url: payload.openai_base_url ?? null }
              : entry,
          );
          refreshPreviewSnapshot();
        }
        return mockAction("Base URL updated in preview mode") as Promise<T>;
      }
      case "delete_profile": {
        const payload = args?.payload as { profile?: string } | undefined;
        if (payload?.profile) {
          previewSnapshot.profiles = previewSnapshot.profiles.filter(
            (entry) => entry.folder_name !== payload.profile,
          );
          refreshPreviewSnapshot();
        }
        return mockAction("Deleted in preview mode") as Promise<T>;
      }
      case "clear_profile_account": {
        const payload = args?.payload as { profile?: string } | undefined;
        if (payload?.profile) {
          previewSnapshot.profiles = previewSnapshot.profiles.map((entry) =>
            entry.folder_name === payload.profile
              ? {
                  ...entry,
                  account_label: null,
                  display_title: entry.folder_name,
                  status: "missing_auth",
                  auth_present: false,
                  has_account_identity: false,
                  plan_name: null,
                  subscription_days_left: null,
                  openai_base_url: null,
                  quota: quota(null, null, null, null),
                }
              : entry,
          );
          refreshPreviewSnapshot();
        }
        return mockAction("Cleared in preview mode") as Promise<T>;
      }
      case "check_update":
        return Promise.resolve({
          ok: true,
          current_version: "1.0.0",
          latest_version: "1.0.0",
          has_update: false,
          release_url: "https://github.com/ChArLiEdance/codex-switch/releases",
          notes: null,
          checked_url: "preview",
        }) as Promise<T>;
      case "open_url":
        return mockAction("Opened URL in preview mode", "preview:url") as Promise<T>;
      case "get_codex_cli_status":
      case "set_codex_cli_path":
      case "clear_codex_cli_path":
        return Promise.resolve({
          resolved_path: "/preview/codex",
          source: command === "set_codex_cli_path" ? "user_override" : "discovery",
          suggested_paths: ["/preview/codex", "/preview/usr/local/bin/codex"],
        }) as Promise<T>;
      case "redetect_codex_cli_path":
        return Promise.resolve({
          candidates: [{ path: "/preview/codex", version: "codex-cli 0.133.0" }],
          status: {
            resolved_path: "/preview/codex",
            source: "user_override",
            suggested_paths: ["/preview/codex", "/preview/usr/local/bin/codex"],
          },
        }) as Promise<T>;
      case "cancel_codex_login":
        return Promise.resolve(true) as Promise<T>;
      case "sync_tray_state":
      case "show_main_window":
      case "hide_main_window":
      case "quit_app":
      case "open_profile_folder":
      case "open_codex":
      case "login_current_profile":
      case "login_profile":
      case "refresh_profile":
      case "open_releases":
      case "open_contact":
      case "open_xiaohongshu":
        return mockAction(`${command} completed in preview mode`) as Promise<T>;
      default:
        return Promise.reject(new Error(`Unsupported preview command: ${command}`));
    }
  }

  try {
    return await invoke<T>(command, args);
  } catch (error) {
    throw toError(error);
  }
}

export function getProfilesSnapshot(): Promise<ProfilesSnapshotResponse> {
  return invokeCommand<ProfilesSnapshotResponse>("get_profiles_snapshot");
}

export function getCurrentLiveQuota(): Promise<CurrentQuotaResponse> {
  return invokeCommand<CurrentQuotaResponse>("get_current_live_quota");
}

export function refreshActiveProfileQuotaSilent(): Promise<CurrentQuotaResponse> {
  return invokeCommand<CurrentQuotaResponse>("refresh_active_profile_quota_silent");
}

export function refreshAllOauthProfilePlansSilent(): Promise<number> {
  return invokeCommand<number>("refresh_all_oauth_profile_plans_silent");
}

export function getUsageStats(payload: UsageStatsPayload): Promise<UsageStatsResponse> {
  return invokeCommand<UsageStatsResponse>("get_usage_stats", { payload });
}

export function listCodexSessions(): Promise<CodexSessionMeta[]> {
  return invokeCommand<CodexSessionMeta[]>("list_codex_sessions");
}

export function getCodexSessionMessages(sourcePath: string): Promise<CodexSessionMessage[]> {
  return invokeCommand<CodexSessionMessage[]>("get_codex_session_messages", {
    sourcePath,
  });
}

export function getUsageQuerySettings(profile: string): Promise<UsageQuerySettings> {
  return invokeCommand<UsageQuerySettings>("get_usage_query_settings", { profile });
}

export function saveUsageQuerySettings(
  profile: string,
  settings: UsageQuerySettings,
): Promise<UsageQuerySettings> {
  return invokeCommand<UsageQuerySettings>("save_usage_query_settings", {
    payload: { profile, settings },
  });
}

export function switchProfile(
  profile: string,
  restartTargets?: SwitchRestartTargets,
): Promise<SwitchResponse> {
  return invokeCommand<SwitchResponse>("switch_profile", {
    payload: { profile, restart_targets: restartTargets ?? null },
  });
}

export function openProfileFolder(profile: string): Promise<ActionResponse> {
  return invokeCommand<ActionResponse>("open_profile_folder", { payload: { profile } });
}

export function addProfile(folderName: string, openaiBaseUrl: string | null): Promise<ActionResponse> {
  return invokeCommand<ActionResponse>("add_profile", {
    payload: {
      folder_name: folderName,
      openai_base_url: openaiBaseUrl,
    },
  });
}

export function openCodex(): Promise<ActionResponse> {
  return invokeCommand<ActionResponse>("open_codex");
}

export function loginCurrentProfile(): Promise<ActionResponse> {
  return invokeCommand<ActionResponse>("login_current_profile");
}

export function loginProfile(profile: string): Promise<ActionResponse> {
  return invokeCommand<ActionResponse>("login_profile", { payload: { profile } });
}

export function refreshProfile(profile: string): Promise<ActionResponse> {
  return invokeCommand<ActionResponse>("refresh_profile", { payload: { profile } });
}

export function renameProfile(profile: string, newFolderName: string): Promise<ActionResponse> {
  return invokeCommand<ActionResponse>("rename_profile", {
    payload: { profile, new_folder_name: newFolderName },
  });
}

export function updateProfileBaseUrl(profile: string, openaiBaseUrl: string): Promise<ActionResponse> {
  return invokeCommand<ActionResponse>("update_profile_base_url", {
    payload: { profile, openai_base_url: openaiBaseUrl },
  });
}

export function deleteProfile(profile: string): Promise<ActionResponse> {
  return invokeCommand<ActionResponse>("delete_profile", { payload: { profile } });
}

export function clearProfileAccount(profile: string): Promise<ActionResponse> {
  return invokeCommand<ActionResponse>("clear_profile_account", { payload: { profile } });
}

export function openContact(): Promise<ActionResponse> {
  return invokeCommand<ActionResponse>("open_contact");
}

export function openReleases(): Promise<ActionResponse> {
  return invokeCommand<ActionResponse>("open_releases");
}

export function openUrl(url: string): Promise<ActionResponse> {
  return invokeCommand<ActionResponse>("open_url", {
    payload: { url },
  });
}

export function checkUpdate(updateUrl: string): Promise<UpdateCheckResponse> {
  return invokeCommand<UpdateCheckResponse>("check_update", {
    payload: { update_url: updateUrl },
  });
}

export function openXiaohongshu(): Promise<ActionResponse> {
  return invokeCommand<ActionResponse>("open_xiaohongshu");
}

export function getCodexCliStatus(): Promise<CodexCliStatus> {
  return invokeCommand<CodexCliStatus>("get_codex_cli_status");
}

export function setCodexCliPath(path: string): Promise<CodexCliStatus> {
  return invokeCommand<CodexCliStatus>("set_codex_cli_path", {
    payload: { path },
  });
}

export function clearCodexCliPath(): Promise<CodexCliStatus> {
  return invokeCommand<CodexCliStatus>("clear_codex_cli_path");
}

export function redetectCodexCliPath(): Promise<CodexCliRedetectResult> {
  return invokeCommand<CodexCliRedetectResult>("redetect_codex_cli_path");
}

export function cancelCodexLogin(): Promise<boolean> {
  return invokeCommand<boolean>("cancel_codex_login");
}

export function syncTrayState(payload: TrayStatePayload): Promise<ActionResponse> {
  return invokeCommand<ActionResponse>("sync_tray_state", { payload });
}

export function showMainWindow(): Promise<ActionResponse> {
  return invokeCommand<ActionResponse>("show_main_window");
}

export function hideMainWindow(): Promise<ActionResponse> {
  return invokeCommand<ActionResponse>("hide_main_window");
}

export function quitApp(): Promise<ActionResponse> {
  return invokeCommand<ActionResponse>("quit_app");
}
