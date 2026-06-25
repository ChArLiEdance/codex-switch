import type { ProfilesSnapshotResponse, QuotaSummary, ShellRoute } from "@front-shared/types";
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
};
