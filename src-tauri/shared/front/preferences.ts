import type { UsageStatsRangePreset, UsageStatsRefreshSeconds } from "@front-shared/types";

const ACCOUNT_DETAIL_STORAGE_KEY = "codex-switch-show-account-detail";
const USAGE_STATS_RANGE_STORAGE_KEY = "codex-switch-usage-stats-range";
const USAGE_STATS_REFRESH_SECONDS_STORAGE_KEY = "codex-switch-usage-stats-refresh-seconds";
const USAGE_STATS_CUSTOM_START_STORAGE_KEY = "codex-switch-usage-stats-custom-start";
const USAGE_STATS_CUSTOM_END_STORAGE_KEY = "codex-switch-usage-stats-custom-end";

const usageStatsRanges: readonly UsageStatsRangePreset[] = ["today", "1d", "7d", "14d", "30d", "custom"];
const usageStatsRefreshSeconds: readonly UsageStatsRefreshSeconds[] = [0, 5, 10, 30, 60];

export function resolveInitialShowAccountDetail(): boolean {
  const stored = globalThis.localStorage?.getItem(ACCOUNT_DETAIL_STORAGE_KEY);
  return stored === null ? true : stored !== "false";
}

export function persistShowAccountDetail(showDetail: boolean): void {
  globalThis.localStorage?.setItem(ACCOUNT_DETAIL_STORAGE_KEY, String(showDetail));
}

export function isUsageStatsRangePreset(value: unknown): value is UsageStatsRangePreset {
  return typeof value === "string" && usageStatsRanges.includes(value as UsageStatsRangePreset);
}

export function resolveInitialUsageStatsRange(): UsageStatsRangePreset {
  const stored = globalThis.localStorage?.getItem(USAGE_STATS_RANGE_STORAGE_KEY);
  return isUsageStatsRangePreset(stored) ? stored : "today";
}

export function persistUsageStatsRange(range: UsageStatsRangePreset): void {
  globalThis.localStorage?.setItem(USAGE_STATS_RANGE_STORAGE_KEY, range);
}

function parseStoredTimestamp(value: string | null): number | null {
  if (!value) {
    return null;
  }
  const timestamp = Number(value);
  return Number.isFinite(timestamp) && timestamp > 0 ? Math.floor(timestamp) : null;
}

export function resolveInitialUsageStatsCustomRange(): { startAt: number | null; endAt: number | null } {
  return {
    startAt: parseStoredTimestamp(globalThis.localStorage?.getItem(USAGE_STATS_CUSTOM_START_STORAGE_KEY) ?? null),
    endAt: parseStoredTimestamp(globalThis.localStorage?.getItem(USAGE_STATS_CUSTOM_END_STORAGE_KEY) ?? null),
  };
}

export function persistUsageStatsCustomRange(startAt: number | null, endAt: number | null): void {
  if (startAt) {
    globalThis.localStorage?.setItem(USAGE_STATS_CUSTOM_START_STORAGE_KEY, String(startAt));
  } else {
    globalThis.localStorage?.removeItem(USAGE_STATS_CUSTOM_START_STORAGE_KEY);
  }
  if (endAt) {
    globalThis.localStorage?.setItem(USAGE_STATS_CUSTOM_END_STORAGE_KEY, String(endAt));
  } else {
    globalThis.localStorage?.removeItem(USAGE_STATS_CUSTOM_END_STORAGE_KEY);
  }
}

export function normalizeUsageStatsRefreshSeconds(value: unknown): UsageStatsRefreshSeconds {
  const parsed = typeof value === "number" ? value : Number(value);
  return usageStatsRefreshSeconds.includes(parsed as UsageStatsRefreshSeconds)
    ? parsed as UsageStatsRefreshSeconds
    : 30;
}

export function resolveInitialUsageStatsRefreshSeconds(): UsageStatsRefreshSeconds {
  return normalizeUsageStatsRefreshSeconds(
    globalThis.localStorage?.getItem(USAGE_STATS_REFRESH_SECONDS_STORAGE_KEY) ?? "30",
  );
}

export function persistUsageStatsRefreshSeconds(seconds: UsageStatsRefreshSeconds): void {
  globalThis.localStorage?.setItem(USAGE_STATS_REFRESH_SECONDS_STORAGE_KEY, String(seconds));
}
