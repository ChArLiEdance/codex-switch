import type {
  CodexSessionMessage,
  CodexSessionMeta,
  CodexPromptEntry,
  CodexSkillEntry,
  CurrentCard,
  DashboardViewModel,
  PagingInfo,
  ProfileCard,
  QuotaSummary,
  QuotaWindow,
  ShellRoute,
  UpdateCheckResponse,
  UpdateDownloadProgress,
  UsageStatsResponse,
  UsageStatsRangePreset,
  UsageStatsRefreshSeconds,
} from "@front-shared/types";
import { t, type MessageKey } from "@front-shared/i18n";
import { state } from "@front-shared/state";
import { getThemeOption, isThemeId } from "@front-shared/theme";

const isWindowsUiTarget = __CODEX_UI_TARGET__ === "windows";
const defaultRoute: ShellRoute = "profiles";
const usesUnifiedProfileCards = true as boolean;
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

function requiredChild<T extends HTMLElement>(
  parent: ParentNode,
  selector: string,
): T {
  const element = parent.querySelector(selector);
  if (!(element instanceof HTMLElement)) {
    throw new Error(`Missing required child: ${selector}`);
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
  settingsExportBackupButton: requiredElement<HTMLButtonElement>("settings-export-backup-button"),
  settingsImportBackupButton: requiredElement<HTMLButtonElement>("settings-import-backup-button"),
  settingsVersionValue: requiredElement<HTMLSpanElement>("settings-version-value"),
  settingsUsageProfileSelect: document.getElementById("settings-usage-profile-select") as HTMLSelectElement | null,
  settingsUsageEnabledToggle: document.getElementById("settings-usage-enabled-toggle") as HTMLInputElement | null,
  settingsUsageTimeoutInput: document.getElementById("settings-usage-timeout-input") as HTMLInputElement | null,
  settingsUsageIntervalInput: document.getElementById("settings-usage-interval-input") as HTMLInputElement | null,
  settingsUsageSaveButton: document.getElementById("settings-usage-save-button") as HTMLButtonElement | null,
  settingsQuotaAlertProfileSelect: document.getElementById("settings-quota-alert-profile-select") as HTMLSelectElement | null,
  settingsQuotaAlertEnabledToggle: document.getElementById("settings-quota-alert-enabled-toggle") as HTMLInputElement | null,
  settingsQuotaAlertFiveHourToggle: document.getElementById("settings-quota-alert-five-hour-toggle") as HTMLInputElement | null,
  settingsQuotaAlertWeeklyToggle: document.getElementById("settings-quota-alert-weekly-toggle") as HTMLInputElement | null,
  settingsQuotaAlertSaveButton: document.getElementById("settings-quota-alert-save-button") as HTMLButtonElement | null,
  usageProfileFilter: document.getElementById("usage-profile-filter") as HTMLSelectElement | null,
  usageRangeFilter: document.getElementById("usage-range-filter") as HTMLSelectElement | null,
  usageRefreshIntervalFilter: document.getElementById("usage-refresh-interval-filter") as HTMLSelectElement | null,
  usageRefreshButton: document.getElementById("usage-refresh-button") as HTMLButtonElement | null,
  usageCustomRangePanel: document.getElementById("usage-custom-range-panel") as HTMLDivElement | null,
  usageCustomStartInput: document.getElementById("usage-custom-start-input") as HTMLInputElement | null,
  usageCustomEndInput: document.getElementById("usage-custom-end-input") as HTMLInputElement | null,
  usageCustomApplyButton: document.getElementById("usage-custom-apply-button") as HTMLButtonElement | null,
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
  skillsCount: document.getElementById("skills-count"),
  skillsList: document.getElementById("skills-list"),
  skillsRefreshButton: document.getElementById("skills-refresh-button") as HTMLButtonElement | null,
  skillsOpenFolderButton: document.getElementById("skills-open-folder-button") as HTMLButtonElement | null,
  skillEditorHeading: document.getElementById("skill-editor-heading"),
  skillNewButton: document.getElementById("skill-new-button") as HTMLButtonElement | null,
  skillDeleteButton: document.getElementById("skill-delete-button") as HTMLButtonElement | null,
  skillSaveButton: document.getElementById("skill-save-button") as HTMLButtonElement | null,
  skillNameInput: document.getElementById("skill-name-input") as HTMLInputElement | null,
  skillDescriptionInput: document.getElementById("skill-description-input") as HTMLInputElement | null,
  skillContentInput: document.getElementById("skill-content-input") as HTMLTextAreaElement | null,
  promptsCount: document.getElementById("prompts-count"),
  promptsList: document.getElementById("prompts-list"),
  promptsRefreshButton: document.getElementById("prompts-refresh-button") as HTMLButtonElement | null,
  promptsImportButton: document.getElementById("prompts-import-button") as HTMLButtonElement | null,
  promptEditorHeading: document.getElementById("prompt-editor-heading"),
  promptNewButton: document.getElementById("prompt-new-button") as HTMLButtonElement | null,
  promptEnableButton: document.getElementById("prompt-enable-button") as HTMLButtonElement | null,
  promptDeleteButton: document.getElementById("prompt-delete-button") as HTMLButtonElement | null,
  promptSaveButton: document.getElementById("prompt-save-button") as HTMLButtonElement | null,
  promptNameInput: document.getElementById("prompt-name-input") as HTMLInputElement | null,
  promptDescriptionInput: document.getElementById("prompt-description-input") as HTMLInputElement | null,
  promptContentInput: document.getElementById("prompt-content-input") as HTMLTextAreaElement | null,
  settingsShowAccountDetailToggle: document.getElementById("settings-show-account-detail-toggle"),
  settingsRestartCliToggle: document.getElementById("settings-restart-cli-toggle") as HTMLInputElement | null,
  settingsRestartVscodeToggle: document.getElementById("settings-restart-vscode-toggle") as HTMLInputElement | null,
  settingsRestartDesktopToggle: document.getElementById("settings-restart-desktop-toggle") as HTMLInputElement | null,
  settingsCloseBehaviorSelect: document.getElementById("settings-close-behavior-select") as HTMLSelectElement | null,
  restartChoiceDialog: requiredElement<HTMLDialogElement>("restart-choice-dialog"),
  restartChoiceCliToggle: requiredElement<HTMLInputElement>("restart-choice-cli-toggle"),
  restartChoiceVscodeToggle: requiredElement<HTMLInputElement>("restart-choice-vscode-toggle"),
  restartChoiceDesktopToggle: requiredElement<HTMLInputElement>("restart-choice-desktop-toggle"),
  restartChoiceCancelButton: requiredElement<HTMLButtonElement>("restart-choice-cancel-button"),
  restartChoiceConfirmButton: requiredElement<HTMLButtonElement>("restart-choice-confirm-button"),
  closeChoiceDialog: requiredElement<HTMLDialogElement>("close-choice-dialog"),
  closeChoiceRemember: requiredElement<HTMLInputElement>("close-choice-remember"),
  closeChoiceHideButton: requiredElement<HTMLButtonElement>("close-choice-hide-button"),
  closeChoiceQuitButton: requiredElement<HTMLButtonElement>("close-choice-quit-button"),
  closeChoiceCancelButton: requiredElement<HTMLButtonElement>("close-choice-cancel-button"),
  updateDialog: requiredElement<HTMLDialogElement>("update-dialog"),
  updateDialogCopy: requiredElement<HTMLParagraphElement>("update-dialog-copy"),
  updateDialogNotes: requiredElement<HTMLDivElement>("update-dialog-notes"),
  updateProgressPanel: requiredElement<HTMLDivElement>("update-progress-panel"),
  updateProgressTrack: requiredChild<HTMLDivElement>(
    requiredElement<HTMLDivElement>("update-progress-panel"),
    ".update-progress-track",
  ),
  updateProgressFill: requiredElement<HTMLSpanElement>("update-progress-fill"),
  updateProgressStatus: requiredElement<HTMLParagraphElement>("update-progress-status"),
  updateDialogLaterButton: requiredElement<HTMLButtonElement>("update-dialog-later-button"),
  updateDialogRetryButton: requiredElement<HTMLButtonElement>("update-dialog-retry-button"),
  updateDialogRestartButton: requiredElement<HTMLButtonElement>("update-dialog-restart-button"),
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
  profileBackupDialog: requiredElement<HTMLDialogElement>("profile-backup-dialog"),
  profileBackupForm: requiredElement<HTMLFormElement>("profile-backup-form"),
  profileBackupDialogTitle: requiredElement<HTMLHeadingElement>("profile-backup-dialog-title"),
  profileBackupDialogCopy: requiredElement<HTMLParagraphElement>("profile-backup-dialog-copy"),
  profileBackupPathLabel: requiredElement<HTMLSpanElement>("profile-backup-path-label"),
  profileBackupPathInput: requiredElement<HTMLInputElement>("profile-backup-path-input"),
  profileBackupPasswordLabel: requiredElement<HTMLSpanElement>("profile-backup-password-label"),
  profileBackupPasswordInput: requiredElement<HTMLInputElement>("profile-backup-password-input"),
  profileBackupOverwriteRow: requiredElement<HTMLLabelElement>("profile-backup-overwrite-row"),
  profileBackupOverwriteInput: requiredElement<HTMLInputElement>("profile-backup-overwrite-input"),
  profileBackupOverwriteLabel: requiredElement<HTMLSpanElement>("profile-backup-overwrite-label"),
  profileBackupDialogError: requiredElement<HTMLParagraphElement>("profile-backup-dialog-error"),
  cancelProfileBackupButton: requiredElement<HTMLButtonElement>("cancel-profile-backup-button"),
  submitProfileBackupButton: requiredElement<HTMLButtonElement>("submit-profile-backup-button"),
  switchHealthDialog: requiredElement<HTMLDialogElement>("switch-health-dialog"),
  switchHealthDialogTitle: requiredElement<HTMLHeadingElement>("switch-health-dialog-title"),
  switchHealthDialogCopy: requiredElement<HTMLParagraphElement>("switch-health-dialog-copy"),
  switchHealthList: requiredElement<HTMLDivElement>("switch-health-list"),
  switchHealthDialogError: requiredElement<HTMLParagraphElement>("switch-health-dialog-error"),
  switchHealthCancelButton: requiredElement<HTMLButtonElement>("switch-health-cancel-button"),
  switchHealthConfirmButton: requiredElement<HTMLButtonElement>("switch-health-confirm-button"),
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
        button.classList.add("is-pressing");
        window.setTimeout(() => button.classList.remove("is-pressing"), 160);
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
  elements.updateDialogNotes.hidden = !update.notes;
  elements.updateDialogNotes.textContent = update.notes ?? "";
  setUpdateProgress({
    phase: "ready",
    received_bytes: 0,
    total_bytes: null,
    percent: 0,
    message: t(state.locale, "updateProgressReady"),
  });
  elements.updateProgressPanel.hidden = true;
  elements.updateDialogOpenButton.hidden = false;
  elements.updateDialogOpenButton.disabled = false;
  elements.updateDialogRetryButton.hidden = true;
  elements.updateDialogRetryButton.disabled = false;
  elements.updateDialogRestartButton.hidden = true;
  elements.updateDialogRestartButton.disabled = false;
  elements.updateDialogLaterButton.disabled = false;

  if (!elements.updateDialog.open) {
    elements.updateDialog.showModal();
  }
}

function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) {
    return "0 B";
  }
  const units = ["B", "KB", "MB", "GB"];
  let value = bytes;
  let unitIndex = 0;
  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024;
    unitIndex += 1;
  }
  return `${value >= 10 || unitIndex === 0 ? value.toFixed(0) : value.toFixed(1)} ${units[unitIndex]}`;
}

function progressMessage(progress: UpdateDownloadProgress): string {
  const percent = progress.percent ?? 0;
  if (progress.phase === "downloading") {
    const base = t(state.locale, "updateProgressDownloading", { percent: String(percent) });
    if (progress.total_bytes && progress.total_bytes > 0) {
      return `${base} · ${formatBytes(progress.received_bytes)} / ${formatBytes(progress.total_bytes)}`;
    }
    if (progress.received_bytes > 0) {
      return `${base} · ${formatBytes(progress.received_bytes)}`;
    }
    return base;
  }
  if (progress.phase === "opening") {
    return t(state.locale, "updateProgressOpening");
  }
  if (progress.phase === "opened") {
    return t(state.locale, "updateProgressOpened");
  }
  if (progress.phase === "failed") {
    return progress.message || t(state.locale, "updateProgressFailed");
  }
  if (progress.phase === "restarting") {
    return t(state.locale, "updateRestarting");
  }
  return progress.message || t(state.locale, "updateProgressReady");
}

export function setUpdateProgress(progress: UpdateDownloadProgress): void {
  const percent = Math.max(0, Math.min(100, progress.percent ?? 0));
  elements.updateProgressPanel.hidden = false;
  elements.updateProgressFill.style.width = `${percent}%`;
  elements.updateProgressTrack.setAttribute("aria-valuenow", String(percent));
  elements.updateProgressStatus.textContent = progressMessage(progress);
}

export function setUpdateInstalling(isInstalling: boolean): void {
  elements.updateDialogOpenButton.disabled = isInstalling;
  elements.updateDialogLaterButton.disabled = isInstalling;
  elements.updateDialogRetryButton.disabled = isInstalling;
  elements.updateDialogRestartButton.disabled = isInstalling;
  elements.updateProgressPanel.hidden = false;
}

export function showUpdateInstallError(message: string): void {
  setUpdateProgress({
    phase: "failed",
    received_bytes: 0,
    total_bytes: null,
    percent: null,
    message,
  });
  elements.updateDialogOpenButton.hidden = true;
  elements.updateDialogRetryButton.hidden = false;
  elements.updateDialogRestartButton.hidden = true;
  elements.updateDialogLaterButton.disabled = false;
}

export function showUpdateInstallComplete(): void {
  setUpdateProgress({
    phase: "opened",
    received_bytes: 0,
    total_bytes: null,
    percent: 100,
    message: t(state.locale, "updateProgressOpened"),
  });
  elements.updateDialogOpenButton.hidden = true;
  elements.updateDialogRetryButton.hidden = true;
  elements.updateDialogRestartButton.hidden = false;
  elements.updateDialogLaterButton.disabled = false;
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
  document.body.classList.toggle("mac-detail-route", state.route !== "profiles");

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
    renderGeneralSettingsControls();
    renderQuotaAlertSettingsControls([]);
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
  renderGeneralSettingsControls();
  renderUsageSettingsControls(profiles);
  renderQuotaAlertSettingsControls(profiles);
  renderUsageStats();
  renderSessionManager();
}

function formatCompactNumber(value: number): string {
  if (!Number.isFinite(value)) {
    return "--";
  }
  if (value >= 10_000) {
    if (state.locale === "zh-CN") {
      return `${(value / 10_000).toFixed(1)}万`;
    }
    return `${(value / 1_000_000).toFixed(2)} millions`;
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

function formatDateOnly(seconds: number): string {
  if (!seconds) {
    return "--";
  }
  return new Date(seconds * 1000).toLocaleDateString(state.locale === "zh-CN" ? "zh-CN" : "en-US", {
    month: "2-digit",
    day: "2-digit",
  });
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

function formatTrendAxisLabel(seconds: number, showDateOnly: boolean): string {
  return showDateOnly ? formatDateOnly(seconds) : formatDateTime(seconds);
}

function toDateTimeLocalValue(seconds: number | null): string {
  if (!seconds) {
    return "";
  }
  const date = new Date(seconds * 1000);
  const offsetMs = date.getTimezoneOffset() * 60_000;
  return new Date(date.getTime() - offsetMs).toISOString().slice(0, 16);
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

function renderQuotaAlertSettingsControls(profiles: ProfileCard[]): void {
  const select = elements.settingsQuotaAlertProfileSelect;
  if (!select) {
    return;
  }
  const selected = state.quotaAlertProfile ?? state.currentProfile ?? profiles[0]?.folder_name ?? "";
  if (state.quotaAlertProfile !== selected) {
    state.quotaAlertProfile = selected || null;
  }
  select.innerHTML = profiles
    .map((profile) => (
      `<option value="${escapeHtml(profile.folder_name)}"${profile.folder_name === selected ? " selected" : ""}>${escapeHtml(profileDisplayTitle(profile))}</option>`
    ))
    .join("");
  const settings = selected ? state.quotaAlertSettingsByProfile[selected] : null;
  if (elements.settingsQuotaAlertEnabledToggle) {
    elements.settingsQuotaAlertEnabledToggle.checked = Boolean(settings?.enabled);
  }
  if (elements.settingsQuotaAlertFiveHourToggle) {
    elements.settingsQuotaAlertFiveHourToggle.checked = settings?.five_hour_enabled ?? true;
  }
  if (elements.settingsQuotaAlertWeeklyToggle) {
    elements.settingsQuotaAlertWeeklyToggle.checked = settings?.weekly_enabled ?? true;
  }
}

function renderGeneralSettingsControls(): void {
  if (elements.settingsRestartCliToggle) {
    elements.settingsRestartCliToggle.checked = state.switchRestartTargets.cli;
  }
  if (elements.settingsRestartVscodeToggle) {
    elements.settingsRestartVscodeToggle.checked = state.switchRestartTargets.vscode;
  }
  if (elements.settingsRestartDesktopToggle) {
    elements.settingsRestartDesktopToggle.checked = state.switchRestartTargets.codex_desktop;
  }
  if (elements.settingsCloseBehaviorSelect) {
    elements.settingsCloseBehaviorSelect.value = state.closeBehavior;
  }
}

function renderStatsFilters(options: {
  stats: UsageStatsResponse | null;
  profileSelect: HTMLSelectElement | null;
  rangeSelect: HTMLSelectElement | null;
  refreshSelect?: HTMLSelectElement | null;
  customPanel?: HTMLElement | null;
  customStartInput?: HTMLInputElement | null;
  customEndInput?: HTMLInputElement | null;
  selectedProfile: string | null;
  selectedRange: UsageStatsRangePreset;
  selectedRefreshSeconds?: UsageStatsRefreshSeconds;
  customStartAt?: number | null;
  customEndAt?: number | null;
}): void {
  if (options.rangeSelect) {
    options.rangeSelect.value = options.selectedRange;
  }
  if (options.refreshSelect && options.selectedRefreshSeconds !== undefined) {
    options.refreshSelect.value = String(options.selectedRefreshSeconds);
  }
  if (options.customPanel) {
    options.customPanel.hidden = options.selectedRange !== "custom";
  }
  if (options.customStartInput) {
    options.customStartInput.value = toDateTimeLocalValue(options.customStartAt ?? null);
  }
  if (options.customEndInput) {
    options.customEndInput.value = toDateTimeLocalValue(options.customEndAt ?? null);
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
    refreshSelect: elements.usageRefreshIntervalFilter,
    customPanel: elements.usageCustomRangePanel,
    customStartInput: elements.usageCustomStartInput,
    customEndInput: elements.usageCustomEndInput,
    selectedProfile: state.usageStatsProfile,
    selectedRange: state.usageStatsRange,
    selectedRefreshSeconds: state.usageStatsRefreshSeconds,
    customStartAt: state.usageStatsCustomStartAt,
    customEndAt: state.usageStatsCustomEndAt,
  });
}

function usageLineColor(kind: "cost" | "input" | "output" | "cacheCreate" | "cacheHit"): string {
  switch (kind) {
    case "cost":
      return "#ff4770";
    case "input":
      return "#2f7dff";
    case "output":
      return "#22c55e";
    case "cacheCreate":
      return "#f97316";
    case "cacheHit":
      return "#a855f7";
  }
}

function smoothSvgPath(points: Array<{ x: number; y: number }>): string {
  if (points.length === 0) {
    return "";
  }
  if (points.length === 1) {
    return `M${points[0].x.toFixed(1)} ${points[0].y.toFixed(1)}`;
  }
  const commands = [`M${points[0].x.toFixed(1)} ${points[0].y.toFixed(1)}`];
  for (let index = 0; index < points.length - 1; index += 1) {
    const previous = points[Math.max(0, index - 1)];
    const current = points[index];
    const next = points[index + 1];
    const afterNext = points[Math.min(points.length - 1, index + 2)];
    const control1 = {
      x: current.x + (next.x - previous.x) / 6,
      y: current.y + (next.y - previous.y) / 6,
    };
    const control2 = {
      x: next.x - (afterNext.x - current.x) / 6,
      y: next.y - (afterNext.y - current.y) / 6,
    };
    commands.push(
      `C${control1.x.toFixed(1)} ${control1.y.toFixed(1)} ${control2.x.toFixed(1)} ${control2.y.toFixed(1)} ${next.x.toFixed(1)} ${next.y.toFixed(1)}`,
    );
  }
  return commands.join(" ");
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
  const chartPoints = stats.trends.map((point, index) => ({
    index,
    point,
    x: x(index),
    tokenY: yToken(point.real_total_tokens),
    inputY: yToken(point.input_tokens),
    outputY: yToken(point.output_tokens),
    cacheCreateY: yToken(point.cache_creation_tokens),
    cacheHitY: yToken(point.cache_read_tokens),
    costY: yCost(point.total_cost_usd),
  }));
  const tokenPath = smoothSvgPath(chartPoints.map((entry) => ({ x: entry.x, y: entry.tokenY })));
  const inputPath = smoothSvgPath(chartPoints.map((entry) => ({ x: entry.x, y: entry.inputY })));
  const outputPath = smoothSvgPath(chartPoints.map((entry) => ({ x: entry.x, y: entry.outputY })));
  const cacheCreatePath = smoothSvgPath(chartPoints.map((entry) => ({ x: entry.x, y: entry.cacheCreateY })));
  const cacheHitPath = smoothSvgPath(chartPoints.map((entry) => ({ x: entry.x, y: entry.cacheHitY })));
  const costPath = smoothSvgPath(chartPoints.map((entry) => ({ x: entry.x, y: entry.costY })));
  const areaPath = `${tokenPath} L${chartPoints[chartPoints.length - 1].x.toFixed(1)} ${height - pad.bottom} L${pad.left} ${height - pad.bottom} Z`;
  const tickStep = Math.max(1, Math.ceil(stats.trends.length / 4));
  const tickIndexes = Array.from(new Set([
    0,
    ...stats.trends.map((_, index) => index).filter((index) => index % tickStep === 0),
    stats.trends.length - 1,
  ])).sort((left, right) => left - right);
  const showDateOnly = stats.end_at - stats.start_at > 24 * 60 * 60;
  const tooltipRows = [
    { key: "input", color: usageLineColor("input"), label: t(state.locale, "usageInput") },
    { key: "output", color: usageLineColor("output"), label: t(state.locale, "usageOutput") },
    { key: "cacheCreate", color: usageLineColor("cacheCreate"), label: t(state.locale, "usageCacheCreate") },
    { key: "cacheHit", color: usageLineColor("cacheHit"), label: t(state.locale, "usageCacheHit") },
    { key: "cost", color: usageLineColor("cost"), label: t(state.locale, "usageCost") },
  ] as const;
  const pointNodes = chartPoints.map((entry) => (
    `<circle cx="${entry.x.toFixed(1)}" cy="${entry.tokenY.toFixed(1)}" r="4" class="usage-point" data-usage-point="${entry.index}" />`
  )).join("");
  const tooltipWidth = 178;
  const tooltipHeight = 128;
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
      <path d="${tokenPath}" class="usage-line usage-line--total" />
      <path d="${costPath}" class="usage-line usage-line--cost" />
      <path d="${cacheCreatePath}" class="usage-line usage-line--cache-create" />
      <path d="${cacheHitPath}" class="usage-line usage-line--cache" />
      <path d="${inputPath}" class="usage-line usage-line--input" />
      <path d="${outputPath}" class="usage-line usage-line--output" />
      ${pointNodes}
      <rect x="${pad.left}" y="${pad.top}" width="${width - pad.left - pad.right}" height="${height - pad.top - pad.bottom}" class="usage-hover-capture" />
      <g class="usage-active-layer">
        <line x1="${pad.left}" y1="${pad.top}" x2="${pad.left}" y2="${height - pad.bottom}" class="usage-hover-line" data-usage-cursor />
        <circle cx="${pad.left}" cy="${pad.top}" r="6" class="usage-active-dot" data-usage-active-dot />
        <g class="usage-hover-tooltip" data-usage-tooltip>
          <rect width="${tooltipWidth}" height="${tooltipHeight}" rx="10" />
          <text x="12" y="22" class="usage-tooltip-title" data-usage-tooltip-title>--</text>
          ${tooltipRows.map((row, rowIndex) => `
            <circle cx="14" cy="${44 + rowIndex * 17}" r="3" fill="${row.color}" />
            <text x="24" y="${48 + rowIndex * 17}" class="usage-tooltip-label" data-usage-tooltip-value="${row.key}">${escapeHtml(row.label)}: --</text>
          `).join("")}
        </g>
      </g>
      ${tickIndexes.map((tickIndex, index) => {
        const point = stats.trends[tickIndex];
        return `<text x="${x(tickIndex)}" y="${height - 12}" class="usage-axis-label" text-anchor="${index === 0 ? "start" : "middle"}">${escapeHtml(formatTrendAxisLabel(point.timestamp, showDateOnly))}</text>`;
      }).join("")}
      <text x="${pad.left}" y="${pad.top + 8}" class="usage-axis-label">tokens</text>
      <text x="${width - pad.right}" y="${pad.top + 8}" class="usage-axis-label" text-anchor="end">$</text>
    </svg>
    <div class="usage-legend">
      <span class="legend-cost">${escapeHtml(t(state.locale, "usageCost"))}</span>
      <span class="legend-cache-create">${escapeHtml(t(state.locale, "usageCacheCreate"))}</span>
      <span class="legend-cache">${escapeHtml(t(state.locale, "usageCacheHit"))}</span>
      <span class="legend-input">${escapeHtml(t(state.locale, "usageInput"))}</span>
      <span class="legend-output">${escapeHtml(t(state.locale, "usageOutput"))}</span>
    </div>
  `;

  const svg = container.querySelector<SVGSVGElement>("svg");
  const activeLayer = container.querySelector<SVGGElement>(".usage-active-layer");
  const cursor = container.querySelector<SVGLineElement>("[data-usage-cursor]");
  const activeDot = container.querySelector<SVGCircleElement>("[data-usage-active-dot]");
  const tooltip = container.querySelector<SVGGElement>("[data-usage-tooltip]");
  const tooltipTitle = container.querySelector<SVGTextElement>("[data-usage-tooltip-title]");
  const tooltipValues = Array.from(container.querySelectorAll<SVGTextElement>("[data-usage-tooltip-value]"));
  const pointElements = Array.from(container.querySelectorAll<SVGCircleElement>("[data-usage-point]"));
  let activeIndex = -1;
  let pendingClientX: number | null = null;
  let pendingFrame = 0;

  const formatTooltipRow = (key: string, point: UsageStatsResponse["trends"][number]): string => {
    if (key === "cost") {
      return formatMoney(point.total_cost_usd);
    }
    if (key === "input") {
      return formatFullNumber(point.input_tokens);
    }
    if (key === "output") {
      return formatFullNumber(point.output_tokens);
    }
    if (key === "cacheCreate") {
      return formatFullNumber(point.cache_creation_tokens);
    }
    return formatFullNumber(point.cache_read_tokens);
  };

  const setActivePoint = (nextIndex: number): void => {
    if (!activeLayer || !cursor || !activeDot || !tooltip || !tooltipTitle || nextIndex === activeIndex) {
      return;
    }
    activeIndex = nextIndex;
    const entry = chartPoints[nextIndex];
    const tooltipX = Math.min(width - pad.right - tooltipWidth, Math.max(pad.left, entry.x + 12));
    const tooltipY = Math.max(pad.top, entry.tokenY - tooltipHeight - 10);
    activeLayer.classList.add("is-active");
    cursor.setAttribute("x1", entry.x.toFixed(1));
    cursor.setAttribute("x2", entry.x.toFixed(1));
    activeDot.setAttribute("cx", entry.x.toFixed(1));
    activeDot.setAttribute("cy", entry.tokenY.toFixed(1));
    tooltip.setAttribute("transform", `translate(${tooltipX.toFixed(1)} ${tooltipY.toFixed(1)})`);
    tooltipTitle.textContent = formatDateTime(entry.point.timestamp);
    for (const valueNode of tooltipValues) {
      const key = valueNode.dataset.usageTooltipValue ?? "";
      const row = tooltipRows.find((candidate) => candidate.key === key);
      valueNode.textContent = `${row?.label ?? key}: ${formatTooltipRow(key, entry.point)}`;
    }
    for (const pointElement of pointElements) {
      pointElement.classList.toggle("is-active", pointElement.dataset.usagePoint === String(nextIndex));
    }
  };

  const nearestIndexForClientX = (clientX: number): number => {
    if (!svg) {
      return stats.trends.length - 1;
    }
    const rect = svg.getBoundingClientRect();
    const relativeX = ((clientX - rect.left) / Math.max(1, rect.width)) * width;
    let nearestIndex = 0;
    let nearestDistance = Number.POSITIVE_INFINITY;
    stats.trends.forEach((_, index) => {
      const distance = Math.abs(relativeX - x(index));
      if (distance < nearestDistance) {
        nearestDistance = distance;
        nearestIndex = index;
      }
    });
    return nearestIndex;
  };

  setActivePoint(stats.trends.length - 1);
  svg?.addEventListener("pointermove", (event) => {
    pendingClientX = event.clientX;
    if (pendingFrame) {
      return;
    }
    pendingFrame = window.requestAnimationFrame(() => {
      pendingFrame = 0;
      if (pendingClientX != null) {
        setActivePoint(nearestIndexForClientX(pendingClientX));
      }
    });
  });
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

const SESSION_MESSAGE_COLLAPSE_THRESHOLD = 3000;
const SESSION_MESSAGE_COLLAPSED_LENGTH = 1500;

function renderSessionMessage(message: CodexSessionMessage, index: number): string {
  const key = `${state.selectedCodexSessionPath ?? "session"}:${index}`;
  const expanded = state.expandedSessionMessages.includes(key);
  const isLong = message.content.length > SESSION_MESSAGE_COLLAPSE_THRESHOLD;
  const collapsed = isLong && !expanded;
  const content = collapsed
    ? `${message.content.slice(0, SESSION_MESSAGE_COLLAPSED_LENGTH)}…`
    : message.content;
  const role = message.role.toLowerCase();
  return `
    <article class="session-message-card session-message-card--${escapeHtml(role || "unknown")}">
      <div class="session-message-head">
        <strong>${escapeHtml(roleLabel(message.role))}</strong>
        <span>${escapeHtml(formatSessionDateTime(message.ts))}</span>
      </div>
      <pre>${escapeHtml(content)}</pre>
      ${isLong ? `
        <button class="session-message-expand" type="button" data-session-message-key="${escapeHtml(key)}" aria-expanded="${expanded ? "true" : "false"}">
          ${escapeHtml(expanded ? t(state.locale, "sessionCollapseMessage") : t(state.locale, "sessionExpandMessage"))}
          <span>${escapeHtml(expanded ? "⌃" : `(${Math.round(message.content.length / 1000)}k) ⌄`)}</span>
        </button>
      ` : ""}
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
  const visibleSessions = sessions.slice(0, state.sessionVisibleCount);

  if (elements.historySessionCount) {
    elements.historySessionCount.textContent = String(sessions.length);
  }
  if (elements.historySearchInput && elements.historySearchInput.value !== state.sessionSearch) {
    elements.historySearchInput.value = state.sessionSearch;
  }

  elements.historySessionList.innerHTML = sessions.length
    ? [
      ...visibleSessions.map((session) => {
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
    }),
      sessions.length > visibleSessions.length
        ? `<button class="session-load-more" type="button" data-session-load-more="true">${escapeHtml(t(state.locale, "sessionLoadMore", {
          shown: String(visibleSessions.length),
          total: String(sessions.length),
        }))}</button>`
        : "",
    ].join("")
    : `<div class="session-empty">${escapeHtml(t(state.locale, "sessionNoSessions"))}</div>`;

  const selected = state.codexSessions.find((session) => session.source_path === state.selectedCodexSessionPath) ?? null;
  const hasSelected = Boolean(selected);
  if (elements.historyDetailEmpty) {
    elements.historyDetailEmpty.hidden = hasSelected;
    elements.historyDetailEmpty.classList.toggle("is-visible", !hasSelected);
  }
  if (elements.historyDetailBody) {
    elements.historyDetailBody.hidden = !hasSelected;
    elements.historyDetailBody.classList.toggle("is-visible", hasSelected);
  }
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
    const visibleMessages = state.codexSessionMessages.slice(0, state.sessionMessageVisibleCount);
    elements.historyMessageList.innerHTML = state.sessionMessagesLoading
      ? `<div class="session-empty session-empty--loading">${macIcon("refresh-cw", "cc-icon cc-icon--spin")}${escapeHtml(t(state.locale, "sessionLoading"))}</div>`
      : state.codexSessionMessages.length
      ? [
        ...visibleMessages.map(renderSessionMessage),
        state.codexSessionMessages.length > visibleMessages.length
          ? `<button class="session-load-more session-load-more--messages" type="button" data-session-message-load-more="true">${escapeHtml(t(state.locale, "sessionLoadMoreMessages", {
            shown: String(visibleMessages.length),
            total: String(state.codexSessionMessages.length),
          }))}</button>`
          : "",
      ].join("")
      : `<div class="session-empty">${escapeHtml(t(state.locale, "sessionNoMessages"))}</div>`;
  }
}

function toolUpdatedAt(value: number | null | undefined): string {
  if (!value) {
    return "--";
  }
  const ms = value > 10_000_000_000 ? value : value * 1000;
  return new Date(ms).toLocaleString(state.locale === "zh-CN" ? "zh-CN" : "en-US", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function firstParagraph(content: string): string {
  return content
    .split(/\n{2,}/)
    .map((value) => value.trim())
    .find((value) => value && !value.startsWith("#"))
    ?.slice(0, 150) ?? "";
}

function selectedSkill(): CodexSkillEntry | null {
  return state.codexSkills.find((skill) => skill.id === state.selectedCodexSkillId) ?? null;
}

function selectedPrompt(): CodexPromptEntry | null {
  return state.codexPrompts.find((prompt) => prompt.id === state.selectedCodexPromptId) ?? null;
}

function renderToolEmpty(label: string): string {
  return `<div class="tool-empty">${escapeHtml(label)}</div>`;
}

export function renderCodexSkills(): void {
  if (!elements.skillsList) {
    return;
  }
  if (elements.skillsCount) {
    elements.skillsCount.textContent = String(state.codexSkills.length);
  }

  if (state.codexSkillsLoading) {
    elements.skillsList.innerHTML = renderToolEmpty(t(state.locale, "loading"));
  } else if (state.codexSkills.length === 0) {
    elements.skillsList.innerHTML = renderToolEmpty(t(state.locale, "skillsEmpty"));
  } else {
    elements.skillsList.innerHTML = state.codexSkills.map((skill) => {
      const active = skill.id === state.selectedCodexSkillId;
      return `
        <button class="tool-list-item${active ? " is-active" : ""}" type="button" data-skill-id="${escapeHtml(skill.id)}">
          <span class="tool-list-icon">${macIcon("wrench", "cc-icon cc-icon--inline")}</span>
          <span class="tool-list-copy">
            <strong>${escapeHtml(skill.name)}</strong>
            <small>${escapeHtml(skill.description || firstParagraph(skill.content) || skill.id)}</small>
            <em>${escapeHtml(toolUpdatedAt(skill.updated_at))}</em>
          </span>
          <span class="session-list-chevron">›</span>
        </button>
      `;
    }).join("");
  }

  const skill = selectedSkill();
  if (elements.skillEditorHeading) {
    elements.skillEditorHeading.textContent = skill?.name || t(state.locale, "skillNewTitle");
  }
  if (elements.skillNameInput && elements.skillNameInput.value !== (skill?.name ?? "")) {
    elements.skillNameInput.value = skill?.name ?? "";
  }
  if (elements.skillDescriptionInput && elements.skillDescriptionInput.value !== (skill?.description ?? "")) {
    elements.skillDescriptionInput.value = skill?.description ?? "";
  }
  if (elements.skillContentInput && elements.skillContentInput.value !== (skill?.content ?? "")) {
    elements.skillContentInput.value = skill?.content ?? "";
  }
  if (elements.skillDeleteButton) {
    elements.skillDeleteButton.disabled = !skill;
  }
}

export function renderCodexPrompts(): void {
  if (!elements.promptsList) {
    return;
  }
  if (elements.promptsCount) {
    elements.promptsCount.textContent = String(state.codexPrompts.length);
  }

  if (state.codexPromptsLoading) {
    elements.promptsList.innerHTML = renderToolEmpty(t(state.locale, "loading"));
  } else if (state.codexPrompts.length === 0) {
    elements.promptsList.innerHTML = renderToolEmpty(t(state.locale, "promptsEmpty"));
  } else {
    elements.promptsList.innerHTML = state.codexPrompts.map((prompt) => {
      const active = prompt.id === state.selectedCodexPromptId;
      return `
        <button class="tool-list-item${active ? " is-active" : ""}" type="button" data-prompt-id="${escapeHtml(prompt.id)}">
          <span class="tool-list-icon">${macIcon("book-open", "cc-icon cc-icon--inline")}</span>
          <span class="tool-list-copy">
            <strong>${escapeHtml(prompt.name)}${prompt.enabled ? ` <span class="tool-enabled-badge">${escapeHtml(t(state.locale, "promptEnabled"))}</span>` : ""}</strong>
            <small>${escapeHtml(prompt.description || firstParagraph(prompt.content) || prompt.id)}</small>
            <em>${escapeHtml(toolUpdatedAt(prompt.updated_at ?? prompt.created_at))}</em>
          </span>
          <span class="session-list-chevron">›</span>
        </button>
      `;
    }).join("");
  }

  const prompt = selectedPrompt();
  if (elements.promptEditorHeading) {
    elements.promptEditorHeading.textContent = prompt?.name || t(state.locale, "promptNewTitle");
  }
  if (elements.promptNameInput && elements.promptNameInput.value !== (prompt?.name ?? "")) {
    elements.promptNameInput.value = prompt?.name ?? "";
  }
  if (elements.promptDescriptionInput && elements.promptDescriptionInput.value !== (prompt?.description ?? "")) {
    elements.promptDescriptionInput.value = prompt?.description ?? "";
  }
  if (elements.promptContentInput && elements.promptContentInput.value !== (prompt?.content ?? "")) {
    elements.promptContentInput.value = prompt?.content ?? "";
  }
  if (elements.promptEnableButton) {
    elements.promptEnableButton.disabled = !prompt || prompt.enabled;
  }
  if (elements.promptDeleteButton) {
    elements.promptDeleteButton.disabled = !prompt || prompt.enabled;
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

      if (usesUnifiedProfileCards) {
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

          <div class="profile-card-actions">
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

  for (const element of document.querySelectorAll<HTMLElement>("[data-i18n-key-title]")) {
    const key = element.dataset.i18nKeyTitle as MessageKey | undefined;
    if (key) {
      const value = t(state.locale, key);
      element.title = value;
      element.setAttribute("aria-label", value);
    }
  }

  for (const element of document.querySelectorAll<HTMLInputElement | HTMLTextAreaElement>(
    "[data-i18n-key-placeholder]",
  )) {
    const key = element.dataset.i18nKeyPlaceholder as MessageKey | undefined;
    if (key) {
      element.placeholder = t(state.locale, key);
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
