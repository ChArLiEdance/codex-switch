import type {
  ProfilesSnapshotResponse,
  QuotaSummary,
  ShellRoute,
  UsageQuerySettings,
  UsageStatsResponse,
} from "@front-shared/types";
import type { Locale } from "@front-shared/i18n";
import type { ThemeId } from "@front-shared/theme";
import { resolveInitialShowAccountDetail } from "@front-shared/preferences";

export const state = {
  page: 1,
  loading: false,
  refreshActiveProfiles: [] as string[],
  loginActiveProfile: null as string | null,
  reloginProfiles: [] as string[],
  currentProfile: null as string | null,
  expandedQuotaProfiles: [] as string[],
  route: "dashboard" as ShellRoute,
  locale: "en" as Locale,
  theme: "light" as ThemeId,
  showAccountDetail: resolveInitialShowAccountDetail(),
  pageSize: 8,
  snapshot: null as ProfilesSnapshotResponse | null,
  currentQuota: null as QuotaSummary | null,
  usageStats: null as UsageStatsResponse | null,
  usageStatsProfile: null as string | null,
  usageStatsRange: "today" as "today" | "7d" | "30d",
  historyStats: null as UsageStatsResponse | null,
  historyStatsProfile: null as string | null,
  historyStatsRange: "today" as "today" | "7d" | "30d",
  settingsUsageProfile: null as string | null,
  usageSettingsByProfile: {} as Record<string, UsageQuerySettings>,
};
