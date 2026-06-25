import { persistLocale, resolveInitialLocale, t, type Locale } from "@front-shared/i18n";
import { state } from "@front-shared/state";
import {
  applyTheme,
  getThemeOption,
  isThemeId,
  persistTheme,
  resolveEffectiveTheme,
  resolveInitialTheme,
  type ThemeId,
} from "@front-shared/theme";
import { persistShowAccountDetail } from "@front-shared/preferences";
import {
  applyCurrentQuota,
  applySnapshot,
  buildDashboardViewModel,
  persistProfileOrder,
} from "@front-shared/dashboard-view-model";
import {
  addProfile,
  cancelCodexLogin,
  checkUpdate,
  clearCodexCliPath,
  clearProfileAccount,
  deleteProfile,
  getCodexCliStatus,
  getCurrentLiveQuota,
  getProfilesSnapshot,
  getUsageQuerySettings,
  getUsageStats,
  loginCurrentProfile,
  openCodex,
  openContact,
  openReleases,
  openUrl,
  openXiaohongshu,
  openProfileFolder,
  loginProfile,
  refreshActiveProfileQuotaSilent,
  refreshAllOauthProfilePlansSilent,
  refreshProfile,
  redetectCodexCliPath,
  renameProfile,
  saveUsageQuerySettings,
  setCodexCliPath,
  switchProfile,
  updateProfileBaseUrl,
} from "@front-shared/tauri";
import type {
  CodexCliCandidate,
  CodexCliRedetectResult,
  CodexCliStatus,
  ProfileCard,
  UsageQuerySettings,
} from "@front-shared/types";
import {
  applyLocale,
  elements,
  renderCurrentCard,
  renderPaging,
  renderProfiles,
  renderShellOverview,
  renderShellRoute,
  renderThemeOptions,
  routeFromLocation,
  showUpdateDialog,
  showToast,
} from "@front-shared/render";

type ErrorWithCode = Error & {
  code?: string;
};

function rerenderDashboard(): void {
  state.route = routeFromLocation();
  applyLocale();
  renderShellRoute();

  const dashboard = buildDashboardViewModel();
  if (!dashboard) {
    renderPaging({ has_previous: false, has_next: false, page: 1, total_pages: 1 });
    renderShellOverview(null);
    return;
  }

  renderProfiles(
    dashboard,
    handleDeleteProfileClick,
    handleRenameProfileClick,
    handleSwitchProfile,
    handleRefreshProfile,
    handleBaseUrlProfileClick,
    (profile) => {
      void handleLoginProfile(profile);
    },
    (profile) => {
      void handleToggleQuotaProfile(profile);
    },
    handleReorderProfiles,
  );
  renderCurrentCard(dashboard);
  renderPaging(dashboard.paging);
  renderShellOverview(dashboard);
}

let renameSourceProfile: string | null = null;
let baseUrlSourceProfile: string | null = null;
let pendingUpdateReleaseUrl: string | null = null;
let deleteSourceProfile: string | null = null;
let pendingLoginRetry: (() => Promise<void>) | null = null;
let cancelledLoginProfile: string | null = null;
let usageConfigSourceProfile: string | null = null;

function isRefreshPending(profile: string): boolean {
  return state.refreshActiveProfiles.includes(profile);
}

function defaultUsageSettings(): UsageQuerySettings {
  return {
    enabled: false,
    timeout_seconds: 10,
    auto_query_interval_minutes: 5,
  };
}

function usageRangeBounds(range: "today" | "7d" | "30d"): { start_at: number; end_at: number } {
  const now = new Date();
  const start = new Date(now);
  start.setHours(0, 0, 0, 0);
  if (range === "7d") {
    start.setDate(start.getDate() - 6);
  } else if (range === "30d") {
    start.setDate(start.getDate() - 29);
  }
  return {
    start_at: Math.floor(start.getTime() / 1000),
    end_at: Math.floor(now.getTime() / 1000),
  };
}

async function ensureUsageSettings(profile: string): Promise<UsageQuerySettings> {
  const cached = state.usageSettingsByProfile[profile];
  if (cached) {
    return cached;
  }
  const settings = await getUsageQuerySettings(profile);
  state.usageSettingsByProfile = {
    ...state.usageSettingsByProfile,
    [profile]: settings,
  };
  return settings;
}

async function loadUsageSettingsForProfiles(profiles: ProfileCard[]): Promise<void> {
  const missing = profiles.filter((profile) => !state.usageSettingsByProfile[profile.folder_name]);
  if (missing.length === 0) {
    return;
  }
  const loaded = await Promise.all(
    missing.map(async (profile) => [profile.folder_name, await getUsageQuerySettings(profile.folder_name)] as const),
  );
  state.usageSettingsByProfile = {
    ...state.usageSettingsByProfile,
    ...Object.fromEntries(loaded),
  };
}

function usageSettingsFromInputs(prefix: "settings" | "dialog"): UsageQuerySettings {
  const enabled = prefix === "settings"
    ? elements.settingsUsageEnabledToggle?.checked
    : elements.usageConfigEnabledToggle?.checked;
  const timeout = prefix === "settings"
    ? elements.settingsUsageTimeoutInput?.valueAsNumber
    : elements.usageConfigTimeoutInput?.valueAsNumber;
  const interval = prefix === "settings"
    ? elements.settingsUsageIntervalInput?.valueAsNumber
    : elements.usageConfigIntervalInput?.valueAsNumber;
  return {
    enabled: Boolean(enabled),
    timeout_seconds: Number.isFinite(timeout) ? Math.max(1, Math.round(timeout ?? 10)) : 10,
    auto_query_interval_minutes: Number.isFinite(interval) ? Math.max(0, Math.round(interval ?? 5)) : 5,
  };
}

async function saveUsageSettingsForProfile(profile: string, settings: UsageQuerySettings): Promise<void> {
  const saved = await saveUsageQuerySettings(profile, settings);
  state.usageSettingsByProfile = {
    ...state.usageSettingsByProfile,
    [profile]: saved,
  };
}

async function refreshUsageStats(showError = false): Promise<void> {
  try {
    const bounds = usageRangeBounds(state.usageStatsRange);
    state.usageStats = await getUsageStats({
      profile: state.usageStatsProfile,
      start_at: bounds.start_at,
      end_at: bounds.end_at,
    });
  } catch (error) {
    if (showError) {
      showToast(error instanceof Error ? error.message : t(state.locale, "usageEmpty"), true);
    }
  } finally {
    rerenderDashboard();
  }
}

async function refreshHistoryStats(showError = false): Promise<void> {
  try {
    const bounds = usageRangeBounds(state.historyStatsRange);
    state.historyStats = await getUsageStats({
      profile: state.historyStatsProfile,
      start_at: bounds.start_at,
      end_at: bounds.end_at,
    });
  } catch (error) {
    if (showError) {
      showToast(error instanceof Error ? error.message : t(state.locale, "usageEmpty"), true);
    }
  } finally {
    rerenderDashboard();
  }
}

function selectedSettingsUsageProfile(): string | null {
  const selected = elements.settingsUsageProfileSelect?.value || state.settingsUsageProfile;
  return selected || state.currentProfile || state.snapshot?.profiles[0]?.folder_name || null;
}

async function saveSettingsUsageProfile(): Promise<void> {
  const profile = selectedSettingsUsageProfile();
  if (!profile) {
    showToast(t(state.locale, "settingsUsageEmpty"), true);
    return;
  }
  try {
    await saveUsageSettingsForProfile(profile, usageSettingsFromInputs("settings"));
    showToast(t(state.locale, "usageQuerySaved"));
    rerenderDashboard();
  } catch (error) {
    showToast(error instanceof Error ? error.message : t(state.locale, "failedToSaveBaseUrl"), true);
  }
}

function openUsageConfigDialog(profile: string): void {
  if (
    !elements.usageConfigDialog ||
    !elements.usageConfigForm ||
    !elements.usageConfigDialogError ||
    !elements.usageConfigEnabledToggle ||
    !elements.usageConfigTimeoutInput ||
    !elements.usageConfigIntervalInput
  ) {
    return;
  }
  usageConfigSourceProfile = profile;
  const settings = state.usageSettingsByProfile[profile] ?? defaultUsageSettings();
  clearDialogError(elements.usageConfigDialogError);
  elements.usageConfigForm.reset();
  elements.usageConfigEnabledToggle.checked = settings.enabled;
  elements.usageConfigTimeoutInput.value = String(settings.timeout_seconds);
  elements.usageConfigIntervalInput.value = String(settings.auto_query_interval_minutes);
  if (elements.usageConfigDialogTitle) {
    elements.usageConfigDialogTitle.textContent = t(state.locale, "usageQuerySetupTitle");
  }
  if (elements.usageConfigDialogCopy) {
    elements.usageConfigDialogCopy.textContent = t(state.locale, "usageQuerySetupCopy");
  }
  elements.usageConfigDialog.showModal();
}

function closeUsageConfigDialog(): void {
  usageConfigSourceProfile = null;
  elements.usageConfigDialog?.close();
}

function scheduleUsageStatsRefresh(): void {
  window.setInterval(() => {
    const intervals = Object.values(state.usageSettingsByProfile)
      .filter((settings) => settings.enabled && settings.auto_query_interval_minutes > 0)
      .map((settings) => settings.auto_query_interval_minutes);
    if (intervals.length === 0) {
      return;
    }

    const shortestMs = Math.max(60_000, Math.min(...intervals) * 60_000);
    const now = Date.now();
    const lastRun = Number(document.body.dataset.usageStatsLastRun ?? "0");
    if (now - lastRun < shortestMs) {
      return;
    }
    document.body.dataset.usageStatsLastRun = String(now);
    void refreshUsageStats(false);
    void refreshHistoryStats(false);
  }, 60_000);
}

async function handleSubmitUsageConfig(event: SubmitEvent): Promise<void> {
  event.preventDefault();
  const profile = usageConfigSourceProfile;
  const error = elements.usageConfigDialogError;
  if (!profile || !error) {
    return;
  }
  clearDialogError(error);
  try {
    await saveUsageSettingsForProfile(profile, usageSettingsFromInputs("dialog"));
    closeUsageConfigDialog();
    if (!state.expandedQuotaProfiles.includes(profile)) {
      state.expandedQuotaProfiles = [...state.expandedQuotaProfiles, profile];
    }
    showToast(t(state.locale, "usageQuerySaved"));
    rerenderDashboard();
  } catch (submitError) {
    showDialogError(
      error,
      submitError instanceof Error ? submitError.message : t(state.locale, "failedToSaveBaseUrl"),
    );
  }
}

function clearDialogError(element: HTMLParagraphElement): void {
  element.hidden = true;
  element.textContent = "";
}

function showDialogError(element: HTMLParagraphElement, message: string): void {
  element.hidden = false;
  element.textContent = message;
}

function openTextDialog(options: {
  dialog: HTMLDialogElement;
  form: HTMLFormElement;
  error: HTMLParagraphElement;
  input: HTMLInputElement;
  value?: string;
}): void {
  clearDialogError(options.error);
  options.form.reset();
  options.input.value = options.value ?? "";
  options.dialog.showModal();
  options.input.focus();
  options.input.select();
}

async function runBlockingAction<T>(run: () => Promise<T>): Promise<T> {
  state.loading = true;
  rerenderDashboard();
  try {
    return await run();
  } finally {
    state.loading = false;
    rerenderDashboard();
  }
}

function setLocale(locale: Locale): void {
  if (state.locale === locale) {
    return;
  }

  state.locale = locale;
  persistLocale(locale);
  rerenderDashboard();
}

function setLocaleFromValue(value: string | undefined): void {
  if (value === "en" || value === "zh-CN") {
    setLocale(value);
  }
}

function setTheme(theme: ThemeId): void {
  if (state.theme === theme) {
    applyTheme(theme);
    document.documentElement.dataset.effectiveTheme = resolveEffectiveTheme(theme);
    renderThemeOptions();
    return;
  }

  state.theme = theme;
  applyTheme(theme);
  document.documentElement.dataset.effectiveTheme = resolveEffectiveTheme(theme);
  persistTheme(theme);
  renderThemeOptions();
  showToast(t(state.locale, "themeChanged", { theme: t(state.locale, getThemeOption(theme).nameKey) }));
}

function setThemeFromValue(value: string | undefined): void {
  if (isThemeId(value)) {
    setTheme(value);
  }
}

function setShowAccountDetail(showDetail: boolean): void {
  state.showAccountDetail = showDetail;
  persistShowAccountDetail(showDetail);
  rerenderDashboard();
}

function handleSystemThemeChange(): void {
  if (state.theme === "system") {
    applyTheme(state.theme);
    document.documentElement.dataset.effectiveTheme = resolveEffectiveTheme(state.theme);
  }
}

async function handleToggleQuotaProfile(profile: string): Promise<void> {
  let settings = state.usageSettingsByProfile[profile];
  try {
    settings = await ensureUsageSettings(profile);
  } catch {
    settings = defaultUsageSettings();
  }

  if (!settings.enabled && !state.expandedQuotaProfiles.includes(profile)) {
    openUsageConfigDialog(profile);
    return;
  }

  if (state.expandedQuotaProfiles.includes(profile)) {
    state.expandedQuotaProfiles = state.expandedQuotaProfiles.filter((value) => value !== profile);
  } else {
    state.expandedQuotaProfiles = [...state.expandedQuotaProfiles, profile];
  }
  rerenderDashboard();
}

function activateSettingsTab(tabName: string): void {
  const buttons = document.querySelectorAll<HTMLButtonElement>("[data-settings-tab]");
  const panels = document.querySelectorAll<HTMLElement>("[data-settings-panel]");

  for (const button of buttons) {
    const isActive = button.dataset.settingsTab === tabName;
    button.classList.toggle("is-active", isActive);
    button.setAttribute("aria-pressed", isActive ? "true" : "false");
  }

  for (const panel of panels) {
    const isActive = panel.dataset.settingsPanel === tabName;
    panel.classList.toggle("is-active", isActive);
    panel.hidden = !isActive;
  }
}

async function refreshCurrentQuota(showError = false): Promise<void> {
  if (state.loading || !state.snapshot) {
    return;
  }

  try {
    applyCurrentQuota(await getCurrentLiveQuota());
    rerenderDashboard();
  } catch (error) {
    if (showError) {
      showToast(error instanceof Error ? error.message : "Failed to refresh quota.", true);
    }
  }
}

// Silent companion to refreshCurrentQuota. Backend gates on >5min staleness
// and silently swallows non-OAuth / HTTP / parse failures, so any error here
// just means "skip this tick".
async function refreshActiveQuotaSilently(): Promise<void> {
  if (state.loading || !state.snapshot) {
    return;
  }

  try {
    applyCurrentQuota(await refreshActiveProfileQuotaSilent());
    rerenderDashboard();
  } catch {
    // Intentional: silent ticker, never surface errors to the user.
  }
}

// Tracks the last unmanaged account we prompted about so a single drift event
// shows the toast once, not on every dashboard refresh. Resets to null when the
// live account is managed again, so a later drift re-prompts.
let lastUnmanagedAccountPrompt: string | null = null;

function maybePromptUnmanagedAccount(account: string | null): void {
  if (!account) {
    lastUnmanagedAccountPrompt = null;
    return;
  }
  if (account === lastUnmanagedAccountPrompt) {
    return;
  }
  lastUnmanagedAccountPrompt = account;
  showToast(t(state.locale, "unmanagedAccountToast", { account }), true);
}

async function refreshAllData(showError = true): Promise<void> {
  try {
    const [snapshot, currentQuota] = await Promise.all([
      getProfilesSnapshot(),
      getCurrentLiveQuota(),
    ]);

    applySnapshot(snapshot);
    applyCurrentQuota(currentQuota);
    await loadUsageSettingsForProfiles(snapshot.profiles);
    await refreshUsageStats(false);
    await refreshHistoryStats(false);
    rerenderDashboard();
    maybePromptUnmanagedAccount(snapshot.unmanaged_live_account);
  } catch (error) {
    if (showError) {
      showToast(error instanceof Error ? error.message : "Failed to load dashboard.", true);
    }
  }
}

function isExpiredProfileAuthError(error: unknown): boolean {
  if (!(error instanceof Error)) {
    return false;
  }

  const code = (error as ErrorWithCode).code;
  if (code === "AUTH_REFRESH_RELOGIN_REQUIRED") {
    return true;
  }

  return /token_invalidated|refresh_token_reused|sign(?:ing)? in again|log out and sign in again/i.test(
    error.message,
  );
}

function refreshProfileErrorMessage(error: unknown): string {
  if (isExpiredProfileAuthError(error)) {
    return t(state.locale, "profileRefreshRequiresLogin");
  }

  return error instanceof Error ? error.message : t(state.locale, "failedToRefreshProfile");
}

async function handleSwitchProfile(profile: string): Promise<void> {
  // Guard against double-click / racing rerender: the disabled
  // attribute on the button doesn't take effect until the next
  // browser paint, so a fast second click can fire before the first
  // switch's `runBlockingAction` flips `state.loading`. Without this
  // guard the second IPC hits the backend lock and the user sees the
  // SWITCH_IN_PROGRESS toast instead of a no-op. The other card
  // actions (Refresh, Login, Delete) already had this check; Switch
  // missed it.
  if (state.loading || state.loginActiveProfile !== null || isRefreshPending(profile)) {
    return;
  }
  try {
    await runBlockingAction(async () => {
      await switchProfile(profile);
      showToast(t(state.locale, "switchedTo", { profile }));
      await refreshAllData();
    });
  } catch (error) {
    showToast(error instanceof Error ? error.message : t(state.locale, "failedToSwitchProfile"), true);
  }
}

async function performProfileRefresh(profile: string): Promise<void> {
  state.refreshActiveProfiles.push(profile);
  rerenderDashboard();
  try {
    await refreshProfile(profile);
    state.reloginProfiles = state.reloginProfiles.filter((value) => value !== profile);
    showToast(t(state.locale, "refreshedProfile", { profile }));
    try {
      const snapshot = await getProfilesSnapshot();
      applySnapshot(snapshot);
      if (snapshot.current_card?.folder_name === profile) {
        applyCurrentQuota(await getCurrentLiveQuota());
      }
    } catch (error) {
      console.warn("Snapshot refresh after profile refresh failed:", error);
    }
  } catch (error) {
    if (isExpiredProfileAuthError(error)) {
      state.reloginProfiles = Array.from(new Set([...state.reloginProfiles, profile]));
    }
    showToast(refreshProfileErrorMessage(error), true);
  } finally {
    state.refreshActiveProfiles = state.refreshActiveProfiles.filter(p => p !== profile);
    rerenderDashboard();
  }
}

function handleRefreshProfile(profile: string): void {
  if (
    state.loading
    || state.loginActiveProfile === profile
    || isRefreshPending(profile)
  ) {
    return;
  }

  void performProfileRefresh(profile);
}

function loginErrorCode(error: unknown): string | undefined {
  return error instanceof Error ? (error as ErrorWithCode).code : undefined;
}

function loginErrorMessage(profile: string, error: unknown): string {
  const code = loginErrorCode(error);
  if (code === "LOGIN_BUSY") {
    return t(state.locale, "loginBusy");
  }
  if (code === "REAL_CODEX_NOT_FOUND") {
    return t(state.locale, "codexCliNotFoundToast");
  }
  if (code === "LOGIN_CANCELLED") {
    return t(state.locale, "loginCancelled");
  }
  if (error instanceof Error && error.message) {
    return error.message;
  }
  return t(state.locale, "failedToLoginProfile", { profile });
}

async function handleLoginProfile(profile: string): Promise<void> {
  // Reuse this entry point as the "cancel" channel: when the same profile
  // already owns the in-flight login, the click means "kill the codex
  // process I just spawned" rather than "start another login". Any other
  // profile holding the lock falls through to the early-return.
  if (state.loginActiveProfile === profile) {
    void handleCancelLogin(profile);
    return;
  }

  // Login serializes through the same .switch.lock as switch and refresh,
  // so block the click if any of the three is already in flight on this
  // card. Other cards' actions are independent.
  if (
    state.loading ||
    state.loginActiveProfile !== null ||
    isRefreshPending(profile)
  ) {
    return;
  }

  state.loginActiveProfile = profile;
  cancelledLoginProfile = null;
  rerenderDashboard();
  showToast(t(state.locale, "loginStarting", { profile }));
  try {
    await loginProfile(profile);
    state.reloginProfiles = state.reloginProfiles.filter((value) => value !== profile);
    showToast(t(state.locale, "loggedInProfile", { profile }));
    await refreshAllData(false);
  } catch (error) {
    const code = loginErrorCode(error);
    if (cancelledLoginProfile === profile || code === "LOGIN_CANCELLED") {
      showToast(t(state.locale, "loginCancelled"));
    } else {
      showToast(loginErrorMessage(profile, error), true);
      if (code === "REAL_CODEX_NOT_FOUND") {
        void openCodexCliDialog(() => handleLoginProfile(profile));
      }
    }
  } finally {
    state.loginActiveProfile = null;
    cancelledLoginProfile = null;
    rerenderDashboard();
  }
}

function handleReorderProfiles(sourceProfile: string, targetProfile: string): void {
  const snapshot = state.snapshot;
  const profiles = snapshot?.profiles;
  if (!snapshot || !profiles || sourceProfile === targetProfile) {
    return;
  }

  const sourceIndex = profiles.findIndex((profile) => profile.folder_name === sourceProfile);
  const targetIndex = profiles.findIndex((profile) => profile.folder_name === targetProfile);
  if (sourceIndex < 0 || targetIndex < 0) {
    return;
  }

  const reordered = [...profiles];
  const [moved] = reordered.splice(sourceIndex, 1);
  if (!moved) {
    return;
  }
  reordered.splice(targetIndex, 0, moved);
  state.snapshot = {
    ...snapshot,
    profiles: reordered,
  };
  persistProfileOrder(reordered);
  rerenderDashboard();
  showToast(t(state.locale, "profileOrderUpdated"));
}

async function handleCancelLogin(profile: string): Promise<void> {
  // Set the flag eagerly so the in-flight loginProfile rejection — which
  // can settle on the same task tick — sees it and shows "已取消登录"
  // instead of LOGIN_FAILED. If the backend reports nothing was actually
  // cancelled (login already completed/failed), we roll the flag back so
  // the real toast still surfaces.
  cancelledLoginProfile = profile;
  try {
    const cancelled = await cancelCodexLogin();
    if (!cancelled) {
      cancelledLoginProfile = null;
    }
  } catch (error) {
    cancelledLoginProfile = null;
    showToast(
      error instanceof Error ? error.message : t(state.locale, "loginCancelFailed"),
      true,
    );
  }
}

function handleRenameProfileClick(profile: string): void {
  renameSourceProfile = profile;
  openTextDialog({
    dialog: elements.renameDialog,
    form: elements.renameProfileForm,
    error: elements.renameDialogError,
    input: elements.renameFolderNameInput,
    value: profile,
  });
}

function handleBaseUrlProfileClick(profile: string): void {
  const currentBaseUrl =
    state.snapshot?.profiles.find((entry) => entry.folder_name === profile)?.openai_base_url ?? "";
  baseUrlSourceProfile = profile;
  openTextDialog({
    dialog: elements.baseUrlDialog,
    form: elements.baseUrlForm,
    error: elements.baseUrlDialogError,
    input: elements.baseUrlInput,
    value: currentBaseUrl,
  });
}

function handleDeleteProfileClick(profile: string): void {
  if (!elements.deleteProfileDialog || !elements.deleteProfileDialogError) {
    return;
  }

  deleteSourceProfile = profile;
  clearDialogError(elements.deleteProfileDialogError);
  elements.deleteProfileDialog.showModal();
}

async function handleOpenCurrentFolder(): Promise<void> {
  if (!state.currentProfile) {
    return;
  }

  try {
    await openProfileFolder(state.currentProfile);
    showToast(t(state.locale, "openedProfileFolder"));
  } catch (error) {
    showToast(error instanceof Error ? error.message : t(state.locale, "failedToOpenProfileFolder"), true);
  }
}

async function handleOpenCodex(): Promise<void> {
  try {
    await openCodex();
    showToast(t(state.locale, "openedCodex"));
  } catch (error) {
    showToast(error instanceof Error ? error.message : t(state.locale, "failedToOpenCodex"), true);
  }
}

async function handleLoginCurrentProfile(): Promise<void> {
  if (!state.currentProfile) {
    return;
  }

  try {
    await runBlockingAction(async () => {
      await loginCurrentProfile();
      showToast(t(state.locale, "loggedIn", { profile: state.currentProfile as string }));
      await refreshAllData();
    });
  } catch (error) {
    if (loginErrorCode(error) === "REAL_CODEX_NOT_FOUND") {
      showToast(t(state.locale, "codexCliNotFoundToast"), true);
      void openCodexCliDialog(() => handleLoginCurrentProfile());
      return;
    }
    showToast(error instanceof Error ? error.message : t(state.locale, "failedToLogin"), true);
  }
}

async function handleOpenContact(): Promise<void> {
  try {
    await openContact();
    showToast(t(state.locale, "openedRepository"));
  } catch (error) {
    showToast(error instanceof Error ? error.message : t(state.locale, "failedToOpenRepository"), true);
  }
}

async function handleOpenReleases(): Promise<void> {
  try {
    await openReleases();
    showToast(t(state.locale, "openedReleases"));
  } catch (error) {
    showToast(error instanceof Error ? error.message : t(state.locale, "failedToOpenReleases"), true);
  }
}

async function handleOpenUpdateRelease(): Promise<void> {
  const releaseUrl = pendingUpdateReleaseUrl;
  if (!releaseUrl) {
    await handleOpenReleases();
    return;
  }

  try {
    await openUrl(releaseUrl);
    showToast(t(state.locale, "openedReleases"));
  } catch (error) {
    showToast(error instanceof Error ? error.message : t(state.locale, "failedToOpenReleases"), true);
  }
}

async function handleCheckUpdate(silent = false): Promise<void> {
  if (!silent) {
    showToast(t(state.locale, "checkingUpdate"));
  }

  try {
    const update = await checkUpdate(elements.settingsUpdateUrlInput.value);
    if (update.has_update) {
      pendingUpdateReleaseUrl = update.release_url;
      showUpdateDialog(update);
      if (!silent) {
        showToast(t(state.locale, "updateAvailable", {
          current: update.current_version,
          latest: update.latest_version ?? "--",
        }));
      }
      return;
    }

    if (!silent) {
      showToast(t(state.locale, "updateAlreadyLatest", { current: update.current_version }));
    }
  } catch (error) {
    if (!silent) {
      showToast(error instanceof Error ? error.message : t(state.locale, "failedToCheckUpdate"), true);
    }
  }
}

async function handleOpenXiaohongshu(): Promise<void> {
  try {
    await openXiaohongshu();
    showToast(t(state.locale, "openedXiaohongshu"));
  } catch (error) {
    showToast(error instanceof Error ? error.message : t(state.locale, "failedToOpenXiaohongshu"), true);
  }
}

function openAddProfileDialog(): void {
  openTextDialog({
    dialog: elements.dialog,
    form: elements.addProfileForm,
    error: elements.dialogError,
    input: elements.folderNameInput,
  });
}

function closeRenameProfileDialog(): void {
  renameSourceProfile = null;
  elements.renameDialog.close();
}

function closeBaseUrlDialog(): void {
  baseUrlSourceProfile = null;
  elements.baseUrlDialog.close();
}

function closeDeleteProfileDialog(): void {
  deleteSourceProfile = null;
  elements.deleteProfileDialog?.close();
}

async function handleSubmitAddProfile(event: SubmitEvent): Promise<void> {
  event.preventDefault();
  clearDialogError(elements.dialogError);

  const folderName = elements.folderNameInput.value.trim();
  const openaiBaseUrl = elements.addBaseUrlInput.value.trim();
  if (!folderName) {
    showDialogError(elements.dialogError, t(state.locale, "folderNameRequired"));
    return;
  }

  try {
    await runBlockingAction(async () => {
      await addProfile(folderName, openaiBaseUrl || null);
      elements.dialog.close();
      showToast(t(state.locale, "createdProfile", { profile: folderName }));
      await refreshAllData();
    });
  } catch (error) {
    showDialogError(
      elements.dialogError,
      error instanceof Error ? error.message : t(state.locale, "failedToCreateProfile"),
    );
  }
}

async function handleSubmitRenameProfile(event: SubmitEvent): Promise<void> {
  event.preventDefault();
  clearDialogError(elements.renameDialogError);

  const sourceProfile = renameSourceProfile;
  const nextFolderName = elements.renameFolderNameInput.value.trim();
  if (!nextFolderName) {
    showDialogError(elements.renameDialogError, t(state.locale, "folderNameRequired"));
    return;
  }
  if (!sourceProfile) {
    showDialogError(elements.renameDialogError, t(state.locale, "failedToRenameProfile"));
    return;
  }
  if (nextFolderName === sourceProfile) {
    closeRenameProfileDialog();
    return;
  }

  try {
    await runBlockingAction(async () => {
      await renameProfile(sourceProfile, nextFolderName);
      closeRenameProfileDialog();
      showToast(t(state.locale, "renamedProfile", { from: sourceProfile, to: nextFolderName }));
      await refreshAllData();
    });
  } catch (error) {
    showDialogError(
      elements.renameDialogError,
      error instanceof Error ? error.message : t(state.locale, "failedToRenameProfile"),
    );
  }
}

async function handleSubmitBaseUrl(event: SubmitEvent): Promise<void> {
  event.preventDefault();
  clearDialogError(elements.baseUrlDialogError);

  const sourceProfile = baseUrlSourceProfile;
  const nextBaseUrl = elements.baseUrlInput.value.trim();
  if (!sourceProfile) {
    showDialogError(elements.baseUrlDialogError, t(state.locale, "failedToSaveBaseUrl"));
    return;
  }

  try {
    await runBlockingAction(async () => {
      await updateProfileBaseUrl(sourceProfile, nextBaseUrl);
      closeBaseUrlDialog();
      showToast(
        nextBaseUrl
          ? t(state.locale, "savedBaseUrl", { profile: sourceProfile })
          : t(state.locale, "clearedBaseUrl", { profile: sourceProfile }),
      );
      await refreshAllData();
    });
  } catch (error) {
    showDialogError(
      elements.baseUrlDialogError,
      error instanceof Error ? error.message : t(state.locale, "failedToSaveBaseUrl"),
    );
  }
}

function applyCodexCliSettingsDisplay(status: CodexCliStatus): void {
  const path = status.resolved_path?.trim();
  if (path) {
    elements.settingsCodexCliValue.textContent = `${path} (${codexCliSourceLabel(status.source)})`;
  } else {
    elements.settingsCodexCliValue.textContent = t(state.locale, "settingsCodexCliEmpty");
  }
}

async function refreshCodexCliSettingsDisplay(): Promise<void> {
  try {
    const status = await getCodexCliStatus();
    applyCodexCliSettingsDisplay(status);
  } catch {
    // Best-effort: leave the previous label up. The dialog itself
    // surfaces the actual error when the user opens it.
  }
}

function codexCliSourceLabel(source: CodexCliStatus["source"]): string {
  switch (source) {
    case "user_override":
      return t(state.locale, "codexCliSourceUserOverride");
    case "install_state":
      return t(state.locale, "codexCliSourceInstallState");
    case "discovery":
      return t(state.locale, "codexCliSourceDiscovery");
    default:
      return t(state.locale, "codexCliSourceNone");
  }
}

function renderCodexCliStatus(status: CodexCliStatus, detected?: CodexCliCandidate[]): void {
  if (status.resolved_path) {
    elements.codexCliCurrentValue.textContent = status.resolved_path;
    elements.codexCliCurrentSource.textContent = ` (${codexCliSourceLabel(status.source)})`;
    elements.codexCliCurrentSource.hidden = false;
    elements.clearCodexCliButton.hidden = status.source !== "user_override";
  } else {
    elements.codexCliCurrentValue.textContent = t(state.locale, "codexCliCurrentNone");
    elements.codexCliCurrentSource.textContent = "";
    elements.codexCliCurrentSource.hidden = true;
    elements.clearCodexCliButton.hidden = true;
  }

  // When auto-detect routes here with verified-runnable candidates, show
  // those (with versions) instead of the raw common-location hints, so
  // the user only picks from installs that actually ran.
  const showingDetected = detected !== undefined && detected.length > 0;
  elements.codexCliSuggestionsHeading.textContent = showingDetected
    ? t(state.locale, "codexCliDetectedHeading")
    : t(state.locale, "codexCliSuggestionsHeading");

  // Normalise both sources to { path, version } so one render loop serves
  // detected candidates (with versions) and plain suggestion hints.
  const chips: CodexCliCandidate[] = showingDetected
    ? detected
    : status.suggested_paths.map((path) => ({ path, version: null }));

  elements.codexCliSuggestions.replaceChildren();
  if (chips.length === 0) {
    const empty = document.createElement("p");
    empty.className = "codex-cli-suggestions-empty";
    empty.textContent = t(state.locale, "codexCliSuggestionsEmpty");
    elements.codexCliSuggestions.append(empty);
    return;
  }

  for (const candidate of chips) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "codex-cli-suggestion";
    button.textContent = candidate.version
      ? `${candidate.path}  ·  ${candidate.version}`
      : candidate.path;
    button.addEventListener("click", () => {
      elements.codexCliInput.value = candidate.path;
      elements.codexCliInput.focus();
      elements.codexCliInput.select();
      clearDialogError(elements.codexCliDialogError);
    });
    elements.codexCliSuggestions.append(button);
  }
}

async function openCodexCliDialog(
  onSavedRetry?: () => Promise<void>,
  detectedCandidates?: CodexCliCandidate[],
): Promise<void> {
  pendingLoginRetry = onSavedRetry ?? null;
  clearDialogError(elements.codexCliDialogError);
  elements.codexCliInput.value = "";

  let status: CodexCliStatus = {
    resolved_path: null,
    source: "none",
    suggested_paths: [],
  };
  try {
    status = await getCodexCliStatus();
  } catch (error) {
    showDialogError(
      elements.codexCliDialogError,
      error instanceof Error ? error.message : t(state.locale, "codexCliPathSaveFailed"),
    );
  }

  // Detected-mode opens after auto-detect found several runnable codex,
  // so use a "pick one" copy — NOT the default "couldn't find it" copy
  // that misled users into thinking detection had failed.
  const hasDetected = detectedCandidates !== undefined && detectedCandidates.length > 0;
  elements.codexCliDialogCopy.textContent = hasDetected
    ? t(state.locale, "codexCliDetectPickCopy")
    : t(state.locale, "codexCliDialogCopy");

  // Prefill the first detected candidate; otherwise the resolved path.
  if (detectedCandidates !== undefined && detectedCandidates.length > 0) {
    elements.codexCliInput.value = detectedCandidates[0].path;
  } else if (status.resolved_path) {
    elements.codexCliInput.value = status.resolved_path;
  }
  elements.submitCodexCliButton.textContent = onSavedRetry
    ? t(state.locale, "codexCliRetryLogin")
    : t(state.locale, "save");

  renderCodexCliStatus(status, detectedCandidates);
  elements.codexCliDialog.showModal();
  elements.codexCliInput.focus();
  elements.codexCliInput.select();
}

/// Settings "auto-detect" button: force a fresh runnable scan and act on
/// the result — apply a lone hit, let the user pick when several survive,
/// or fall back to the manual dialog when none do.
async function handleDetectCodexCli(): Promise<void> {
  const button = elements.settingsCodexCliDetectButton;
  if (button.disabled) {
    return;
  }
  button.disabled = true;
  button.textContent = t(state.locale, "settingsCodexCliDetecting");
  try {
    const result: CodexCliRedetectResult = await redetectCodexCliPath();
    if (result.candidates.length === 1) {
      // Lone runnable hit → apply it straight away (the small-user
      // fallback: one click, done). If the backend's set/validate then
      // rejects it (managed shim, or the file vanished between probe and
      // set), don't dump the raw error — fall back to the dialog with the
      // candidate so the user can adjust.
      const only = result.candidates[0];
      try {
        const status = await setCodexCliPath(only.path);
        applyCodexCliSettingsDisplay(status);
        showToast(t(state.locale, "codexCliDetectApplied", { path: only.path }));
      } catch {
        applyCodexCliSettingsDisplay(result.status);
        void openCodexCliDialog(undefined, result.candidates);
      }
    } else if (result.candidates.length === 0) {
      // Nothing runnable. Distinguish "no codex anywhere" from "codex
      // exists on disk but none would run" (a broken install, not a
      // missing one) via the on-disk suggestions in the refreshed status.
      applyCodexCliSettingsDisplay(result.status);
      const foundButBroken = result.status.suggested_paths.length > 0;
      showToast(
        t(state.locale, foundButBroken ? "codexCliDetectFoundButBroken" : "codexCliDetectNone"),
        true,
      );
      void openCodexCliDialog();
    } else {
      // Several runnable hits → let the user choose in the dialog.
      applyCodexCliSettingsDisplay(result.status);
      showToast(
        t(state.locale, "codexCliDetectMultiple", { count: String(result.candidates.length) }),
      );
      void openCodexCliDialog(undefined, result.candidates);
    }
  } catch (error) {
    showToast(
      error instanceof Error ? error.message : t(state.locale, "codexCliDetectFailed"),
      true,
    );
  } finally {
    button.disabled = false;
    button.textContent = t(state.locale, "settingsCodexCliDetect");
  }
}

function closeCodexCliDialog(): void {
  pendingLoginRetry = null;
  elements.codexCliDialog.close();
}

function codexCliErrorMessage(error: unknown): string {
  const code = error instanceof Error ? (error as ErrorWithCode).code : undefined;
  switch (code) {
    case "CODEX_CLI_PATH_EMPTY":
      return t(state.locale, "codexCliPathEmpty");
    case "CODEX_CLI_PATH_INVALID":
      return t(state.locale, "codexCliPathInvalid");
    case "CODEX_CLI_PATH_REJECTED":
      return t(state.locale, "codexCliPathRejected");
    default:
      return error instanceof Error
        ? error.message
        : t(state.locale, "codexCliPathSaveFailed");
  }
}

async function handleSubmitCodexCliPath(event: SubmitEvent): Promise<void> {
  event.preventDefault();
  clearDialogError(elements.codexCliDialogError);

  const rawInput = elements.codexCliInput.value;
  if (!rawInput.trim()) {
    showDialogError(elements.codexCliDialogError, t(state.locale, "codexCliPathEmpty"));
    return;
  }

  let status: CodexCliStatus;
  try {
    status = await setCodexCliPath(rawInput);
  } catch (error) {
    showDialogError(elements.codexCliDialogError, codexCliErrorMessage(error));
    return;
  }

  const retry = pendingLoginRetry;
  closeCodexCliDialog();
  showToast(t(state.locale, "codexCliPathSaved"));
  renderCodexCliStatus(status);
  applyCodexCliSettingsDisplay(status);

  if (retry) {
    await retry();
  }
}

async function handleClearCodexCliPath(): Promise<void> {
  clearDialogError(elements.codexCliDialogError);
  try {
    const status = await clearCodexCliPath();
    renderCodexCliStatus(status);
    applyCodexCliSettingsDisplay(status);
    elements.codexCliInput.value = status.resolved_path ?? "";
    showToast(t(state.locale, "codexCliPathCleared"));
  } catch (error) {
    showDialogError(elements.codexCliDialogError, codexCliErrorMessage(error));
  }
}

async function handleDeleteProfileAction(action: "delete" | "clear"): Promise<void> {
  const sourceProfile = deleteSourceProfile;
  const errorElement = elements.deleteProfileDialogError;
  if (!errorElement) {
    return;
  }

  clearDialogError(errorElement);
  if (!sourceProfile) {
    showDialogError(errorElement, t(state.locale, "failedToDeleteProfile"));
    return;
  }

  try {
    await runBlockingAction(async () => {
      if (action === "delete") {
        await deleteProfile(sourceProfile);
        closeDeleteProfileDialog();
        showToast(t(state.locale, "deletedProfile", { profile: sourceProfile }));
      } else {
        await clearProfileAccount(sourceProfile);
        closeDeleteProfileDialog();
        showToast(t(state.locale, "clearedProfileAccount", { profile: sourceProfile }));
      }
      await refreshAllData();
    });
  } catch (error) {
    showDialogError(
      errorElement,
      error instanceof Error ? error.message : t(state.locale, "failedToDeleteProfile"),
    );
  }
}

export function bootstrap(): void {
  state.locale = resolveInitialLocale();
  state.theme = resolveInitialTheme();
  state.route = routeFromLocation();
  applyTheme(state.theme);
  document.documentElement.dataset.effectiveTheme = resolveEffectiveTheme(state.theme);
  applyLocale();
  renderShellRoute();

  const systemThemeMedia = globalThis.matchMedia?.("(prefers-color-scheme: dark)");
  systemThemeMedia?.addEventListener("change", handleSystemThemeChange);
  activateSettingsTab("general");

  window.addEventListener("hashchange", () => {
    state.route = routeFromLocation();
    renderShellRoute();
  });

  elements.previousPageButton.addEventListener("click", () => {
    state.page -= 1;
    rerenderDashboard();
  });
  elements.nextPageButton.addEventListener("click", () => {
    state.page += 1;
    rerenderDashboard();
  });
  elements.openCurrentFolderButton.addEventListener("click", () => {
    void handleOpenCurrentFolder();
  });
  elements.currentLoginButton.addEventListener("click", () => {
    void handleLoginCurrentProfile();
  });
  elements.openCodexButton.addEventListener("click", () => {
    void handleOpenCodex();
  });
  elements.settingsGithubButton.addEventListener("click", () => {
    void handleOpenContact();
  });
  elements.settingsCheckUpdateButton.addEventListener("click", () => {
    void handleCheckUpdate();
  });
  elements.updateDialogLaterButton.addEventListener("click", () => {
    elements.updateDialog.close();
  });
  elements.updateDialogOpenButton.addEventListener("click", () => {
    elements.updateDialog.close();
    void handleOpenUpdateRelease();
  });
  elements.starButton.addEventListener("click", () => {
    window.location.hash = "guide";
  });
  elements.xiaohongshuButton.addEventListener("click", () => {
    void handleOpenXiaohongshu();
  });
  elements.addProfilesButton.addEventListener("click", openAddProfileDialog);
  for (const button of elements.addProfileButtons) {
    button.addEventListener("click", openAddProfileDialog);
  }
  elements.cancelAddProfileButton.addEventListener("click", () => {
    elements.dialog.close();
  });
  elements.cancelRenameProfileButton.addEventListener("click", () => {
    closeRenameProfileDialog();
  });
  elements.cancelBaseUrlButton.addEventListener("click", () => {
    closeBaseUrlDialog();
  });
  elements.cancelDeleteProfileButton?.addEventListener("click", () => {
    closeDeleteProfileDialog();
  });
  elements.deleteProfileButton?.addEventListener("click", () => {
    void handleDeleteProfileAction("delete");
  });
  elements.clearProfileAccountButton?.addEventListener("click", () => {
    void handleDeleteProfileAction("clear");
  });
  elements.addProfileForm.addEventListener("submit", (event) => {
    void handleSubmitAddProfile(event as SubmitEvent);
  });
  elements.renameProfileForm.addEventListener("submit", (event) => {
    void handleSubmitRenameProfile(event as SubmitEvent);
  });
  elements.baseUrlForm.addEventListener("submit", (event) => {
    void handleSubmitBaseUrl(event as SubmitEvent);
  });
  elements.usageConfigForm?.addEventListener("submit", (event) => {
    void handleSubmitUsageConfig(event as SubmitEvent);
  });
  elements.cancelUsageConfigButton?.addEventListener("click", () => {
    closeUsageConfigDialog();
  });
  elements.testUsageConfigButton?.addEventListener("click", () => {
    showToast(t(state.locale, "usageQuerySaved"));
  });
  elements.cancelCodexCliButton.addEventListener("click", () => {
    closeCodexCliDialog();
  });
  elements.clearCodexCliButton.addEventListener("click", () => {
    void handleClearCodexCliPath();
  });
  elements.codexCliForm.addEventListener("submit", (event) => {
    void handleSubmitCodexCliPath(event as SubmitEvent);
  });
  elements.settingsCodexCliDetectButton.addEventListener("click", () => {
    void handleDetectCodexCli();
  });
  elements.settingsCodexCliButton.addEventListener("click", () => {
    void openCodexCliDialog();
  });
  if (elements.settingsShowAccountDetailToggle instanceof HTMLInputElement) {
    elements.settingsShowAccountDetailToggle.addEventListener("change", () => {
      const toggle = elements.settingsShowAccountDetailToggle;
      if (toggle instanceof HTMLInputElement) {
        setShowAccountDetail(toggle.checked);
      }
    });
  }
  elements.settingsUsageProfileSelect?.addEventListener("change", () => {
    const profile = elements.settingsUsageProfileSelect?.value || null;
    state.settingsUsageProfile = profile;
    if (profile) {
      void ensureUsageSettings(profile).finally(rerenderDashboard);
    } else {
      rerenderDashboard();
    }
  });
  elements.settingsUsageSaveButton?.addEventListener("click", () => {
    void saveSettingsUsageProfile();
  });
  elements.usageProfileFilter?.addEventListener("change", () => {
    state.usageStatsProfile = elements.usageProfileFilter?.value || null;
    void refreshUsageStats(true);
  });
  elements.usageRangeFilter?.addEventListener("change", () => {
    const value = elements.usageRangeFilter?.value;
    if (value === "today" || value === "7d" || value === "30d") {
      state.usageStatsRange = value;
      void refreshUsageStats(true);
    }
  });
  elements.usageRefreshButton?.addEventListener("click", () => {
    void refreshUsageStats(true);
  });
  elements.historyProfileFilter?.addEventListener("change", () => {
    state.historyStatsProfile = elements.historyProfileFilter?.value || null;
    void refreshHistoryStats(true);
  });
  elements.historyRangeFilter?.addEventListener("change", () => {
    const value = elements.historyRangeFilter?.value;
    if (value === "today" || value === "7d" || value === "30d") {
      state.historyStatsRange = value;
      void refreshHistoryStats(true);
    }
  });
  elements.historyRefreshButton?.addEventListener("click", () => {
    void refreshHistoryStats(true);
  });
  elements.localeEnButton.addEventListener("click", () => {
    setLocale("en");
  });
  elements.localeZhButton.addEventListener("click", () => {
    setLocale("zh-CN");
  });
  for (const button of elements.localeButtons) {
    button.addEventListener("click", () => {
      setLocaleFromValue(button.dataset.setLocale);
    });
  }
  for (const button of elements.themeButtons) {
    button.addEventListener("click", () => {
      setThemeFromValue(button.dataset.themeOption);
    });
  }
  for (const button of document.querySelectorAll<HTMLButtonElement>("[data-settings-tab]")) {
    button.addEventListener("click", () => {
      activateSettingsTab(button.dataset.settingsTab ?? "general");
    });
  }
  window.setInterval(() => {
    void refreshCurrentQuota();
  }, 15_000);

  // Slower silent ticker (5 min) — backend hits the ChatGPT API directly so
  // the 5h-window remaining percent stays accurate even when no Codex
  // session has run recently. The 15s JSONL poll above remains the visible
  // source of truth.
  window.setInterval(() => {
    void refreshActiveQuotaSilently();
  }, 5 * 60_000);

  // Relative countdown timer tick: rerender the dashboard every 15 seconds
  // to update the remaining relative countdown times.
  window.setInterval(() => {
    rerenderDashboard();
  }, 15_000);

  // Bulk plan refresh: forces an OAuth refresh on every OAuth profile so
  // the cached id_token claims (plan tier, subscription expiry) move
  // forward even for inactive profiles that the 5-min ticker never
  // visits. Run once at startup (after the initial dashboard load) and
  // then once per local-day rollover. Failures inside the backend are
  // swallowed per-profile, so this never surfaces a toast.
  scheduleDailyPlanRefresh();
  scheduleUsageStatsRefresh();

  void refreshCodexCliSettingsDisplay();

  state.loading = true;
  rerenderDashboard();
  void refreshAllData().finally(() => {
    state.loading = false;
    rerenderDashboard();
    void handleCheckUpdate(true);
    // Kick the bulk plan refresh after the dashboard's first render so
    // the user sees their cards immediately without waiting on N
    // serial OAuth refreshes.
    void refreshAllOauthProfilePlansSilent().catch(() => {
      // Best-effort; backend already swallows per-profile errors.
    });
  });
}

/// Detect local-day rollovers (midnight in the user's timezone) by
/// comparing the cached date string against `new Date().toDateString()`
/// every 10 minutes. When the date changes, kick a bulk plan refresh so
/// the dashboard reflects subscription renewals / plan switches that
/// happened overnight. The 10-minute polling cadence is short enough
/// that the user never sees stale data more than ~10 min into a new
/// day, but long enough that we don't spin the event loop.
function scheduleDailyPlanRefresh(): void {
  let lastBulkDateKey = new Date().toDateString();
  window.setInterval(
    () => {
      const today = new Date().toDateString();
      if (today === lastBulkDateKey) {
        return;
      }
      lastBulkDateKey = today;
      void refreshAllOauthProfilePlansSilent().catch(() => {
        // Best-effort; backend already swallows per-profile errors.
      });
    },
    10 * 60_000,
  );
}
