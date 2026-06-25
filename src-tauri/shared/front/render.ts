import type {
  CodexSessionMessage,
  CodexSessionMeta,
  CurrentCard,
  DashboardViewModel,
  PagingInfo,
  ProfileCard,
  QuotaSummary,
  QuotaWindow,
  ShellRoute,
  UpdateCheckResponse,
  UsageStatsResponse,
} from "@front-shared/types";
import { t, type MessageKey } from "@front-shared/i18n";
import { state } from "@front-shared/state";
import { getThemeOption, isThemeId } from "@front-shared/theme";

const isWindowsUiTarget = __CODEX_UI_TARGET__ === "windows";
const defaultRoute: ShellRoute = isWindowsUiTarget ? "dashboard" : "profiles";
const shellRoutes: readonly ShellRoute[] = [
  "dashboard",
  "profiles",
  "settings",
  "guide",
  "skills",
  "prompts",
  "history",
];

function isShellRoute(value: string): value is ShellRoute {
  return shellRoutes.includes(value as ShellRoute);
}

export function routeFromLocation(): ShellRoute {
  const hash = window.location.hash.replace(/^#/, "");
  return isShellRoute(hash) ? hash : defaultRoute;
}

function requiredElement<T extends HTMLElement>(id: string): T {
  const element = document.getElementById(id);
  if (!(element instanceof HTMLElement)) {
    throw new Error(`Missing required element: ${id}`);
  }
  return element as T;
}

const hasDeleteProfileUi = document.getElementById("delete-profile-dialog") instanceof HTMLDialogElement;
const macIconBase = "/ccswitch-icons";

function macIcon(name: string, className = "cc-icon"): string {
  return `<img class="${className}" src="${macIconBase}/${name}.svg" alt="" aria-hidden="true" />`;
}

export const elements = {
  profilesHeading: requiredElement<HTMLHeadingElement>("profiles-heading"),
  profilesGrid: requiredElement<HTMLDivElement>("profiles-grid"),
  pageIndicator: requiredElement<HTMLSpanElement>("page-indicator"),
  previousPageButton: requiredElement<HTMLButtonElement>("previous-page-button"),
  nextPageButton: requiredElement<HTMLButtonElement>("next-page-button"),
  currentSectionHeading: requiredElement<HTMLHeadingElement>("current-section-heading"),
  currentTitle: requiredElement<HTMLHeadingElement>("current-title"),
  currentPlan: requiredElement<HTMLParagraphElement>("current-plan"),
  currentQuotaPanel: requiredElement<HTMLDivElement>("current-quota-panel"),
  currentLoginButton: requiredElement<HTMLButtonElement>("current-login-button"),
  openCurrentFolderButton: requiredElement<HTMLButtonElement>("open-current-folder-button"),
  controlDeckHeading: requiredElement<HTMLHeadingElement>("control-deck-heading"),
  addProfilesButton: requiredElement<HTMLButtonElement>("add-profiles-button"),
  openCodexButton: requiredElement<HTMLButtonElement>("open-codex-button"),
  settingsGithubButton: requiredElement<HTMLButtonElement>("settings-github-button"),
  settingsCheckUpdateButton: requiredElement<HTMLButtonElement>("settings-check-update-button"),
  settingsUpdateUrlInput: requiredElement<HTMLInputElement>("settings-update-url-input"),
  settingsVersionValue: requiredElement<HTMLSpanElement>("settings-version-value"),
  settingsUsageProfileSelect: document.getElementById("settings-usage-profile-select") as HTMLSelectElement | null,
  settingsUsageEnabledToggle: document.getElementById("settings-usage-enabled-toggle") as HTMLInputElement | null,
  settingsUsageTimeoutInput: document.getElementById("settings-usage-timeout-input") as HTMLInputElement | null,
  settingsUsageIntervalInput: document.getElementById("settings-usage-interval-input") as HTMLInputElement | null,
  settingsUsageSaveButton: document.getElementById("settings-usage-save-button") as HTMLButtonElement | null,
  usageProfileFilter: document.getElementById("usage-profile-filter") as HTMLSelectElement | null,
  usageRangeFilter: document.getElementById("usage-range-filter") as HTMLSelectElement | null,
  usageRefreshButton: document.getElementById("usage-refresh-button") as HTMLButtonElement | null,
  usageRealTotal: document.getElementById("usage-real-total"),
  usageRealTotalCompact: document.getElementById("usage-real-total-compact"),
  usageRequestCount: document.getElementById("usage-request-count"),
  usageTotalCost: document.getElementById("usage-total-cost"),
  usageInputTokens: document.getElementById("usage-input-tokens"),
  usageOutputTokens: document.getElementById("usage-output-tokens"),
  usageCacheCreateTokens: document.getElementById("usage-cache-create-tokens"),
  usageCacheHitTokens: document.getElementById("usage-cache-hit-tokens"),
  usageCacheHitRate: document.getElementById("usage-cache-hit-rate"),
  usageCacheHitFill: document.getElementById("usage-cache-hit-fill") as HTMLSpanElement | null,
  usageTrendChart: document.getElementById("usage-trend-chart"),
  usageChartRange: document.getElementById("usage-chart-range"),
  usageSessionRows: document.getElementById("usage-session-rows"),
  historyProfileFilter: document.getElementById("history-profile-filter") as HTMLSelectElement | null,
  historyRangeFilter: document.getElementById("history-range-filter") as HTMLSelectElement | null,
  historyRefreshButton: document.getElementById("history-refresh-button") as HTMLButtonElement | null,
  historyRequestCount: document.getElementById("history-request-count"),
  historyTokenCount: document.getElementById("history-token-count"),
  historyCostTotal: document.getElementById("history-cost-total"),
  historySessionRows: document.getElementById("history-session-rows"),
  historySessionCount: document.getElementById("history-session-count"),
  historySearchButton: document.getElementById("history-search-button") as HTMLButtonElement | null,
  historySearchInput: document.getElementById("history-search-input") as HTMLInputElement | null,
  historyRefreshSessionsButton: document.getElementById("history-refresh-sessions-button") as HTMLButtonElement | null,
  historySessionList: document.getElementById("history-session-list"),
  historyDetailEmpty: document.getElementById("history-detail-empty"),
  historyDetailBody: document.getElementById("history-detail-body"),
  historyDetailTitle: document.getElementById("history-detail-title"),
  historyDetailMeta: document.getElementById("history-detail-meta"),
  historyResumeCommand: document.getElementById("history-resume-command"),
  historyCopyResumeButton: document.getElementById("history-copy-resume-button") as HTMLButtonElement | null,
  historyResumeButton: document.getElementById("history-resume-button") as HTMLButtonElement | null,
  historyDeleteButton: document.getElementById("history-delete-button") as HTMLButtonElement | null,
  historyMessageCount: document.getElementById("history-message-count"),
  historyMessageList: document.getElementById("history-message-list"),
  settingsShowAccountDetailToggle: document.getElementById("settings-show-account-detail-toggle"),
  updateDialog: requiredElement<HTMLDialogElement>("update-dialog"),
  updateDialogCopy: requiredElement<HTMLParagraphElement>("update-dialog-copy"),
  updateDialogLaterButton: requiredElement<HTMLButtonElement>("update-dialog-later-button"),
  updateDialogOpenButton: requiredElement<HTMLButtonElement>("update-dialog-open-button"),
  starButton: requiredElement<HTMLButtonElement>("star-button"),
  xiaohongshuButton: requiredElement<HTMLButtonElement>("xiaohongshu-button"),
  localeEnButton: requiredElement<HTMLButtonElement>("locale-en-button"),
  localeZhButton: requiredElement<HTMLButtonElement>("locale-zh-button"),
  quotaMonitorLabel: requiredElement<HTMLSpanElement>("quota-monitor-label"),
  dialog: document.getElementById("add-profile-dialog") as HTMLDialogElement,
  addProfileForm: requiredElement<HTMLFormElement>("add-profile-form"),
  cancelAddProfileButton: requiredElement<HTMLButtonElement>("cancel-add-profile-button"),
  submitAddProfileButton: requiredElement<HTMLButtonElement>("submit-add-profile-button"),
  dialogTitle: requiredElement<HTMLHeadingElement>("dialog-title"),
  dialogCopy: requiredElement<HTMLParagraphElement>("dialog-copy"),
  folderNameLabel: requiredElement<HTMLSpanElement>("folder-name-label"),
  folderNameInput: requiredElement<HTMLInputElement>("folder-name-input"),
  addBaseUrlLabel: requiredElement<HTMLSpanElement>("add-base-url-label"),
  addBaseUrlInput: requiredElement<HTMLInputElement>("add-base-url-input"),
  addBaseUrlCopy: requiredElement<HTMLSpanElement>("add-base-url-copy"),
  dialogError: requiredElement<HTMLParagraphElement>("dialog-error"),
  renameDialog: document.getElementById("rename-profile-dialog") as HTMLDialogElement,
  renameProfileForm: requiredElement<HTMLFormElement>("rename-profile-form"),
  renameDialogTitle: requiredElement<HTMLHeadingElement>("rename-dialog-title"),
  renameDialogCopy: requiredElement<HTMLParagraphElement>("rename-dialog-copy"),
  renameFolderNameLabel: requiredElement<HTMLSpanElement>("rename-folder-name-label"),
  renameFolderNameInput: requiredElement<HTMLInputElement>("rename-folder-name-input"),
  renameDialogError: requiredElement<HTMLParagraphElement>("rename-dialog-error"),
  cancelRenameProfileButton: requiredElement<HTMLButtonElement>("cancel-rename-profile-button"),
  submitRenameProfileButton: requiredElement<HTMLButtonElement>("submit-rename-profile-button"),
  deleteProfileDialog: hasDeleteProfileUi
    ? requiredElement<HTMLDialogElement>("delete-profile-dialog")
    : null,
  deleteProfileDialogTitle: hasDeleteProfileUi
    ? requiredElement<HTMLHeadingElement>("delete-profile-dialog-title")
    : null,
  deleteProfileDialogCopy: hasDeleteProfileUi
    ? requiredElement<HTMLParagraphElement>("delete-profile-dialog-copy")
    : null,
  deleteProfileDialogError: hasDeleteProfileUi
    ? requiredElement<HTMLParagraphElement>("delete-profile-dialog-error")
    : null,
  deleteProfileButton: hasDeleteProfileUi
    ? requiredElement<HTMLButtonElement>("delete-profile-button")
    : null,
  clearProfileAccountButton: hasDeleteProfileUi
    ? requiredElement<HTMLButtonElement>("clear-profile-account-button")
    : null,
  cancelDeleteProfileButton: hasDeleteProfileUi
    ? requiredElement<HTMLButtonElement>("cancel-delete-profile-button")
    : null,
  baseUrlDialog: document.getElementById("base-url-dialog") as HTMLDialogElement,
  baseUrlForm: requiredElement<HTMLFormElement>("base-url-form"),
  baseUrlDialogTitle: requiredElement<HTMLHeadingElement>("base-url-dialog-title"),
  baseUrlDialogCopy: requiredElement<HTMLParagraphElement>("base-url-dialog-copy"),
  baseUrlLabel: requiredElement<HTMLSpanElement>("base-url-label"),
  baseUrlInput: requiredElement<HTMLInputElement>("base-url-input"),
  baseUrlDialogError: requiredElement<HTMLParagraphElement>("base-url-dialog-error"),
  cancelBaseUrlButton: requiredElement<HTMLButtonElement>("cancel-base-url-button"),
  submitBaseUrlButton: requiredElement<HTMLButtonElement>("submit-base-url-button"),
  usageConfigDialog: document.getElementById("usage-config-dialog") as HTMLDialogElement | null,
  usageConfigForm: document.getElementById("usage-config-form") as HTMLFormElement | null,
  usageConfigDialogTitle: document.getElementById("usage-config-dialog-title") as HTMLHeadingElement | null,
  usageConfigDialogCopy: document.getElementById("usage-config-dialog-copy") as HTMLParagraphElement | null,
  usageConfigEnabledToggle: document.getElementById("usage-config-enabled-toggle") as HTMLInputElement | null,
  usageConfigTimeoutInput: document.getElementById("usage-config-timeout-input") as HTMLInputElement | null,
  usageConfigIntervalInput: document.getElementById("usage-config-interval-input") as HTMLInputElement | null,
  usageConfigDialogError: document.getElementById("usage-config-dialog-error") as HTMLParagraphElement | null,
  cancelUsageConfigButton: document.getElementById("cancel-usage-config-button") as HTMLButtonElement | null,
  testUsageConfigButton: document.getElementById("test-usage-config-button") as HTMLButtonElement | null,
  submitUsageConfigButton: document.getElementById("submit-usage-config-button") as HTMLButtonElement | null,
  settingsCodexCliLabel: requiredElement<HTMLElement>("settings-codex-cli-label"),
  settingsCodexCliValue: requiredElement<HTMLParagraphElement>("settings-codex-cli-value"),
  settingsCodexCliButton: requiredElement<HTMLButtonElement>("settings-codex-cli-button"),
  settingsCodexCliDetectButton: requiredElement<HTMLButtonElement>("settings-codex-cli-detect-button"),
  codexCliDialog: requiredElement<HTMLDialogElement>("codex-cli-dialog"),
  codexCliForm: requiredElement<HTMLFormElement>("codex-cli-form"),
  codexCliDialogTitle: requiredElement<HTMLHeadingElement>("codex-cli-dialog-title"),
  codexCliDialogCopy: requiredElement<HTMLParagraphElement>("codex-cli-dialog-copy"),
  codexCliCurrentLabel: requiredElement<HTMLSpanElement>("codex-cli-current-label"),
  codexCliCurrentValue: requiredElement<HTMLSpanElement>("codex-cli-current-value"),
  codexCliCurrentSource: requiredElement<HTMLSpanElement>("codex-cli-current-source"),
  codexCliInputLabel: requiredElement<HTMLSpanElement>("codex-cli-input-label"),
  codexCliInput: requiredElement<HTMLInputElement>("codex-cli-input"),
  codexCliSuggestionsHeading: requiredElement<HTMLParagraphElement>("codex-cli-suggestions-heading"),
  codexCliSuggestions: requiredElement<HTMLDivElement>("codex-cli-suggestions"),
  codexCliDialogError: requiredElement<HTMLParagraphElement>("codex-cli-dialog-error"),
  cancelCodexCliButton: requiredElement<HTMLButtonElement>("cancel-codex-cli-button"),
  clearCodexCliButton: requiredElement<HTMLButtonElement>("clear-codex-cli-button"),
  submitCodexCliButton: requiredElement<HTMLButtonElement>("submit-codex-cli-button"),
  toast: requiredElement<HTMLDivElement>("toast"),
  routeTabs: Array.from(document.querySelectorAll<HTMLElement>("[data-route-tab]")),
  pages: Array.from(document.querySelectorAll<HTMLElement>("[data-page]")),
  localizedText: Array.from(document.querySelectorAll<HTMLElement>("[data-i18n-key]")),
  localeButtons: Array.from(document.querySelectorAll<HTMLButtonElement>("[data-set-locale]")),
  themeButtons: Array.from(document.querySelectorAll<HTMLButtonElement>("[data-theme-option]")),
  addProfileButtons: Array.from(document.querySelectorAll<HTMLButtonElement>("[data-add-profile]")),
  dashboardActiveProfile: requiredElement<HTMLElement>("dashboard-active-profile"),
  dashboardProfileCount: requiredElement<HTMLElement>("dashboard-profile-count"),
  dashboardReadyCount: requiredElement<HTMLElement>("dashboard-ready-count"),
  dashboardMissingCount: requiredElement<HTMLElement>("dashboard-missing-count"),
};

function formatPercent(value: number | null): string {
  return value == null ? "--" : `${value}%`;
}

function formatRefresh(entry: QuotaWindow | undefined): string {
  if (!entry) {
    return "--";
  }
  if (entry.reset_at_timestamp != null) {
    const diff = entry.reset_at_timestamp - Math.floor(Date.now() / 1000);
    if (diff > 0) {
      const h = Math.floor(diff / 3600);
      const m = Math.floor((diff % 3600) / 60);
      if (h > 0) {
        return t(state.locale, "resetsIn", { value: `${h}h ${m}m` });
      } else if (m > 0) {
        const s = diff % 60;
        return t(state.locale, "resetsIn", { value: `${m}m ${s}s` });
      } else {
        return t(state.locale, "resetsIn", { value: `${diff}s` });
      }
    } else {
      return t(state.locale, "resetting");
    }
  }
  return entry.refresh_at || "--";
}

function escapeHtml(value: string): string {
  return value
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

function bindProfileButtons(attribute: string, handler: (profile: string) => void): void {
  for (const button of elements.profilesGrid.querySelectorAll<HTMLButtonElement>(`[${attribute}]`)) {
    button.addEventListener("click", () => {
      const profile = button.getAttribute(attribute);
      if (profile) {
        void handler(profile);
      }
    });
  }
}

function isProfileUnavailable(profile: Pick<ProfileCard, "auth_present" | "has_account_identity" | "status">): boolean {
  return profile.status === "missing_auth" || !profile.auth_present || !profile.has_account_identity;
}

function normalizeDisplayParts(
  entry: Pick<ProfileCard | CurrentCard, "folder_name" | "display_title" | "account_label">,
): { folder: string; account: string } {
  const folder = entry.folder_name.trim();
  const account = entry.account_label?.trim() ?? "";

  if (account) {
    return { folder, account };
  }

  const rawTitle = entry.display_title?.trim() ?? "";
  const parts = rawTitle.split(" / ").map((value) => value.trim()).filter(Boolean);
  if (parts.length >= 2) {
    return {
      folder: parts[0] ?? folder,
      account: parts[parts.length - 1] ?? "",
    };
  }

  return { folder, account: rawTitle };
}

function profileDisplayTitle(entry: Pick<ProfileCard, "folder_name" | "display_title" | "account_label">): string {
  const { folder, account } = normalizeDisplayParts(entry);
  if (!state.showAccountDetail) {
    return folder || account || "--";
  }
  if (folder && account && folder !== account) {
    return `${folder} · ${account}`;
  }

  return account || folder || "--";
}

function currentDisplayTitle(entry: Pick<CurrentCard, "folder_name" | "display_title" | "account_label">): string {
  const { folder, account } = normalizeDisplayParts(entry);
  if (!state.showAccountDetail) {
    return folder || account || "--";
  }
  return account || folder || "--";
}

/// Plan tokens from the backend that the front-end translates into
/// localized labels. Today only `unknown_paid` qualifies — surfaced when
/// the id_token claims `free` but quota data implies an active paid
/// window. Mapped to a localized "Unknown paid plan" label so the user
/// is prompted to re-login instead of seeing a fake tier.
const SPECIAL_PLAN_TOKENS = new Set(["unknown_paid"]);

/// `last_plan_check_ms` older than this (millis) is treated as stale —
/// the bulk plan refresh runs at most once per local day, so anything
/// past that window means the cache hasn't been re-confirmed since the
/// last day-rollover.
const PLAN_CHECK_STALE_MS = 36 * 60 * 60 * 1000;

function formatPlanName(planName: string): string {
  if (SPECIAL_PLAN_TOKENS.has(planName)) {
    if (planName === "unknown_paid") {
      return t(state.locale, "planUnknownPaid");
    }
  }
  return planName.replace(/\b([a-z])/g, (match) => match.toUpperCase());
}

export function planLine(planName: string | null, daysLeft: number | null): string {
  const formattedPlanName = planName ? formatPlanName(planName) : null;

  if (!planName && daysLeft == null) {
    return t(state.locale, "profileMetadataMissing");
  }

  // Hide the days-left suffix when it's <= 0. The cached
  // `chatgpt_subscription_active_until` claim from the id_token does not
  // automatically rotate when the subscription renews — it only
  // refreshes on a full re-login or a successful OAuth refresh that
  // re-issues claims, neither of which is guaranteed to happen for
  // weeks on a quiet account. Showing a literal "Plus / 0 days" reads
  // as "your subscription expired" when the account is in fact active.
  // Safer to show just the plan name and let the next refresh decide.
  if (formattedPlanName && daysLeft != null && daysLeft > 0) {
    return t(state.locale, "subscriptionDaysLeft", { plan: formattedPlanName, days: daysLeft });
  }

  if (formattedPlanName) {
    return formattedPlanName;
  }

  return t(state.locale, "subscriptionFallback", { days: daysLeft ?? "--" });
}

function planBadgeLabel(planName: string | null): string {
  if (!planName) {
    return state.locale === "zh-CN" ? "未知" : "Unknown";
  }

  return formatPlanName(planName)
    .replace(/\s+plan$/i, "")
    .replace(/\s+套餐$/i, "")
    .trim();
}

/// Build a hover-time tooltip describing when the plan tier was last
/// confirmed. `unknown_paid` plans get the prompt to re-login appended;
/// stale plans (>36h since last confirmation) get a hint that the bulk
/// refresh will retry at the next local-day rollover. Returns an empty
/// string when there is nothing useful to surface.
export function planFreshnessTitle(
  planName: string | null,
  lastPlanCheckMs: number | null,
): string {
  const parts: string[] = [];

  if (lastPlanCheckMs == null) {
    parts.push(t(state.locale, "planCheckedNever"));
  } else {
    const elapsedMs = Math.max(0, Date.now() - lastPlanCheckMs);
    if (elapsedMs < 60 * 1000) {
      parts.push(t(state.locale, "planCheckedJustNow"));
    } else if (elapsedMs < 60 * 60 * 1000) {
      parts.push(
        t(state.locale, "planCheckedMinutesAgo", { value: Math.floor(elapsedMs / 60_000) }),
      );
    } else if (elapsedMs < 24 * 60 * 60 * 1000) {
      parts.push(
        t(state.locale, "planCheckedHoursAgo", { value: Math.floor(elapsedMs / (60 * 60_000)) }),
      );
    } else {
      parts.push(
        t(state.locale, "planCheckedDaysAgo", {
          value: Math.floor(elapsedMs / (24 * 60 * 60_000)),
        }),
      );
    }
    if (elapsedMs >= PLAN_CHECK_STALE_MS) {
      parts.push(t(state.locale, "planCheckedStaleSuffix").trim());
    }
  }

  if (planName === "unknown_paid") {
    parts.push(t(state.locale, "planUnknownPaidHint"));
  }

  return parts.join(" ");
}

/// True when the cached plan check is old enough to render a visual
/// "stale" indicator (small dot) on the card. Mirrors the threshold
/// in `planFreshnessTitle` so both signals agree.
export function isPlanCheckStale(lastPlanCheckMs: number | null): boolean {
  if (lastPlanCheckMs == null) {
    return true;
  }
  return Date.now() - lastPlanCheckMs >= PLAN_CHECK_STALE_MS;
}

function buildMetricLineMarkup(
  label: string,
  entry: QuotaWindow | undefined,
  fillVariant: "blue" | "pink",
  unavailable: boolean,
  layout: "profile" | "current",
): string {
  const percent = unavailable ? 0 : (entry?.remaining_percent ?? 0);
  const metricClass = layout === "current" ? "current-quota-metric" : "profile-quota-metric";
  const lineClass = layout === "current" ? "current-quota-line" : "profile-quota-line";
  const titleClass = layout === "current" ? "current-quota-title" : "profile-quota-title";
  const refreshClass = layout === "current" ? "current-quota-refresh" : "profile-quota-refresh";
  const valueClass = layout === "current" ? "current-quota-value" : "profile-quota-value";
  const fillClass = unavailable ? "gray" : fillVariant;

  return `
    <section class="${metricClass}${unavailable ? " is-unavailable" : ""}">
      <div class="${lineClass}">
        <span class="${titleClass}">${escapeHtml(label)}</span>
        <span class="${refreshClass}">${escapeHtml(formatRefresh(entry))}</span>
        <span class="${valueClass}">${escapeHtml(formatPercent(unavailable ? null : entry?.remaining_percent ?? null))}</span>
      </div>
      <div class="quota-track">
        <div class="quota-fill quota-fill--${fillClass}" style="width: ${percent}%;"></div>
      </div>
    </section>
  `;
}

function buildProfileQuotaMarkup(profile: ProfileCard): string {
  const unavailable = isProfileUnavailable(profile);
  const quota = profile.quota;

  return `
    <div class="profile-quota-stack">
      ${buildMetricLineMarkup(t(state.locale, "fiveHourAllowance"), quota?.five_hour, "blue", unavailable, "profile")}
      ${buildMetricLineMarkup(t(state.locale, "weeklyAllowance"), quota?.weekly, "pink", unavailable, "profile")}
    </div>
  `;
}

function buildCurrentQuotaMarkup(
  quota: QuotaSummary | null | undefined,
  hasAccountIdentity: boolean,
): string {
  const unavailable = !hasAccountIdentity;

  return `
    <div class="current-quota-stack">
      ${buildMetricLineMarkup(t(state.locale, "fiveHourAllowance"), quota?.five_hour, "blue", unavailable, "current")}
      ${buildMetricLineMarkup(t(state.locale, "weeklyAllowance"), quota?.weekly, "pink", unavailable, "current")}
    </div>
  `;
}

export function showToast(message: string, isError = false): void {
  elements.toast.hidden = false;
  elements.toast.textContent = message;
  elements.toast.classList.toggle("is-error", isError);
  window.clearTimeout((showToast as typeof showToast & { timeoutId?: number }).timeoutId);
  (showToast as typeof showToast & { timeoutId?: number }).timeoutId = window.setTimeout(() => {
    elements.toast.hidden = true;
  }, 3200);
}

export function showUpdateDialog(update: UpdateCheckResponse): void {
  elements.updateDialogCopy.textContent = t(state.locale, "updateDialogCopy", {
    current: update.current_version,
    latest: update.latest_version ?? "--",
  });

  if (!elements.updateDialog.open) {
    elements.updateDialog.showModal();
  }
}

export function renderThemeOptions(): void {
  for (const button of elements.themeButtons) {
    const themeId = button.dataset.themeOption;
    if (!isThemeId(themeId)) {
      continue;
    }

    const option = getThemeOption(themeId);
    const title = t(state.locale, option.nameKey);
    const description = t(state.locale, option.descriptionKey);
    const titleElement = button.querySelector<HTMLElement>("[data-theme-option-title]");
    const descriptionElement = button.querySelector<HTMLElement>("[data-theme-option-description]");
    const isActive = option.id === state.theme;

    if (titleElement) {
      titleElement.textContent = title;
    }
    if (descriptionElement) {
      descriptionElement.textContent = description;
    }

    button.classList.toggle("is-active", isActive);
    button.setAttribute("aria-label", `${title} ${description}`);
    button.setAttribute("aria-pressed", isActive ? "true" : "false");
    button.setAttribute("title", `${title} - ${description}`);
  }
}

export function renderShellRoute(): void {
  document.body.dataset.route = state.route;
  document.body.classList.toggle("mac-detail-route", !isWindowsUiTarget && state.route !== "profiles");

  for (const page of elements.pages) {
    page.classList.toggle("active", page.dataset.page === state.route);
  }

  for (const tab of elements.routeTabs) {
    const isActive = tab.dataset.routeTab === state.route;
    tab.classList.toggle("active", isActive);
    tab.setAttribute("aria-current", isActive ? "page" : "false");
  }
}

export function renderShellOverview(dashboard: DashboardViewModel | null): void {
  if (!dashboard || !state.snapshot) {
    elements.dashboardActiveProfile.textContent = "--";
    elements.dashboardProfileCount.textContent = "--";
    elements.dashboardReadyCount.textContent = "--";
    elements.dashboardMissingCount.textContent = "--";
    renderUsageStats();
    renderSessionManager();
    return;
  }

  const profiles = state.snapshot.profiles;
  const readyCount = profiles.filter((profile) => (
    profile.auth_present && profile.has_account_identity && profile.status !== "missing_auth"
  )).length;
  const missingCount = profiles.length - readyCount;

  elements.dashboardActiveProfile.textContent = dashboard.current_card
    ? currentDisplayTitle(dashboard.current_card)
    : t(state.locale, "noActiveProfile");
  elements.dashboardProfileCount.textContent = String(profiles.length);
  elements.dashboardReadyCount.textContent = String(readyCount);
  elements.dashboardMissingCount.textContent = String(Math.max(0, missingCount));
  renderUsageSettingsControls(profiles);
  renderUsageStats();
  renderSessionManager();
}

function formatCompactNumber(value: number): string {
  if (!Number.isFinite(value)) {
    return "--";
  }
  if (value >= 10_000_000) {
    return `${(value / 10_000_000).toFixed(2)}kw`;
  }
  if (value >= 10_000) {
    return `${(value / 10_000).toFixed(1)}w`;
  }
  return Math.round(value).toLocaleString();
}

function formatFullNumber(value: number): string {
  if (!Number.isFinite(value)) {
    return "--";
  }
  return Math.round(value).toLocaleString(state.locale === "zh-CN" ? "zh-CN" : "en-US");
}

function formatSecondaryTokenUnit(value: number): string {
  if (!Number.isFinite(value)) {
    return "--";
  }
  if (state.locale === "zh-CN") {
    return `${(value / 10_000).toFixed(2)}万`;
  }
  return `${(value / 1_000_000).toFixed(2)} millions`;
}

function formatMoney(value: number): string {
  return `$${value.toFixed(4)}`;
}

function formatDateTime(seconds: number): string {
  if (!seconds) {
    return "--";
  }
  return new Date(seconds * 1000).toLocaleString(state.locale === "zh-CN" ? "zh-CN" : "en-US", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function formatSessionDateTime(seconds: number | null): string {
  if (!seconds) {
    return "--";
  }
  return new Date(seconds * 1000).toLocaleString(state.locale === "zh-CN" ? "zh-CN" : "en-US", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}

function formatRelativeSessionTime(seconds: number | null): string {
  if (!seconds) {
    return "--";
  }
  const diff = Math.max(0, Math.floor(Date.now() / 1000) - seconds);
  if (diff < 60) {
    return t(state.locale, "sessionJustNow");
  }
  if (diff < 3600) {
    return t(state.locale, "sessionMinutesAgo", { value: String(Math.floor(diff / 60)) });
  }
  if (diff < 86_400) {
    return t(state.locale, "sessionHoursAgo", { value: String(Math.floor(diff / 3600)) });
  }
  return t(state.locale, "sessionDaysAgo", { value: String(Math.floor(diff / 86_400)) });
}

function renderUsageSettingsControls(profiles: ProfileCard[]): void {
  const select = elements.settingsUsageProfileSelect;
  if (!select) {
    return;
  }
  const selected = state.settingsUsageProfile ?? state.currentProfile ?? profiles[0]?.folder_name ?? "";
  if (state.settingsUsageProfile !== selected) {
    state.settingsUsageProfile = selected || null;
  }
  select.innerHTML = profiles
    .map((profile) => (
      `<option value="${escapeHtml(profile.folder_name)}"${profile.folder_name === selected ? " selected" : ""}>${escapeHtml(profileDisplayTitle(profile))}</option>`
    ))
    .join("");
  const settings = selected ? state.usageSettingsByProfile[selected] : null;
  if (elements.settingsUsageEnabledToggle) {
    elements.settingsUsageEnabledToggle.checked = Boolean(settings?.enabled);
  }
  if (elements.settingsUsageTimeoutInput) {
    elements.settingsUsageTimeoutInput.value = String(settings?.timeout_seconds ?? 10);
  }
  if (elements.settingsUsageIntervalInput) {
    elements.settingsUsageIntervalInput.value = String(settings?.auto_query_interval_minutes ?? 5);
  }
}

function renderStatsFilters(options: {
  stats: UsageStatsResponse | null;
  profileSelect: HTMLSelectElement | null;
  rangeSelect: HTMLSelectElement | null;
  selectedProfile: string | null;
  selectedRange: "today" | "7d" | "30d";
}): void {
  if (options.rangeSelect) {
    options.rangeSelect.value = options.selectedRange;
  }
  const select = options.profileSelect;
  if (!select || !options.stats) {
    return;
  }
  select.innerHTML = [
    `<option value="">${escapeHtml(t(state.locale, "usageFilterAll"))}</option>`,
    ...options.stats.profiles.map((profile) => (
      `<option value="${escapeHtml(profile.folder_name)}"${profile.folder_name === options.selectedProfile ? " selected" : ""}>${escapeHtml(profile.display_title)}</option>`
    )),
  ].join("");
}

function renderUsageFilters(stats: UsageStatsResponse | null): void {
  renderStatsFilters({
    stats,
    profileSelect: elements.usageProfileFilter,
    rangeSelect: elements.usageRangeFilter,
    selectedProfile: state.usageStatsProfile,
    selectedRange: state.usageStatsRange,
  });
}

function renderUsageTrendChart(stats: UsageStatsResponse): void {
  const container = elements.usageTrendChart;
  if (!container) {
    return;
  }
  if (!stats.trends.length) {
    container.innerHTML = `<div class="settings-usage-empty">${escapeHtml(t(state.locale, "usageEmpty"))}</div>`;
    return;
  }
  const width = 760;
  const height = 260;
  const pad = { left: 44, right: 42, top: 18, bottom: 40 };
  const maxToken = Math.max(1, ...stats.trends.map((point) => point.real_total_tokens));
  const maxCost = Math.max(0.0001, ...stats.trends.map((point) => point.total_cost_usd));
  const x = (index: number) => {
    const span = Math.max(1, stats.trends.length - 1);
    return pad.left + ((width - pad.left - pad.right) * index) / span;
  };
  const yToken = (value: number) => pad.top + (height - pad.top - pad.bottom) * (1 - value / maxToken);
  const yCost = (value: number) => pad.top + (height - pad.top - pad.bottom) * (1 - value / maxCost);
  const tokenPath = stats.trends.map((point, index) => `${index === 0 ? "M" : "L"}${x(index).toFixed(1)} ${yToken(point.real_total_tokens).toFixed(1)}`).join(" ");
  const inputPath = stats.trends.map((point, index) => `${index === 0 ? "M" : "L"}${x(index).toFixed(1)} ${yToken(point.input_tokens).toFixed(1)}`).join(" ");
  const outputPath = stats.trends.map((point, index) => `${index === 0 ? "M" : "L"}${x(index).toFixed(1)} ${yToken(point.output_tokens).toFixed(1)}`).join(" ");
  const costPath = stats.trends.map((point, index) => `${index === 0 ? "M" : "L"}${x(index).toFixed(1)} ${yCost(point.total_cost_usd).toFixed(1)}`).join(" ");
  const areaPath = `${tokenPath} L${x(stats.trends.length - 1).toFixed(1)} ${height - pad.bottom} L${pad.left} ${height - pad.bottom} Z`;
  const ticks = stats.trends.filter((_, index) => index === 0 || index === stats.trends.length - 1 || index % Math.ceil(stats.trends.length / 4) === 0);
  const points = stats.trends.map((point, index) => {
    const label = [
      point.bucket,
      `${t(state.locale, "usageTokens")}: ${formatFullNumber(point.real_total_tokens)}`,
      `${t(state.locale, "usageInput")}: ${formatFullNumber(point.input_tokens)}`,
      `${t(state.locale, "usageOutput")}: ${formatFullNumber(point.output_tokens)}`,
      `${t(state.locale, "usageCost")}: ${formatMoney(point.total_cost_usd)}`,
    ].join("\n");
    return `
      <g class="usage-point-group" tabindex="0">
        <circle cx="${x(index).toFixed(1)}" cy="${yToken(point.real_total_tokens).toFixed(1)}" r="4.5" class="usage-point usage-point--cache" />
        <title>${escapeHtml(label)}</title>
      </g>
    `;
  }).join("");
  container.innerHTML = `
    <svg viewBox="0 0 ${width} ${height}" role="img" aria-label="${escapeHtml(t(state.locale, "usageTrendTitle"))}">
      <defs>
        <linearGradient id="usageTokenGradient" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stop-color="#a855f7" stop-opacity="0.22" />
          <stop offset="100%" stop-color="#a855f7" stop-opacity="0" />
        </linearGradient>
      </defs>
      ${[0, 0.25, 0.5, 0.75, 1].map((ratio) => {
        const y = pad.top + (height - pad.top - pad.bottom) * ratio;
        return `<line x1="${pad.left}" y1="${y}" x2="${width - pad.right}" y2="${y}" class="usage-grid-line" />`;
      }).join("")}
      <path d="${areaPath}" fill="url(#usageTokenGradient)" />
      <path d="${tokenPath}" class="usage-line usage-line--cache" />
      <path d="${costPath}" class="usage-line usage-line--cost" />
      <path d="${inputPath}" class="usage-line usage-line--input" />
      <path d="${outputPath}" class="usage-line usage-line--output" />
      ${points}
      ${ticks.map((point, index) => `<text x="${x(stats.trends.indexOf(point))}" y="${height - 12}" class="usage-axis-label" text-anchor="${index === 0 ? "start" : "middle"}">${escapeHtml(formatDateTime(point.timestamp))}</text>`).join("")}
      <text x="${pad.left}" y="${pad.top + 8}" class="usage-axis-label">tokens</text>
      <text x="${width - pad.right}" y="${pad.top + 8}" class="usage-axis-label" text-anchor="end">$</text>
    </svg>
    <div class="usage-legend">
      <span class="legend-cost">${escapeHtml(t(state.locale, "usageCost"))}</span>
      <span class="legend-cache">${escapeHtml(t(state.locale, "usageCacheHit"))}</span>
      <span class="legend-input">${escapeHtml(t(state.locale, "usageInput"))}</span>
      <span class="legend-output">${escapeHtml(t(state.locale, "usageOutput"))}</span>
    </div>
  `;
}

export function renderUsageStats(): void {
  const stats = state.usageStats;
  renderUsageFilters(stats);
  if (
    !stats ||
    !elements.usageRealTotal ||
    !elements.usageRealTotalCompact ||
    !elements.usageRequestCount ||
    !elements.usageTotalCost ||
    !elements.usageInputTokens ||
    !elements.usageOutputTokens ||
    !elements.usageCacheCreateTokens ||
    !elements.usageCacheHitTokens ||
    !elements.usageCacheHitRate ||
    !elements.usageCacheHitFill ||
    !elements.usageSessionRows
  ) {
    return;
  }

  elements.usageRealTotal.textContent = formatFullNumber(stats.totals.real_total_tokens);
  elements.usageRealTotalCompact.textContent = formatSecondaryTokenUnit(stats.totals.real_total_tokens);
  elements.usageRequestCount.textContent = stats.totals.request_count.toLocaleString();
  elements.usageTotalCost.textContent = formatMoney(stats.totals.total_cost_usd);
  elements.usageInputTokens.textContent = formatCompactNumber(stats.totals.input_tokens);
  elements.usageOutputTokens.textContent = formatCompactNumber(stats.totals.output_tokens);
  elements.usageCacheCreateTokens.textContent = stats.totals.cache_creation_tokens === 0
    ? "N/A"
    : formatCompactNumber(stats.totals.cache_creation_tokens);
  elements.usageCacheHitTokens.textContent = formatCompactNumber(stats.totals.cache_read_tokens);
  const cacheRate = Math.max(0, Math.min(100, stats.totals.cache_hit_rate * 100));
  elements.usageCacheHitRate.textContent = `${cacheRate.toFixed(1)}%`;
  elements.usageCacheHitFill.style.width = `${cacheRate}%`;
  if (elements.usageChartRange) {
    elements.usageChartRange.textContent = `${formatDateTime(stats.start_at)} - ${formatDateTime(stats.end_at)}`;
  }
  renderUsageTrendChart(stats);
  elements.usageSessionRows.innerHTML = stats.sessions.length
    ? stats.sessions.map((row) => `
        <div class="usage-session-row">
          <span title="${escapeHtml(row.session_id)}">${escapeHtml(row.session_id.slice(0, 12))}</span>
          <span>${escapeHtml(row.profile)}</span>
          <span>${escapeHtml(row.model)}</span>
          <span>${escapeHtml(formatDateTime(row.started_at))}</span>
          <strong>${escapeHtml(formatCompactNumber(row.real_total_tokens))}</strong>
        </div>
      `).join("")
    : `<div class="settings-usage-empty">${escapeHtml(t(state.locale, "usageEmpty"))}</div>`;
}

export function renderHistoryStats(): void {
  renderSessionManager();
}

function sessionTitle(session: CodexSessionMeta): string {
  return session.title || session.summary || session.session_id;
}

function sessionSearchBlob(session: CodexSessionMeta): string {
  return [
    session.session_id,
    session.title,
    session.summary,
    session.project_dir,
    session.profile,
    session.resume_command,
  ].filter(Boolean).join(" ").toLowerCase();
}

function roleLabel(role: string): string {
  const normalized = role.toLowerCase();
  if (normalized === "assistant") {
    return t(state.locale, "sessionRoleAssistant");
  }
  if (normalized === "user") {
    return t(state.locale, "sessionRoleUser");
  }
  if (normalized === "developer") {
    return t(state.locale, "sessionRoleDeveloper");
  }
  if (normalized === "tool") {
    return t(state.locale, "sessionRoleTool");
  }
  return role || "unknown";
}

function renderSessionMessage(message: CodexSessionMessage): string {
  return `
    <article class="session-message-card">
      <div class="session-message-head">
        <strong>${escapeHtml(roleLabel(message.role))}</strong>
        <span>${escapeHtml(formatSessionDateTime(message.ts))}</span>
      </div>
      <pre>${escapeHtml(message.content)}</pre>
    </article>
  `;
}

export function renderSessionManager(): void {
  if (!elements.historySessionList) {
    return;
  }

  const query = state.sessionSearch.trim().toLowerCase();
  const sessions = query
    ? state.codexSessions.filter((session) => sessionSearchBlob(session).includes(query))
    : state.codexSessions;

  if (elements.historySessionCount) {
    elements.historySessionCount.textContent = String(sessions.length);
  }
  if (elements.historySearchInput && elements.historySearchInput.value !== state.sessionSearch) {
    elements.historySearchInput.value = state.sessionSearch;
  }

  elements.historySessionList.innerHTML = sessions.length
    ? sessions.map((session) => {
      const active = session.source_path === state.selectedCodexSessionPath;
      const title = sessionTitle(session);
      const time = formatRelativeSessionTime(session.last_active_at ?? session.created_at);
      const project = session.project_dir ? session.project_dir.split("/").filter(Boolean).pop() : "--";
      return `
        <button class="session-list-item${active ? " is-active" : ""}" type="button" data-session-path="${escapeHtml(session.source_path)}">
          <div class="session-list-icon">${macIcon("openai", "cc-icon cc-icon--openai")}</div>
          <div class="session-list-copy">
            <strong>${escapeHtml(title)}</strong>
            <span class="session-list-meta">${escapeHtml(time)} · ${escapeHtml(project || "--")}</span>
            ${session.summary ? `<span class="session-list-summary">${escapeHtml(session.summary)}</span>` : ""}
          </div>
          <span class="session-list-chevron">›</span>
        </button>
      `;
    }).join("")
    : `<div class="session-empty">${escapeHtml(t(state.locale, "sessionNoSessions"))}</div>`;

  const selected = state.codexSessions.find((session) => session.source_path === state.selectedCodexSessionPath) ?? null;
  const hasSelected = Boolean(selected);
  elements.historyDetailEmpty?.toggleAttribute("hidden", hasSelected);
  elements.historyDetailBody?.toggleAttribute("hidden", !hasSelected);
  if (!selected) {
    if (elements.historyMessageList) {
      elements.historyMessageList.innerHTML = "";
    }
    return;
  }

  if (elements.historyDetailTitle) {
    elements.historyDetailTitle.textContent = sessionTitle(selected);
  }
  if (elements.historyDetailMeta) {
    const project = selected.project_dir || "--";
    const timestamp = selected.last_active_at ?? selected.created_at;
    elements.historyDetailMeta.innerHTML = `
      <span>${macIcon("history", "cc-icon cc-icon--inline")}${escapeHtml(formatSessionDateTime(timestamp))}</span>
      <span>${macIcon("book-open", "cc-icon cc-icon--inline")}${escapeHtml(project)}</span>
    `;
  }
  if (elements.historyResumeCommand) {
    elements.historyResumeCommand.textContent = selected.resume_command;
  }
  if (elements.historyMessageCount) {
    elements.historyMessageCount.textContent = String(state.codexSessionMessages.length);
  }
  if (elements.historyMessageList) {
    elements.historyMessageList.innerHTML = state.codexSessionMessages.length
      ? state.codexSessionMessages.map(renderSessionMessage).join("")
      : `<div class="session-empty">${escapeHtml(t(state.locale, "sessionNoMessages"))}</div>`;
  }
}

export function renderCurrentCard(dashboard: DashboardViewModel): void {
  const current = dashboard.current_card;
  if (!current) {
    elements.currentTitle.textContent = t(state.locale, "noActiveProfile");
    elements.currentPlan.textContent = t(state.locale, "switchToStart");
    elements.currentLoginButton.disabled = true;
    elements.openCurrentFolderButton.disabled = true;
    state.currentProfile = null;
    elements.currentQuotaPanel.innerHTML =
      `<div class="empty-state">${t(state.locale, "quotaWillAppear")}</div>`;
    return;
  }

  state.currentProfile = current.folder_name;
  elements.currentTitle.textContent = currentDisplayTitle(current);
  elements.currentPlan.textContent = planLine(current.plan_name, current.subscription_days_left);
  const currentPlanTitle = planFreshnessTitle(current.plan_name, current.last_plan_check_ms);
  if (currentPlanTitle) {
    elements.currentPlan.title = currentPlanTitle;
  } else {
    elements.currentPlan.removeAttribute("title");
  }
  elements.currentPlan.classList.toggle(
    "plan-check-stale",
    isPlanCheckStale(current.last_plan_check_ms),
  );
  elements.currentPlan.classList.toggle(
    "plan-unknown-paid",
    current.plan_name === "unknown_paid",
  );
  elements.currentLoginButton.disabled = state.loading;
  elements.openCurrentFolderButton.disabled = false;
  elements.currentQuotaPanel.innerHTML = buildCurrentQuotaMarkup(
    dashboard.current_quota_card,
    current.has_account_identity,
  );
}

export function renderProfiles(
  dashboard: DashboardViewModel,
  onDelete: (profile: string) => void,
  onRename: (profile: string) => void,
  onSwitch: (profile: string) => void,
  onRefresh: (profile: string) => void,
  onBaseUrl: (profile: string) => void,
  onLogin: (profile: string) => void,
  onToggleQuota: (profile: string) => void,
  onReorder: (sourceProfile: string, targetProfile: string) => void,
): void {
  if (!dashboard.profiles.length) {
    elements.profilesGrid.innerHTML =
      `<div class="empty-state profiles-empty-state">${t(state.locale, "profilesEmpty")}</div>`;
    return;
  }

  elements.profilesGrid.innerHTML = dashboard.profiles
    .map((profile) => {
      const refreshRunning = state.refreshActiveProfiles.includes(profile.folder_name);
      const refreshPending = refreshRunning;
      const loginRunning = state.loginActiveProfile === profile.folder_name;
      // Any in-flight login (on this card or any other) blocks new logins
      // because the OAuth port and `.switch.lock` are global resources.
      const loginPending = state.loginActiveProfile !== null;
      const cardBusy = refreshPending || loginRunning;
      const deleteDisabled =
        state.loading || cardBusy || profile.status === "current";
      const renameDisabled =
        state.loading || cardBusy || profile.status === "current";
      const refreshDisabled =
        !profile.auth_present || state.loading || cardBusy || loginPending;
      const baseDisabled = state.loading || cardBusy;
      const quotaToggleDisabled = state.loading || cardBusy;
      const switchDisabled =
        !profile.auth_present || state.loading || cardBusy || loginPending || profile.status === "current";

      const refreshTitle = refreshRunning
        ? t(state.locale, "profileRefreshRunning")
        : refreshDisabled
          ? t(state.locale, "profileRefreshDisabled")
          : t(state.locale, "profileRefreshReady");

      const loginDisabled =
        state.loading || refreshPending || (loginPending && !loginRunning);
      const unavailable = isProfileUnavailable(profile);

      const planTooltip = planFreshnessTitle(profile.plan_name, profile.last_plan_check_ms);
      const planClasses = [
        "profile-plan",
        isPlanCheckStale(profile.last_plan_check_ms) ? "plan-check-stale" : null,
        profile.plan_name === "unknown_paid" ? "plan-unknown-paid" : null,
      ]
        .filter((value): value is string => value != null)
        .join(" ");

      if (!isWindowsUiTarget) {
        const displayTitle = profileDisplayTitle(profile);
        const showQuotaPanel = state.expandedQuotaProfiles.includes(profile.folder_name);
        const needsRelogin = state.reloginProfiles.includes(profile.folder_name);
        const macCardMode = showQuotaPanel ? " mac-account-card--expanded" : " mac-account-card--compact";
        const primaryActionLabel = needsRelogin
          ? t(state.locale, "reloginButton")
          : profile.status === "current"
          ? (state.locale === "zh-CN" ? "已登录" : "Signed in")
          : t(state.locale, "switch");
        const primaryActionAttribute = needsRelogin || profile.status === "current"
          ? `data-login-profile="${profile.folder_name}"`
          : `data-switch-profile="${profile.folder_name}"`;
        const primaryActionDisabled = needsRelogin ? loginDisabled : profile.status === "current" ? true : switchDisabled;
        const primaryActionTitle = needsRelogin
          ? t(state.locale, "profileReloginReady")
          : profile.status === "current"
          ? (state.locale === "zh-CN" ? "当前账号已登录" : "Current account is signed in")
          : (
              switchDisabled
                ? t(state.locale, "profileSwitchDisabled")
                : t(state.locale, "profileSwitchReady")
            );

        return `
          <article class="profile-card mac-account-card${macCardMode} status-${profile.status}${unavailable ? " is-unavailable-card" : ""}" data-profile-card="${profile.folder_name}">
            <div class="mac-card-grip" data-drag-profile="${profile.folder_name}" title="${escapeHtml(t(state.locale, "dragToReorder"))}" aria-label="${escapeHtml(t(state.locale, "dragToReorder"))}">${macIcon("grip-vertical", "cc-icon cc-icon--muted")}</div>
            <div class="mac-provider-mark" aria-hidden="true">
              <img class="cc-icon cc-icon--provider" src="${macIconBase}/openai.svg" alt="" />
            </div>

            <div class="mac-profile-main">
              <div class="mac-title-row">
                <p class="profile-title-account">${escapeHtml(displayTitle)}</p>
                <span class="mac-route-badge">${escapeHtml(planBadgeLabel(profile.plan_name))}</span>
              </div>
            </div>

            <div class="mac-quota-summary">
              <span>${escapeHtml(t(state.locale, "fiveHourAllowance"))}: <strong>${escapeHtml(formatPercent(unavailable ? null : profile.quota?.five_hour?.remaining_percent ?? null))}</strong></span>
              <span>${escapeHtml(t(state.locale, "weeklyAllowance"))}: <strong>${escapeHtml(formatPercent(unavailable ? null : profile.quota?.weekly?.remaining_percent ?? null))}</strong></span>
            </div>

            ${showQuotaPanel ? buildProfileQuotaMarkup(profile) : ""}

            <div class="profile-card-actions mac-profile-actions">
              <button
                class="profile-action-button mac-profile-primary${profile.status === "current" ? " mac-profile-primary--login" : ""}"
                type="button"
                title="${escapeHtml(primaryActionTitle)}"
                ${primaryActionAttribute}
                ${primaryActionDisabled ? "disabled" : ""}
              >
                ${needsRelogin || profile.status === "current" ? macIcon("openai", "cc-icon cc-icon--primary-action") : macIcon("play", "cc-icon cc-icon--primary-action")}
                <span class="mac-action-label">${escapeHtml(primaryActionLabel)}</span>
              </button>
              <button
                class="profile-action-button mac-icon-button mac-icon-button--edit"
                type="button"
                title="${renameDisabled ? t(state.locale, "profileRenameDisabled") : t(state.locale, "profileRenameReady")}"
                data-rename-profile="${profile.folder_name}"
                ${renameDisabled ? "disabled" : ""}
              >
                ${macIcon("pencil", "cc-icon cc-icon--action")}
                <span class="mac-action-label">${t(state.locale, "rename")}</span>
              </button>
              <button
                class="profile-action-button mac-icon-button mac-icon-button--gpt"
                type="button"
                title="${
                  loginRunning
                    ? t(state.locale, "profileLoginCancelHint")
                    : loginDisabled
                      ? t(state.locale, "profileLoginDisabled")
                      : t(state.locale, "profileLoginReady")
                }"
                aria-label="${
                  loginRunning
                    ? t(state.locale, "profileLoginCancelAria", { profile: profile.folder_name })
                    : t(state.locale, "profileLoginReadyAria", { profile: profile.folder_name })
                }"
                data-login-profile="${profile.folder_name}"
                ${loginDisabled ? "disabled" : ""}
              >
                ${
                  loginRunning
                    ? `<span class="button-spinner" aria-hidden="true"></span><span class="mac-action-label">${t(state.locale, "cancel")}</span>`
                    : `${macIcon("openai", "cc-icon cc-icon--action cc-icon--openai")}<span class="mac-action-label">${t(state.locale, "loginButton")}</span>`
                }
              </button>
              <button
                class="profile-action-button mac-icon-button mac-icon-button--refresh"
                type="button"
                title="${refreshTitle}"
                data-refresh-profile="${profile.folder_name}"
                ${refreshDisabled ? "disabled" : ""}
              >
                ${
                  refreshPending
                    ? '<span class="button-spinner" aria-hidden="true"></span>'
                    : `${macIcon("refresh-cw", "cc-icon cc-icon--action")}<span class="mac-action-label">${t(state.locale, "refreshButton")}</span>`
                }
              </button>
              <button
                class="profile-action-button mac-icon-button mac-icon-button--usage${showQuotaPanel ? " is-active" : ""}"
                type="button"
                title="${showQuotaPanel ? t(state.locale, "profileQuotaCollapse") : t(state.locale, "profileQuotaExpand")}"
                aria-pressed="${showQuotaPanel ? "true" : "false"}"
                data-toggle-quota-profile="${profile.folder_name}"
                ${quotaToggleDisabled ? "disabled" : ""}
              >
                ${macIcon("chart-column", "cc-icon cc-icon--action")}
                <span class="mac-action-label">${showQuotaPanel ? t(state.locale, "collapse") : t(state.locale, "usageButton")}</span>
              </button>
              ${
                hasDeleteProfileUi
                  ? `<button
                      class="profile-action-button mac-icon-button mac-icon-button--delete profile-action-button-danger"
                      type="button"
                      title="${deleteDisabled ? t(state.locale, "profileDeleteDisabled") : t(state.locale, "profileDeleteReady")}"
                      data-delete-profile="${profile.folder_name}"
                      ${deleteDisabled ? "disabled" : ""}
                    >
                      ${macIcon("trash-2", "cc-icon cc-icon--action")}
                      <span class="mac-action-label">${t(state.locale, "deleteButton")}</span>
                    </button>`
                  : ""
              }
            </div>
          </article>
        `;
      }

      return `
        <article class="profile-card status-${profile.status}${unavailable ? " is-unavailable-card" : ""}">
          <div class="profile-title-wrap">
            <p class="profile-title-account">${escapeHtml(profileDisplayTitle(profile))}</p>
            <p class="${planClasses}"${planTooltip ? ` title="${escapeHtml(planTooltip)}"` : ""}>${escapeHtml(planLine(profile.plan_name, profile.subscription_days_left))}</p>
          </div>

          ${buildProfileQuotaMarkup(profile)}

          <div class="profile-card-actions${isWindowsUiTarget ? " profile-card-actions--windows" : ""}">
            ${
              hasDeleteProfileUi
                ? `<button
                    class="profile-action-button profile-action-button-danger"
                    type="button"
                    title="${deleteDisabled ? t(state.locale, "profileDeleteDisabled") : t(state.locale, "profileDeleteReady")}"
                    data-delete-profile="${profile.folder_name}"
                    ${deleteDisabled ? "disabled" : ""}
                  >
                    ${t(state.locale, "deleteButton")}
                  </button>`
                : ""
            }
            <button
              class="profile-action-button"
              type="button"
              title="${renameDisabled ? t(state.locale, "profileRenameDisabled") : t(state.locale, "profileRenameReady")}"
              data-rename-profile="${profile.folder_name}"
              ${renameDisabled ? "disabled" : ""}
            >
              ${t(state.locale, "rename")}
            </button>
            <button
              class="profile-action-button"
              type="button"
              title="${refreshTitle}"
              data-refresh-profile="${profile.folder_name}"
              ${refreshDisabled ? "disabled" : ""}
            >
              ${
                refreshPending
                  ? '<span class="button-spinner" aria-hidden="true"></span>'
                  : t(state.locale, "refreshButton")
              }
            </button>
            <button
              class="profile-action-button"
              type="button"
              title="${
                loginRunning
                  ? t(state.locale, "profileLoginCancelHint")
                  : loginDisabled
                    ? t(state.locale, "profileLoginDisabled")
                    : t(state.locale, "profileLoginReady")
              }"
              aria-label="${
                loginRunning
                  ? t(state.locale, "profileLoginCancelAria", { profile: profile.folder_name })
                  : t(state.locale, "profileLoginReadyAria", { profile: profile.folder_name })
              }"
              data-login-profile="${profile.folder_name}"
              ${loginDisabled ? "disabled" : ""}
            >
              ${
                loginRunning
                  ? `<span class="button-spinner" aria-hidden="true"></span><span class="button-cancel-label">${t(state.locale, "cancel")}</span>`
                  : t(state.locale, "loginButton")
              }
            </button>
            <button
              class="${
                profile.openai_base_url
                  ? "profile-action-button profile-action-button-danger"
                  : "profile-action-button"
              }"
              type="button"
              title="${
                profile.openai_base_url
                  ? t(state.locale, "profileBaseConfigured")
                  : t(state.locale, "profileBaseReady")
              }"
              data-base-url-profile="${profile.folder_name}"
              ${baseDisabled ? "disabled" : ""}
            >
              ${t(state.locale, "baseButton")}
            </button>
            <button
              class="profile-action-button"
              type="button"
              title="${switchDisabled ? t(state.locale, "profileSwitchDisabled") : t(state.locale, "profileSwitchReady")}"
              data-switch-profile="${profile.folder_name}"
              ${switchDisabled ? "disabled" : ""}
            >
              ${t(state.locale, "switch")}
            </button>
          </div>
        </article>
      `;
    })
    .join("");

  if (hasDeleteProfileUi) {
    bindProfileButtons("data-delete-profile", onDelete);
  }
  bindProfileButtons("data-rename-profile", onRename);
  bindProfileButtons("data-refresh-profile", onRefresh);
  bindProfileButtons("data-login-profile", onLogin);
  bindProfileButtons("data-toggle-quota-profile", onToggleQuota);
  bindProfileButtons("data-base-url-profile", onBaseUrl);
  bindProfileButtons("data-switch-profile", onSwitch);
  bindProfileDragHandles(onReorder);
}

function bindProfileDragHandles(handler: (sourceProfile: string, targetProfile: string) => void): void {
  const cards = Array.from(
    elements.profilesGrid.querySelectorAll<HTMLElement>("[data-profile-card]"),
  );
  const activationDistance = 8;

  const clearDragClasses = (): void => {
    for (const card of cards) {
      card.classList.remove("is-dragging", "is-drag-over");
    }
  };

  for (const grip of elements.profilesGrid.querySelectorAll<HTMLElement>("[data-drag-profile]")) {
    grip.addEventListener("pointerdown", (event) => {
      if (event.button !== 0) {
        return;
      }

      const sourceProfile = grip.dataset.dragProfile;
      const sourceCard = grip.closest<HTMLElement>("[data-profile-card]");
      if (!sourceProfile || !sourceCard) {
        return;
      }

      event.preventDefault();
      const pointerId = event.pointerId;
      const startX = event.clientX;
      const startY = event.clientY;
      const sourceRect = sourceCard.getBoundingClientRect();
      let dragStarted = false;
      let dragPreview: HTMLElement | null = null;
      let targetProfile: string | null = null;

      const startDrag = (): void => {
        dragStarted = true;
        sourceCard.classList.add("is-dragging");

        dragPreview = sourceCard.cloneNode(true) as HTMLElement;
        dragPreview.classList.add("mac-drag-preview");
        dragPreview.classList.remove("is-dragging", "is-drag-over");
        dragPreview.removeAttribute("data-profile-card");
        dragPreview.setAttribute("aria-hidden", "true");
        dragPreview.style.width = `${sourceRect.width}px`;
        dragPreview.style.height = `${sourceRect.height}px`;
        dragPreview.style.left = `${sourceRect.left}px`;
        dragPreview.style.top = `${sourceRect.top}px`;
        document.body.append(dragPreview);
      };

      const movePreview = (clientX: number, clientY: number): void => {
        if (!dragPreview) {
          return;
        }
        dragPreview.style.transform = `translate3d(${clientX - startX}px, ${clientY - startY}px, 0)`;
      };

      const handlePointerMove = (moveEvent: PointerEvent): void => {
        if (moveEvent.pointerId !== pointerId) {
          return;
        }

        const distance = Math.hypot(moveEvent.clientX - startX, moveEvent.clientY - startY);
        if (!dragStarted && distance < activationDistance) {
          return;
        }
        if (!dragStarted) {
          startDrag();
        }

        moveEvent.preventDefault();
        movePreview(moveEvent.clientX, moveEvent.clientY);
        const targetCard = document
          .elementFromPoint(moveEvent.clientX, moveEvent.clientY)
          ?.closest<HTMLElement>("[data-profile-card]");
        const nextTarget = targetCard?.dataset.profileCard ?? null;

        for (const card of cards) {
          card.classList.toggle(
            "is-drag-over",
            Boolean(nextTarget && nextTarget !== sourceProfile && card === targetCard),
          );
        }
        targetProfile = nextTarget && nextTarget !== sourceProfile ? nextTarget : null;
      };

      const cleanupDrag = (): void => {
        window.removeEventListener("pointermove", handlePointerMove);
        window.removeEventListener("pointerup", finishDrag);
        window.removeEventListener("pointercancel", cancelDrag);
        dragPreview?.remove();
        dragPreview = null;
        clearDragClasses();
      };

      const finishDrag = (upEvent: PointerEvent): void => {
        if (upEvent.pointerId !== pointerId) {
          return;
        }
        cleanupDrag();
        if (dragStarted && targetProfile) {
          handler(sourceProfile, targetProfile);
        }
      };

      const cancelDrag = (cancelEvent: PointerEvent): void => {
        if (cancelEvent.pointerId !== pointerId) {
          return;
        }
        cleanupDrag();
      };

      window.addEventListener("pointermove", handlePointerMove, { passive: false });
      window.addEventListener("pointerup", finishDrag);
      window.addEventListener("pointercancel", cancelDrag);
    });
  }
}

export function renderPaging(
  paging: Pick<PagingInfo, "has_previous" | "has_next" | "page" | "total_pages">,
): void {
  elements.previousPageButton.disabled = state.loading || !paging.has_previous;
  elements.nextPageButton.disabled = state.loading || !paging.has_next;
  elements.pageIndicator.textContent = `${paging.page} / ${paging.total_pages}`;
}

export function applyLocale(): void {
  document.documentElement.lang = state.locale;
  document.title = t(state.locale, "appTitle");

  for (const element of elements.localizedText) {
    const key = element.dataset.i18nKey as MessageKey | undefined;
    if (key) {
      element.textContent = t(state.locale, key);
    }
  }

  elements.profilesHeading.textContent = t(state.locale, "profilesHeading");
  elements.currentSectionHeading.textContent = t(state.locale, "currentSession");
  elements.controlDeckHeading.textContent = t(state.locale, "controlDeck");
  elements.currentLoginButton.textContent = t(state.locale, "login");
  elements.openCurrentFolderButton.textContent = t(state.locale, "openFolder");
  elements.addProfilesButton.textContent = t(state.locale, "addProfiles");
  elements.openCodexButton.textContent = t(state.locale, "openCodex");
  elements.starButton.textContent = t(state.locale, "star");
  elements.xiaohongshuButton.textContent = t(state.locale, "xiaohongshu");
  elements.previousPageButton.textContent = t(state.locale, "previous");
  elements.nextPageButton.textContent = t(state.locale, "next");
  elements.quotaMonitorLabel.textContent = t(state.locale, "quotaMonitor");
  elements.localeEnButton.textContent = t(state.locale, "languageEnglish");
  elements.localeZhButton.textContent = t(state.locale, "languageChinese");
  elements.localeEnButton.classList.toggle("is-active", state.locale === "en");
  elements.localeZhButton.classList.toggle("is-active", state.locale === "zh-CN");
  elements.localeEnButton.setAttribute("aria-pressed", state.locale === "en" ? "true" : "false");
  elements.localeZhButton.setAttribute("aria-pressed", state.locale === "zh-CN" ? "true" : "false");
  for (const button of elements.localeButtons) {
    const isActive = button.dataset.setLocale === state.locale;
    button.classList.toggle("is-active", isActive);
    button.setAttribute("aria-pressed", isActive ? "true" : "false");
  }
  renderThemeOptions();
  elements.dialogTitle.textContent = t(state.locale, "addProfileTitle");
  elements.dialogCopy.innerHTML = t(state.locale, "addProfileCopy")
    .replace("auth.json", "<code>auth.json</code>")
    .replace("profile.json", "<code>profile.json</code>");
  elements.renameDialogTitle.textContent = t(state.locale, "renameProfileTitle");
  elements.renameDialogCopy.textContent = t(state.locale, "renameProfileCopy");
  if (hasDeleteProfileUi) {
    elements.deleteProfileDialogTitle!.textContent = t(state.locale, "deleteProfileTitle");
    elements.deleteProfileDialogCopy!.textContent = t(state.locale, "deleteProfileCopy");
    elements.deleteProfileButton!.textContent = t(state.locale, "deleteCard");
    elements.clearProfileAccountButton!.textContent = t(state.locale, "clearAccount");
    elements.cancelDeleteProfileButton!.textContent = t(state.locale, "cancel");
  }
  const baseUrlCopy = t(state.locale, isWindowsUiTarget ? "baseUrlWindowsCopy" : "baseUrlCopy");
  elements.baseUrlDialogTitle.textContent = t(state.locale, "baseUrlTitle");
  elements.baseUrlDialogCopy.textContent = baseUrlCopy;
  elements.folderNameLabel.textContent = t(state.locale, "folderName");
  const accountDetailToggle = elements.settingsShowAccountDetailToggle;
  if (accountDetailToggle instanceof HTMLInputElement) {
    accountDetailToggle.checked = state.showAccountDetail;
  }
  elements.addBaseUrlLabel.textContent = t(state.locale, "baseUrlLabel");
  elements.addBaseUrlInput.placeholder = t(state.locale, "baseUrlPlaceholder");
  elements.addBaseUrlCopy.textContent = baseUrlCopy;
  elements.renameFolderNameLabel.textContent = t(state.locale, "folderName");
  elements.baseUrlLabel.textContent = t(state.locale, "baseUrlLabel");
  elements.baseUrlInput.placeholder = t(state.locale, "baseUrlPlaceholder");
  elements.cancelAddProfileButton.textContent = t(state.locale, "cancel");
  elements.submitAddProfileButton.textContent = t(state.locale, "create");
  elements.cancelRenameProfileButton.textContent = t(state.locale, "cancel");
  elements.submitRenameProfileButton.textContent = t(state.locale, "rename");
  elements.cancelBaseUrlButton.textContent = t(state.locale, "cancel");
  elements.submitBaseUrlButton.textContent = t(state.locale, "save");
  elements.codexCliDialogTitle.textContent = t(state.locale, "codexCliDialogTitle");
  elements.codexCliDialogCopy.textContent = t(state.locale, "codexCliDialogCopy");
  elements.codexCliCurrentLabel.textContent = t(state.locale, "codexCliCurrentLabel");
  elements.codexCliInputLabel.textContent = t(state.locale, "codexCliInputLabel");
  elements.codexCliInput.placeholder = t(state.locale, "codexCliInputPlaceholder");
  elements.codexCliSuggestionsHeading.textContent = t(state.locale, "codexCliSuggestionsHeading");
  elements.cancelCodexCliButton.textContent = t(state.locale, "cancel");
  elements.clearCodexCliButton.textContent = t(state.locale, "codexCliClearOverride");
  elements.submitCodexCliButton.textContent = t(state.locale, "save");
  elements.settingsCodexCliLabel.textContent = t(state.locale, "settingsCodexCli");
  elements.settingsCodexCliButton.textContent = t(state.locale, "settingsCodexCliChange");
  elements.settingsCodexCliDetectButton.textContent = t(state.locale, "settingsCodexCliDetect");
  // Version label is locale-independent but lives next to the i18n
  // settings rows; set it here so a single render pass paints both.
  // `__CODEX_APP_VERSION__` is injected by Vite from `package.json` so
  // it stays in lock-step with the Cargo version automatically.
  elements.settingsVersionValue.textContent = __CODEX_APP_VERSION__;
  if (elements.historySearchInput) {
    elements.historySearchInput.placeholder = t(state.locale, "sessionSearch");
  }
  if (elements.historySearchButton) {
    elements.historySearchButton.title = t(state.locale, "sessionSearch");
    elements.historySearchButton.setAttribute("aria-label", t(state.locale, "sessionSearch"));
  }
  if (elements.historyRefreshSessionsButton) {
    elements.historyRefreshSessionsButton.title = t(state.locale, "sessionRefresh");
    elements.historyRefreshSessionsButton.setAttribute("aria-label", t(state.locale, "sessionRefresh"));
  }
  if (elements.historyCopyResumeButton) {
    elements.historyCopyResumeButton.title = t(state.locale, "sessionCopyCommand");
    elements.historyCopyResumeButton.setAttribute("aria-label", t(state.locale, "sessionCopyCommand"));
  }
}
