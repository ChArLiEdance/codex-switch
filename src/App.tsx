import {
  AlertTriangle,
  BarChart3,
  CheckCircle2,
  Clock3,
  ClipboardList,
  Copy,
  Database,
  FolderSearch,
  History,
  KeyRound,
  Laptop,
  ListChecks,
  Pencil,
  Plus,
  RefreshCw,
  RotateCcw,
  Save,
  Settings,
  ShieldCheck,
  SquareTerminal,
  Star,
  Trash2,
  X,
  UserPlus
} from "lucide-react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useMemo, useRef, useState } from "react";
import {
  AppSettings,
  checkRecoveryStatus,
  clearSwitchHistory,
  deleteProfile,
  detectEnvironments,
  emptyEnvironmentScan,
  environmentDiagnosticsReport,
  EnvironmentId,
  EnvironmentPathKind,
  EnvironmentScan,
  EnvironmentState,
  getUsageHistory,
  getSettings,
  importCurrentProfile,
  listProfiles,
  listSwitchHistory,
  previewCurrentImport,
  ProfileMetadata,
  ProfileImportPreflightResult,
  ProfileRecoverySwitchRequest,
  ProfileRecoverySwitchResult,
  ProfileSwitchResult,
  RecoveryStatus,
  resolveRecoveryStatus,
  rollbackUnfinishedTransaction,
  restoreDefaultProfile,
  restoreDefaultOnExit,
  restartDesktopApp,
  restartVscodeApp,
  saveSettings,
  SwitchHistoryEntry,
  switchPreviousProfile,
  switchToProfile,
  TargetEnvironment,
  UsageHistoryReport,
  UsageQuotaSummary,
  UsageSessionSummary,
  UsageTokenTotals,
  updateProfile
} from "./backend";
import { createTranslator, type Translate } from "./i18n";

type Tab = "home" | "profiles" | "environment" | "usage" | "settings";

const defaultSettings: AppSettings = {
  defaultScope: ["cli", "vscode", "desktop"],
  confirmBeforeClosingApps: true,
  autoRestartApps: true,
  restoreDefaultOnExit: false,
  vscodeReloadMode: "manual_reload_window",
  uiLanguage: "en",
  customPaths: []
};

const defaultRecoveryStatus: RecoveryStatus = {
  needsRecovery: false,
  transactionId: null,
  phase: null,
  message: "Recovery check has not run",
  backupManifestFound: false,
  backupEntryCount: null,
  rollbackAvailable: false,
  latestEventMessage: null
};

const defaultUsageHistory: UsageHistoryReport = {
  scannedAt: "Not scanned",
  codexHome: "",
  sessionsRoot: "",
  archivedSessionsRoot: "",
  filesScanned: 0,
  parseErrors: [],
  totals: {
    inputTokens: 0,
    cachedInputTokens: 0,
    outputTokens: 0,
    totalTokens: 0
  },
  latestQuota: null,
  sessions: []
};

export default function App() {
  const [tab, setTab] = useState<Tab>("home");
  const [switchOpen, setSwitchOpen] = useState(false);
  const [scan, setScan] = useState<EnvironmentScan>(emptyEnvironmentScan);
  const [scanBusy, setScanBusy] = useState(false);
  const [profiles, setProfiles] = useState<ProfileMetadata[]>([]);
  const [history, setHistory] = useState<SwitchHistoryEntry[]>([]);
  const [settings, setSettings] = useState<AppSettings>(defaultSettings);
  const [recovery, setRecovery] = useState<RecoveryStatus>(defaultRecoveryStatus);
  const [usageHistory, setUsageHistory] = useState<UsageHistoryReport>(defaultUsageHistory);
  const [usageBusy, setUsageBusy] = useState(false);
  const [usageMessage, setUsageMessage] = useState<string | null>(null);
  const [quickSwitchMessage, setQuickSwitchMessage] = useState<string | null>(null);
  const closeAfterExitRestore = useRef(false);
  const t = useMemo(() => createTranslator(settings.uiLanguage), [settings.uiLanguage]);

  const defaultProfile = useMemo(() => profiles.find((profile) => profile.defaultProfile), [profiles]);
  const currentProfile = useMemo(() => {
    const recentlyUsed = profiles
      .filter((profile) => profile.lastUsedAt)
      .sort((left, right) => Number(right.lastUsedAt ?? 0) - Number(left.lastUsedAt ?? 0))[0];
    return recentlyUsed ?? defaultProfile;
  }, [defaultProfile, profiles]);
  const currentSwitchHistory = useMemo(() => {
    if (!currentProfile) {
      return undefined;
    }
    return history.find((item) =>
      item.toProfileId === currentProfile.id && item.switchedAt === currentProfile.lastUsedAt
    ) ?? history.find((item) => item.toProfileId === currentProfile.id)
      ?? history.find((item) => item.toProfile === currentProfile.name && item.switchedAt === currentProfile.lastUsedAt)
      ?? history.find((item) => item.toProfile === currentProfile.name);
  }, [currentProfile, history]);
  const previousSwitchCandidateAvailable = useMemo(
    () => history.some((item) => item.fromProfileId || item.fromProfile),
    [history]
  );

  useEffect(() => {
    void runScan();
    void refreshProfiles();
    void refreshHistory();
    void refreshSettings();
    void refreshRecovery();
    void refreshUsageHistory();
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    try {
      void getCurrentWindow().onCloseRequested(async (event) => {
        if (closeAfterExitRestore.current || !settings.restoreDefaultOnExit) {
          return;
        }
        event.preventDefault();
        setQuickSwitchMessage("Restoring default account before exit.");
        try {
          const result = await restoreDefaultOnExit();
          setQuickSwitchMessage(
            result.attempted
              ? `Exit restore finished: ${result.switchResult?.transaction.phase ?? "not attempted"}.`
              : `Exit restore skipped: ${result.reason}.`
          );
          closeAfterExitRestore.current = true;
          await getCurrentWindow().close();
        } catch (error) {
          setQuickSwitchMessage(`Exit restore failed: ${String(error)}`);
        }
      }).then((handler) => {
        unlisten = handler;
      });
    } catch {
      // Browser-only development sessions do not expose a Tauri window.
    }
    return () => {
      unlisten?.();
    };
  }, [settings.restoreDefaultOnExit]);

  async function runScan() {
    setScanBusy(true);
    try {
      setScan(await detectEnvironments());
    } finally {
      setScanBusy(false);
    }
  }

  async function refreshProfiles() {
    setProfiles(await listProfiles());
  }

  async function refreshHistory() {
    setHistory(await listSwitchHistory());
  }

  async function refreshSettings() {
    try {
      setSettings(await getSettings());
    } catch {
      setSettings(defaultSettings);
    }
  }

  async function refreshRecovery() {
    setRecovery(await checkRecoveryStatus());
  }

  async function refreshUsageHistory() {
    setUsageBusy(true);
    setUsageMessage(null);
    try {
      const report = await getUsageHistory();
      setUsageHistory(report);
      setUsageMessage(`Usage scan finished: ${report.sessions.length} session(s), ${report.filesScanned} file(s).`);
    } catch (error) {
      setUsageMessage(`Usage scan failed: ${String(error)}`);
    } finally {
      setUsageBusy(false);
    }
  }

  async function resolveRecovery() {
    const nextRecovery = await resolveRecoveryStatus();
    setRecovery(nextRecovery);
    setQuickSwitchMessage("Recovery journal marked reviewed. Use Restore default or Switch back if the local account state still needs correction.");
  }

  async function rollbackRecovery() {
    if (!window.confirm("Rollback the unfinished transaction from its persisted backup manifest?")) {
      return;
    }
    setQuickSwitchMessage("Manual recovery rollback started.");
    try {
      const result = await rollbackUnfinishedTransaction();
      setRecovery(result.status);
      await refreshProfiles();
      await refreshHistory();
      await runScan();
      setQuickSwitchMessage(`${result.message}: ${result.transaction.phase}.`);
    } catch (error) {
      setQuickSwitchMessage(`Manual recovery rollback failed: ${String(error)}`);
    }
  }

  async function updateSettings(next: AppSettings) {
    setSettings(await saveSettings(next));
  }

  async function clearHistory() {
    await clearSwitchHistory();
    await refreshHistory();
  }

  function recoverySwitchRequest() {
    return {
      autoRestartApps: settings.autoRestartApps,
      vscodeReloadMode: settings.vscodeReloadMode,
      confirmProcessClose: true,
      desktopAppPath: scan.environments.find((environment) => environment.id === "Desktop")?.executablePath ?? null,
      vscodeAppPath: scan.environments.find((environment) => environment.id === "VS Code")?.executablePath ?? null,
      quitTimeoutMs: 8000
    } satisfies ProfileRecoverySwitchRequest;
  }

  async function runRecoveryProfileSwitch(
    label: string,
    operation: (request: ProfileRecoverySwitchRequest) => Promise<ProfileRecoverySwitchResult>
  ) {
    if (settings.confirmBeforeClosingApps && !window.confirm(`Close running Desktop / VS Code windows if needed, then ${label}?`)) {
      return;
    }
    setQuickSwitchMessage(`${label} started.`);
    try {
      const result = await operation({
        ...recoverySwitchRequest(),
        confirmProcessClose: true
      });
      await refreshProfiles();
      await refreshHistory();
      await refreshRecovery();
      await runScan();
      const phase = result.switchResult?.transaction.phase;
      const target = result.targetProfile?.name;
      setQuickSwitchMessage(
        result.attempted
          ? `${label} finished for ${target ?? "target profile"}: ${phase ?? result.reason}.`
          : `${label} skipped: ${result.reason}.`
      );
    } catch (error) {
      setQuickSwitchMessage(`${label} failed: ${String(error)}`);
    }
  }

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <ShieldCheck size={24} />
          <div>
            <strong>Codex Switch</strong>
            <span>{t("brandSubtitle")}</span>
          </div>
        </div>
        <nav className="nav-list" aria-label="Primary">
          <NavButton icon={<Laptop size={18} />} label={t("navHome")} active={tab === "home"} onClick={() => setTab("home")} />
          <NavButton icon={<KeyRound size={18} />} label={t("navProfiles")} active={tab === "profiles"} onClick={() => setTab("profiles")} />
          <NavButton icon={<FolderSearch size={18} />} label={t("navEnvironment")} active={tab === "environment"} onClick={() => setTab("environment")} />
          <NavButton icon={<BarChart3 size={18} />} label={t("navUsage")} active={tab === "usage"} onClick={() => setTab("usage")} />
          <NavButton icon={<Settings size={18} />} label={t("navSettings")} active={tab === "settings"} onClick={() => setTab("settings")} />
        </nav>
      </aside>

      <main className="content">
        {tab === "home" && (
          <Home
            currentProfile={currentProfile}
            currentSwitchHistory={currentSwitchHistory}
            defaultProfile={defaultProfile}
            previousSwitchCandidateAvailable={previousSwitchCandidateAvailable}
            scan={scan}
            history={history}
            recovery={recovery}
            quickSwitchMessage={quickSwitchMessage}
            t={t}
            onSwitch={() => setSwitchOpen(true)}
            onResolveRecovery={() => void resolveRecovery()}
            onRollbackRecovery={() => void rollbackRecovery()}
            onRestoreDefault={() => {
              void runRecoveryProfileSwitch("Restore default account", restoreDefaultProfile);
            }}
            onSwitchPrevious={() => {
              void runRecoveryProfileSwitch("Switch back to previous account", switchPreviousProfile);
            }}
          />
        )}
        {tab === "profiles" && (
          <Profiles
            profiles={profiles}
            scan={scan}
            scanBusy={scanBusy}
            t={t}
            onScan={runScan}
            onProfilesChanged={refreshProfiles}
          />
        )}
        {tab === "environment" && <Environment scan={scan} busy={scanBusy} t={t} onScan={runScan} />}
        {tab === "usage" && (
          <UsageHistoryView
            report={usageHistory}
            busy={usageBusy}
            message={usageMessage}
            switchHistory={history}
            t={t}
            onRefresh={() => void refreshUsageHistory()}
          />
        )}
        {tab === "settings" && (
          <SettingsView
            settings={settings}
            t={t}
            onChange={updateSettings}
            onClearHistory={clearHistory}
          />
        )}
      </main>

      {switchOpen && (
        <SwitchDialog
          profiles={profiles}
          settings={settings}
          scan={scan}
          t={t}
          onSwitched={async () => {
            await refreshProfiles();
            await refreshHistory();
            await refreshRecovery();
            await runScan();
          }}
          onClose={() => setSwitchOpen(false)}
        />
      )}
    </div>
  );
}

function NavButton({
  icon,
  label,
  active,
  onClick
}: {
  icon: React.ReactNode;
  label: string;
  active: boolean;
  onClick: () => void;
}) {
  return (
    <button className={`nav-button ${active ? "active" : ""}`} onClick={onClick}>
      {icon}
      <span>{label}</span>
    </button>
  );
}

function Home({
  currentProfile,
  currentSwitchHistory,
  defaultProfile,
  previousSwitchCandidateAvailable,
  scan,
  history,
  recovery,
  quickSwitchMessage,
  t,
  onSwitch,
  onResolveRecovery,
  onRollbackRecovery,
  onRestoreDefault,
  onSwitchPrevious
}: {
  currentProfile?: ProfileMetadata;
  currentSwitchHistory?: SwitchHistoryEntry;
  defaultProfile?: ProfileMetadata;
  previousSwitchCandidateAvailable: boolean;
  scan: EnvironmentScan;
  history: SwitchHistoryEntry[];
  recovery: RecoveryStatus;
  quickSwitchMessage: string | null;
  t: Translate;
  onSwitch: () => void;
  onResolveRecovery: () => void;
  onRollbackRecovery: () => void;
  onRestoreDefault: () => void;
  onSwitchPrevious: () => void;
}) {
  return (
    <section className="view">
      <header className="view-header">
        <div>
          <p className="eyebrow">{t("currentAccount")}</p>
          <h1>{currentProfile?.accountHint ?? t("noVerifiedAccount")}</h1>
          <span className="scan-meta">
            {currentProfile
              ? `${currentProfile.name}${currentProfile.lastUsedAt ? ` · ${t("lastUsed", { value: currentProfile.lastUsedAt })}` : ""}`
              : t("noProfileSwitched")}
          </span>
          {currentProfile && (
            <span className={`account-verification ${verificationClass(currentSwitchHistory)}`}>
              {verificationLabel(currentSwitchHistory, t)}
            </span>
          )}
        </div>
        <div className="header-actions">
          <button className="primary-button" onClick={onSwitch}>
            <RefreshCw size={18} />
            {t("oneClickSwitch")}
          </button>
          <button className="secondary-button" onClick={onRestoreDefault} disabled={!defaultProfile}>
            <RotateCcw size={18} />
            {t("restoreDefault")}
          </button>
          <button className="secondary-button" onClick={onSwitchPrevious} disabled={!previousSwitchCandidateAvailable}>
            <History size={18} />
            {t("switchBack")}
          </button>
        </div>
      </header>

      <div className="status-grid">
        {scan.environments.map((environment) => (
          <EnvironmentSummary key={environment.id} environment={environment} t={t} />
        ))}
      </div>

      {recovery.needsRecovery && (
        <section className="recovery-banner">
          <AlertTriangle size={18} />
          <span>
            {recovery.message}
            <em>
              {recovery.backupManifestFound
                ? `${t("backupManifestFound")}${recovery.backupEntryCount === null ? "" : ` · ${t("backupEntries", { count: recovery.backupEntryCount })}`}${recovery.rollbackAvailable ? ` · ${t("rollbackEvidenceAvailable")}` : ""}`
                : t("backupManifestMissing")}
              {recovery.latestEventMessage ? ` · ${recovery.latestEventMessage}` : ""}
            </em>
          </span>
          <button className="secondary-button compact" onClick={onRollbackRecovery} disabled={!recovery.rollbackAvailable}>
            {t("rollBack")}
          </button>
          <button className="secondary-button compact" onClick={onResolveRecovery}>{t("markReviewed")}</button>
        </section>
      )}

      {quickSwitchMessage && (
        <section className="recovery-banner">
          <ListChecks size={18} />
          <span>{quickSwitchMessage}</span>
        </section>
      )}

      <section className="panel">
        <div className="panel-title">
          <History size={18} />
          <h2>{t("recentSwitchHistory")}</h2>
        </div>
        <div className="history-list">
          {history.length === 0 && (
            <div className="history-row">
              <span>{t("noSwitchHistory")}</span>
              <strong>- {"->"} -</strong>
              <em>{t("waitingFirstTransaction")}</em>
            </div>
          )}
          {history.map((item) => (
            <div className="history-row" key={item.id}>
              <span>{item.switchedAt}</span>
              <strong>{item.fromProfile ?? "-"} {"->"} {item.toProfile}</strong>
              <em>{item.status}{item.errorType ? ` · ${item.errorType}` : ""}</em>
            </div>
          ))}
        </div>
      </section>
    </section>
  );
}

function Profiles({
  profiles,
  scan,
  scanBusy,
  t,
  onScan,
  onProfilesChanged
}: {
  profiles: ProfileMetadata[];
  scan: EnvironmentScan;
  scanBusy: boolean;
  t: Translate;
  onScan: () => Promise<void>;
  onProfilesChanged: () => Promise<void>;
}) {
  const [name, setName] = useState("Imported Profile");
  const [tags, setTags] = useState("current");
  const [note, setNote] = useState("Imported from current local Codex state.");
  const [selected, setSelected] = useState<Record<TargetEnvironment, boolean>>({
    cli: true,
    vscode: true,
    desktop: true
  });
  const [confirmSameAccount, setConfirmSameAccount] = useState(false);
  const [makeDefault, setMakeDefault] = useState(profiles.length === 0);
  const [importing, setImporting] = useState(false);
  const [preflightBusy, setPreflightBusy] = useState(false);
  const [preflight, setPreflight] = useState<ProfileImportPreflightResult | null>(null);
  const [editingProfileId, setEditingProfileId] = useState<string | null>(null);
  const [editName, setEditName] = useState("");
  const [editTags, setEditTags] = useState("");
  const [editNote, setEditNote] = useState("");
  const [busyProfileId, setBusyProfileId] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const selectedEnvironments = (Object.entries(selected) as Array<[TargetEnvironment, boolean]>)
    .filter(([, enabled]) => enabled)
    .map(([environment]) => environment);
  const requiresSameAccountConfirmation = selectedEnvironments.length > 1 && !confirmSameAccount;

  async function previewImportCoverage() {
    if (selectedEnvironments.length === 0) {
      setMessage("Preview failed: select at least one environment.");
      return;
    }
    setPreflightBusy(true);
    setMessage(null);
    try {
      const result = await previewCurrentImport({ environments: selectedEnvironments });
      setPreflight(result);
      const readyCount = result.environments.filter((environment) => environment.readiness === "ready").length;
      setMessage(`Preview complete: ${readyCount} environment(s) have readable artifacts.`);
    } catch (error) {
      setMessage(`Preview failed: ${String(error)}`);
    } finally {
      setPreflightBusy(false);
    }
  }

  async function saveCurrentLogin() {
    if (selectedEnvironments.length === 0) {
      setMessage("Import failed: select at least one environment.");
      return;
    }
    if (requiresSameAccountConfirmation) {
      setMessage("Import blocked: confirm the selected environments belong to the same authorized account.");
      return;
    }
    setImporting(true);
    setMessage(null);
    try {
      const result = await importCurrentProfile({
        name,
        tags: tags.split(",").map((tag) => tag.trim()).filter(Boolean),
        note,
        environments: selectedEnvironments,
        confirmSameAccount,
        defaultProfile: makeDefault
      });
      await onProfilesChanged();
      const importedCount = result.importedEnvironments
        .reduce((sum, item) => sum + item.artifactCount, 0);
      setMessage(`Saved ${result.profile.name}; captured ${importedCount} local artifacts into the secret vault.`);
    } catch (error) {
      setMessage(`Import failed: ${String(error)}`);
    } finally {
      setImporting(false);
    }
  }

  function startEdit(profile: ProfileMetadata) {
    setEditingProfileId(profile.id);
    setEditName(profile.name);
    setEditTags(profile.tags.join(", "));
    setEditNote(profile.note);
    setMessage(null);
  }

  async function saveProfileEdit(profile: ProfileMetadata) {
    setBusyProfileId(profile.id);
    setMessage(null);
    try {
      const updated = await updateProfile({
        profileId: profile.id,
        name: editName,
        tags: editTags.split(",").map((tag) => tag.trim()).filter(Boolean),
        note: editNote,
        defaultProfile: profile.defaultProfile
      });
      setEditingProfileId(null);
      await onProfilesChanged();
      setMessage(`Updated ${updated.name}.`);
    } catch (error) {
      setMessage(`Update failed: ${String(error)}`);
    } finally {
      setBusyProfileId(null);
    }
  }

  async function setDefaultProfile(profile: ProfileMetadata) {
    setBusyProfileId(profile.id);
    setMessage(null);
    try {
      const updated = await updateProfile({
        profileId: profile.id,
        name: profile.name,
        tags: profile.tags,
        note: profile.note,
        defaultProfile: true
      });
      await onProfilesChanged();
      setMessage(`${updated.name} is now the default profile.`);
    } catch (error) {
      setMessage(`Default update failed: ${String(error)}`);
    } finally {
      setBusyProfileId(null);
    }
  }

  async function removeProfile(profile: ProfileMetadata) {
    if (!window.confirm(`Delete profile "${profile.name}" and its stored secret payloads?`)) {
      return;
    }
    setBusyProfileId(profile.id);
    setMessage(null);
    try {
      await deleteProfile(profile.id);
      if (editingProfileId === profile.id) {
        setEditingProfileId(null);
      }
      await onProfilesChanged();
      setMessage(`Deleted ${profile.name}.`);
    } catch (error) {
      setMessage(`Delete failed: ${String(error)}`);
    } finally {
      setBusyProfileId(null);
    }
  }

  return (
    <section className="view">
      <header className="view-header">
        <div>
          <p className="eyebrow">{t("profileManagement")}</p>
          <h1>{t("authorizedProfiles")}</h1>
        </div>
        <button className="secondary-button" onClick={saveCurrentLogin} disabled={importing}>
          <UserPlus size={18} />
          {importing ? t("saving") : t("saveCurrentLogin")}
        </button>
      </header>

      <section className="import-panel">
        <div className="import-guide">
          <div>
            <p className="eyebrow">{t("newAccountWorkflow")}</p>
            <h2>{t("officialLoginFirst")}</h2>
          </div>
          <ol>
            <li>{t("importStepOfficial")}</li>
            <li>{t("importStepRescan")}</li>
            <li>{t("importStepSameAccount")}</li>
          </ol>
          <div className="import-guide-actions">
            <button className="secondary-button compact" onClick={() => void previewImportCoverage()} disabled={preflightBusy || selectedEnvironments.length === 0}>
              <ListChecks size={14} />
              {preflightBusy ? t("previewing") : t("previewCapture")}
            </button>
            <button className="secondary-button compact" onClick={() => void onScan()} disabled={scanBusy}>
              <FolderSearch size={14} />
              {scanBusy ? t("scanning") : t("rescanCurrentState")}
            </button>
          </div>
        </div>
        <div className="import-detected-grid">
          {(["cli", "vscode", "desktop"] as TargetEnvironment[]).map((environment) => {
            const detected = scanEnvironmentForTarget(scan, environment);
            const existingPaths = detected?.discoveredPaths.filter((path) => path.exists).length ?? 0;
            return (
              <article key={environment}>
                <strong>{environmentLabel(environment)}</strong>
                <span>{detected?.support ?? "not-detected"} - {detected?.accountHint ?? t("unknown")}</span>
                <em>{t("existingCandidatePaths", { count: existingPaths })}</em>
              </article>
            );
          })}
        </div>
        <div className="import-fields">
          <label>
            <span>{t("profileName")}</span>
            <input value={name} onChange={(event) => setName(event.target.value)} />
          </label>
          <label>
            <span>{t("tags")}</span>
            <input value={tags} onChange={(event) => setTags(event.target.value)} />
          </label>
          <label>
            <span>{t("note")}</span>
            <input value={note} onChange={(event) => setNote(event.target.value)} />
          </label>
        </div>
        <div className="scope-picker inline">
          {(["cli", "vscode", "desktop"] as TargetEnvironment[]).map((environment) => (
            <label key={environment}>
              <input
                type="checkbox"
                checked={selected[environment]}
                onChange={(event) => setSelected((current) => ({ ...current, [environment]: event.target.checked }))}
              />
              <span>{environmentLabel(environment)}</span>
              <em>{scanEnvironmentForTarget(scan, environment)?.accountHint ?? t("unknown")}</em>
            </label>
          ))}
        </div>
        <div className="import-options">
          <label>
            <input
              type="checkbox"
              checked={confirmSameAccount}
              onChange={(event) => setConfirmSameAccount(event.target.checked)}
            />
            <span>{t("sameAuthorizedAccount")}</span>
          </label>
          <label>
            <input
              type="checkbox"
              checked={makeDefault}
              onChange={(event) => setMakeDefault(event.target.checked)}
            />
            <span>{t("setAsDefaultProfile")}</span>
          </label>
        </div>
        {requiresSameAccountConfirmation && (
          <p className="import-warning">{t("sameAccountRequired")}</p>
        )}
        {preflight && (
          <div className="import-preflight">
            {preflight.environments.map((environment) => (
              <article className={`preflight-row ${environment.readiness}`} key={environment.environment}>
                <div>
                  <strong>{environmentLabel(environment.environment)}</strong>
                  <span>{preflightReadinessLabel(environment.readiness, t)} · {environment.accountHint}</span>
                </div>
                <dl>
                  <div>
                    <dt>{t("candidates")}</dt>
                    <dd>{environment.existingCandidatePathCount}/{environment.candidatePathCount}</dd>
                  </div>
                  <div>
                    <dt>{t("capture")}</dt>
                    <dd>{t("filesEncodedBytes", { files: environment.capturedArtifactCount, bytes: environment.capturedBytes })}</dd>
                  </div>
                  <div>
                    <dt>{t("skipped")}</dt>
                    <dd>{environment.skippedArtifactCount}</dd>
                  </div>
                </dl>
                {environment.skippedReasons.length > 0 && (
                  <em>{environment.skippedReasons.map((reason) => `${reason.reason}: ${reason.count}`).join("; ")}</em>
                )}
              </article>
            ))}
            {preflight.warnings.length > 0 && (
              <p className="import-warning">{preflight.warnings.join(" ")}</p>
            )}
          </div>
        )}
        {message && <p className="import-message">{message}</p>}
      </section>

      <div className="profile-grid">
        {profiles.length === 0 && (
          <article className="empty-state">
            <h2>{t("noSavedProfiles")}</h2>
            <p>{t("noSavedProfilesHint")}</p>
          </article>
        )}
        {profiles.map((profile) => {
          const editing = editingProfileId === profile.id;
          const busy = busyProfileId === profile.id;
          return (
          <article className="profile-card" key={profile.id}>
            {editing ? (
              <div className="profile-edit">
                <label>
                  <span>{t("name")}</span>
                  <input value={editName} onChange={(event) => setEditName(event.target.value)} />
                </label>
                <label>
                  <span>{t("tags")}</span>
                  <input value={editTags} onChange={(event) => setEditTags(event.target.value)} />
                </label>
                <label>
                  <span>{t("note")}</span>
                  <textarea value={editNote} onChange={(event) => setEditNote(event.target.value)} />
                </label>
              </div>
            ) : (
              <>
                <div className="profile-topline">
                  <div>
                    <h2>{profile.name}</h2>
                    <span>{profile.accountHint}</span>
                    <span>{profile.lastUsedAt ? t("lastUsed", { value: profile.lastUsedAt }) : t("neverSwitched")}</span>
                  </div>
                  {profile.defaultProfile && <strong className="pill">{t("default")}</strong>}
                </div>
                <p>{profile.note}</p>
              </>
            )}
            {profile.tags.length > 0 && (
              <div className="tag-list">
                {profile.tags.map((tag) => <span key={tag}>{tag}</span>)}
              </div>
            )}
            <div className="env-strip">
              {profile.environments.map((environment) => (
                <span className={`env-chip ${environment.status}`} key={environment.environment}>
                  {environmentLabel(environment.environment)}: {environment.status}
                </span>
              ))}
            </div>
            <div className="card-actions">
              {editing ? (
                <>
                  <button onClick={() => void saveProfileEdit(profile)} disabled={busy || editName.trim().length === 0}>
                    <Save size={14} />
                    {t("save")}
                  </button>
                  <button onClick={() => setEditingProfileId(null)} disabled={busy}>
                    <X size={14} />
                    {t("cancel")}
                  </button>
                </>
              ) : (
                <>
                  <button onClick={() => startEdit(profile)} disabled={busy}>
                    <Pencil size={14} />
                    {t("edit")}
                  </button>
                  {!profile.defaultProfile && (
                    <button onClick={() => void setDefaultProfile(profile)} disabled={busy}>
                      <Star size={14} />
                      {t("setDefault")}
                    </button>
                  )}
                  <button onClick={() => void removeProfile(profile)} disabled={busy} className="danger-card-button">
                    <Trash2 size={14} />
                    {t("delete")}
                  </button>
                </>
              )}
            </div>
          </article>
          );
        })}
      </div>
    </section>
  );
}

function environmentLabel(environment: TargetEnvironment) {
  if (environment === "cli") {
    return "CLI";
  }
  if (environment === "vscode") {
    return "VS Code";
  }
  return "Desktop";
}

function preflightReadinessLabel(readiness: string, t: Translate) {
  if (readiness === "ready") {
    return t("readyToImport");
  }
  if (readiness === "not_selected") {
    return t("notSelected");
  }
  if (readiness === "scan_missing") {
    return t("scanMissing");
  }
  return t("noReadableArtifacts");
}

function scanEnvironmentForTarget(scan: EnvironmentScan, target: TargetEnvironment) {
  const id: EnvironmentId = target === "cli" ? "CLI" : target === "vscode" ? "VS Code" : "Desktop";
  return scan.environments.find((environment) => environment.id === id);
}

function environmentProfileState(profile: ProfileMetadata | undefined, environment: TargetEnvironment) {
  return profile?.environments.find((state) => state.environment === environment);
}

function profileSupportsEnvironment(profile: ProfileMetadata | undefined, environment: TargetEnvironment) {
  return environmentProfileState(profile, environment)?.status === "available";
}

function previousUsableProfile(profiles: ProfileMetadata[], targetProfileId: string) {
  return profiles
    .filter((profile) => profile.id !== targetProfileId)
    .filter((profile) => profile.environments.some((state) => state.status === "available"))
    .sort((left, right) => Number(right.lastUsedAt ?? 0) - Number(left.lastUsedAt ?? 0))[0];
}

function verificationClass(history?: SwitchHistoryEntry) {
  if (!history) {
    return "unknown";
  }
  if (history.status === "success") {
    return "verified";
  }
  if (history.status === "incomplete") {
    return "incomplete";
  }
  return "failed";
}

function verificationLabel(history: SwitchHistoryEntry | undefined, t: Translate) {
  if (!history) {
    return t("noSwitchVerification");
  }
  if (history.status === "success") {
    return t("verifiedByHint");
  }
  if (history.status === "incomplete") {
    return history.errorType === "IdentityMismatch"
      ? t("accountHintMismatch")
      : t("identityIncomplete");
  }
  if (history.status === "rolled_back") {
    return t("lastSwitchRolledBack");
  }
  return t("lastSwitchFailed");
}

function UsageHistoryView({
  report,
  busy,
  message,
  switchHistory,
  t,
  onRefresh
}: {
  report: UsageHistoryReport;
  busy: boolean;
  message: string | null;
  switchHistory: SwitchHistoryEntry[];
  t: Translate;
  onRefresh: () => void;
}) {
  const recentSessions = report.sessions.slice(0, 12);
  return (
    <section className="view">
      <header className="view-header">
        <div>
          <p className="eyebrow">{t("usageAndHistory")}</p>
          <h1>{t("localCodexActivity")}</h1>
          <span className="scan-meta">
            {report.codexHome || t("codexHomeNotScanned")} · {report.scannedAt}
          </span>
        </div>
        <div className="header-actions">
          <button className="secondary-button" onClick={onRefresh} disabled={busy}>
            <RefreshCw size={18} />
            {busy ? t("scanning") : t("refreshUsage")}
          </button>
        </div>
      </header>

      {message && (
        <section className="recovery-banner">
          <ListChecks size={18} />
          <span>{message}</span>
        </section>
      )}

      <div className="usage-hero-grid">
        <UsageMetricCard title={t("totalTokens")} value={formatTokenCount(report.totals.totalTokens)} detail={t("inputCachedOutput")} />
        <UsageMetricCard title={t("input")} value={formatTokenCount(report.totals.inputTokens)} detail={t("cached", { value: formatTokenCount(report.totals.cachedInputTokens) })} />
        <UsageMetricCard title={t("output")} value={formatTokenCount(report.totals.outputTokens)} detail={t("sessions", { count: report.sessions.length })} />
        <UsageMetricCard title={t("filesScanned")} value={String(report.filesScanned)} detail={t("parseIssues", { count: report.parseErrors.length })} />
      </div>

      <section className="panel">
        <div className="panel-title">
          <BarChart3 size={18} />
          <h2>{t("currentUsageStatus")}</h2>
        </div>
        {report.latestQuota ? (
          <QuotaPanel quota={report.latestQuota} t={t} />
        ) : (
          <div className="empty-state compact">
            <h2>{t("noQuotaSnapshot")}</h2>
            <p>{t("noQuotaSnapshotHint")}</p>
          </div>
        )}
      </section>

      <section className="panel">
        <div className="panel-title">
          <History size={18} />
          <h2>{t("codexSessionHistory")}</h2>
        </div>
        <div className="usage-session-list">
          {recentSessions.length === 0 && (
            <div className="history-row">
              <span>{t("noSessions")}</span>
              <strong>-</strong>
              <em>{t("noTokenEvents")}</em>
            </div>
          )}
          {recentSessions.map((session) => (
            <UsageSessionRow key={`${session.sourcePath}-${session.latestEventAt ?? session.modifiedAt ?? "unknown"}`} session={session} t={t} />
          ))}
        </div>
      </section>

      <section className="panel">
        <div className="panel-title">
          <Database size={18} />
          <h2>{t("switchHistory")}</h2>
        </div>
        <div className="history-list">
          {switchHistory.length === 0 && (
            <div className="history-row">
              <span>{t("noSwitchHistory")}</span>
              <strong>- {"->"} -</strong>
              <em>{t("noSwitchTransaction")}</em>
            </div>
          )}
          {switchHistory.slice(0, 20).map((item) => (
            <div className="history-row" key={item.id}>
              <span>{formatDisplayTime(item.switchedAt)}</span>
              <strong>{item.fromProfile ?? "-"} {"->"} {item.toProfile}</strong>
              <em>{item.status}{item.errorType ? ` · ${item.errorType}` : ""}</em>
            </div>
          ))}
        </div>
      </section>

      <section className="panel">
        <div className="panel-title">
          <FolderSearch size={18} />
          <h2>{t("scannedLocations")}</h2>
        </div>
        <div className="usage-path-grid">
          <div>
            <span>{t("navUsage")}</span>
            <strong>{report.sessionsRoot || "-"}</strong>
          </div>
          <div>
            <span>{t("archivedSessions")}</span>
            <strong>{report.archivedSessionsRoot || "-"}</strong>
          </div>
        </div>
        {report.parseErrors.length > 0 && (
          <div className="usage-errors">
            {report.parseErrors.slice(0, 5).map((error) => <span key={error}>{error}</span>)}
          </div>
        )}
      </section>
    </section>
  );
}

function UsageMetricCard({ title, value, detail }: { title: string; value: string; detail: string }) {
  return (
    <article className="usage-metric-card">
      <span>{title}</span>
      <strong>{value}</strong>
      <em>{detail}</em>
    </article>
  );
}

function QuotaPanel({ quota, t }: { quota: UsageQuotaSummary; t: Translate }) {
  return (
    <div className="quota-grid">
      <QuotaWindow title={t("fiveHourWindow")} window={quota.fiveHour} t={t} />
      <QuotaWindow title={t("weeklyWindow")} window={quota.weekly} t={t} />
    </div>
  );
}

function QuotaWindow({ title, window, t }: { title: string; window: UsageQuotaSummary["fiveHour"]; t: Translate }) {
  const percent = window.remainingPercent ?? 0;
  return (
    <article className={`quota-window ${window.remainingPercent === null ? "unknown" : ""}`}>
      <div>
        <span>{title}</span>
        <strong>{window.remainingPercent === null ? t("quotaUnknown") : t("quotaRemaining", { value: window.remainingPercent })}</strong>
        <em>{window.resetAt ? t("quotaResets", { value: formatDisplayTime(window.resetAt) }) : t("noResetTime")}</em>
      </div>
      <div className="quota-track" aria-hidden="true">
        <div className="quota-fill" style={{ width: `${percent}%` }} />
      </div>
    </article>
  );
}

function UsageSessionRow({ session, t }: { session: UsageSessionSummary; t: Translate }) {
  return (
    <article className="usage-session-row">
      <div>
        <strong>{session.sessionId ?? t("unknownSession")}</strong>
        <span>{session.model} · {formatDisplayTime(session.latestEventAt ?? session.modifiedAt)}</span>
      </div>
      <dl>
        <div>
          <dt>{t("total")}</dt>
          <dd>{formatTokenCount(session.tokens.totalTokens)}</dd>
        </div>
        <div>
          <dt>{t("input")}</dt>
          <dd>{formatTokenCount(session.tokens.inputTokens)}</dd>
        </div>
        <div>
          <dt>{t("cached", { value: "" }).trim()}</dt>
          <dd>{formatTokenCount(session.tokens.cachedInputTokens)}</dd>
        </div>
        <div>
          <dt>{t("output")}</dt>
          <dd>{formatTokenCount(session.tokens.outputTokens)}</dd>
        </div>
      </dl>
      <em>{t("tokenEvents", { count: session.tokenEvents })}</em>
    </article>
  );
}

function formatTokenCount(value: number) {
  return new Intl.NumberFormat("en-US").format(value);
}

function formatDisplayTime(value: string | null | undefined) {
  if (!value) {
    return "Unknown time";
  }
  if (/^\d+$/.test(value)) {
    const numeric = Number(value);
    const millis = value.length <= 10 ? numeric * 1000 : numeric;
    return new Date(millis).toLocaleString();
  }
  const parsed = Date.parse(value);
  return Number.isNaN(parsed) ? value : new Date(parsed).toLocaleString();
}

function Environment({ scan, busy, t, onScan }: { scan: EnvironmentScan; busy: boolean; t: Translate; onScan: () => void }) {
  const [restartMessage, setRestartMessage] = useState<string | null>(null);
  const [diagnosticsText, setDiagnosticsText] = useState<string | null>(null);
  const [diagnosticsMessage, setDiagnosticsMessage] = useState<string | null>(null);

  async function restartEnvironment(environment: EnvironmentState) {
    setRestartMessage(`Restarting ${environment.id}.`);
    try {
      const response = environment.id === "Desktop"
        ? await restartDesktopApp(environment.executablePath)
        : await restartVscodeApp(environment.executablePath);
      setRestartMessage(response.message);
      onScan();
    } catch (error) {
      setRestartMessage(`Restart ${environment.id} failed: ${String(error)}`);
    }
  }

  async function generateDiagnostics() {
    setDiagnosticsMessage("Generating read-only diagnostics.");
    try {
      const report = await environmentDiagnosticsReport();
      setDiagnosticsText(JSON.stringify(report, null, 2));
      setDiagnosticsMessage(`Diagnostics generated at ${report.generatedAt}.`);
    } catch (error) {
      setDiagnosticsMessage(`Diagnostics failed: ${String(error)}`);
    }
  }

  async function copyDiagnostics() {
    if (!diagnosticsText) {
      return;
    }
    try {
      await navigator.clipboard.writeText(diagnosticsText);
      setDiagnosticsMessage("Diagnostics copied to clipboard.");
    } catch (error) {
      setDiagnosticsMessage(`Clipboard copy failed: ${String(error)}`);
    }
  }

  return (
    <section className="view">
      <header className="view-header">
        <div>
          <p className="eyebrow">{t("readOnlyDetection")}</p>
          <h1>{t("environmentStatus")}</h1>
          <span className="scan-meta">{scan.os} · {scan.scannedAt} · {scan.readOnly ? t("readOnly") : t("writeEnabled")}</span>
        </div>
        <div className="header-actions">
          <button className="secondary-button" onClick={() => void generateDiagnostics()}>
            <ClipboardList size={18} />
            {t("diagnostics")}
          </button>
          <button className="secondary-button" onClick={onScan} disabled={busy}>
            <FolderSearch size={18} />
            {busy ? t("scanning") : t("rescan")}
          </button>
        </div>
      </header>

      {restartMessage && (
        <section className="recovery-banner">
          <RefreshCw size={18} />
          <span>{restartMessage}</span>
        </section>
      )}

      {(diagnosticsText || diagnosticsMessage) && (
        <section className="diagnostics-panel">
          <div className="diagnostics-heading">
            <div>
              <p className="eyebrow">{t("readOnlyReport")}</p>
              <h2>{t("detectionDiagnostics")}</h2>
              {diagnosticsMessage && <span>{diagnosticsMessage}</span>}
            </div>
            <div className="header-actions">
              <button className="secondary-button compact" onClick={() => void copyDiagnostics()} disabled={!diagnosticsText}>
                <Copy size={14} />
                {t("copy")}
              </button>
              <button className="secondary-button compact" onClick={() => {
                setDiagnosticsText(null);
                setDiagnosticsMessage(null);
              }}>
                <X size={14} />
                {t("clear")}
              </button>
            </div>
          </div>
          {diagnosticsText && (
            <textarea
              className="diagnostics-output"
              readOnly
              spellCheck={false}
              value={diagnosticsText}
            />
          )}
        </section>
      )}

      <div className="environment-list">
        {scan.environments.map((environment) => (
          <article className="environment-row" key={environment.id}>
            <div className="environment-row-heading">
              <EnvironmentSummary environment={environment} t={t} />
              {(environment.id === "Desktop" || environment.id === "VS Code") && (
                <button
                  className="secondary-button compact"
                  onClick={() => void restartEnvironment(environment)}
                  disabled={!environment.executablePath}
                >
                  <RefreshCw size={14} />
                  {t("restart")}
                </button>
              )}
            </div>
            <dl>
              <div>
                <dt>{t("appPath")}</dt>
                <dd>{environment.executablePath ?? t("notDetected")}</dd>
              </div>
              <div>
                <dt>{t("running")}</dt>
                <dd>{environment.runningProcesses.length > 0 ? environment.runningProcesses.join(", ") : t("noMatchingProcess")}</dd>
              </div>
              <div>
                <dt>{t("status")}</dt>
                <dd>{environment.statusMessage}</dd>
              </div>
              <div>
                <dt>{t("permission")}</dt>
                <dd>{environment.permission}</dd>
              </div>
              <div className="path-list">
                <dt>{t("discoveredPaths")}</dt>
                <dd>
                  {environment.discoveredPaths.length === 0 ? (
                    t("noCandidatePaths")
                  ) : (
                    <ul>
                      {environment.discoveredPaths.map((item) => (
                        <li key={`${item.kind}-${item.path}`}>
                          <strong>{item.kind}</strong>
                          <span>{item.path}</span>
                          <em>{item.exists ? item.permission : t("missing")}</em>
                        </li>
                      ))}
                    </ul>
                  )}
                </dd>
              </div>
            </dl>
          </article>
        ))}
      </div>
    </section>
  );
}

function SettingsView({
  settings,
  t,
  onChange,
  onClearHistory
}: {
  settings: AppSettings;
  t: Translate;
  onChange: (settings: AppSettings) => Promise<void>;
  onClearHistory: () => Promise<void>;
}) {
  const customPaths = settings.customPaths ?? [];

  function toggleScope(environment: TargetEnvironment, enabled: boolean) {
    const nextScope = enabled
      ? Array.from(new Set([...settings.defaultScope, environment]))
      : settings.defaultScope.filter((item) => item !== environment);
    void onChange({ ...settings, defaultScope: nextScope });
  }

  function updateCustomPath(
    index: number,
    field: "environment" | "kind" | "path",
    value: TargetEnvironment | EnvironmentPathKind | string
  ) {
    const nextCustomPaths = customPaths.map((item, itemIndex) => (
      itemIndex === index ? { ...item, [field]: value } : item
    ));
    void onChange({ ...settings, customPaths: nextCustomPaths });
  }

  function addCustomPath() {
    void onChange({
      ...settings,
      customPaths: [
        ...customPaths,
        {
          environment: "vscode",
          kind: "auth",
          path: "~/Library/Application Support/Code/User/globalStorage/openai.chatgpt"
        }
      ]
    });
  }

  function removeCustomPath(index: number) {
    void onChange({
      ...settings,
      customPaths: customPaths.filter((_, itemIndex) => itemIndex !== index)
    });
  }

  return (
    <section className="view">
      <header className="view-header">
        <div>
          <p className="eyebrow">{t("switchPolicy")}</p>
          <h1>{t("settings")}</h1>
        </div>
        <button className="secondary-button" onClick={() => void onChange({ ...defaultSettings, uiLanguage: settings.uiLanguage })}>
          <RotateCcw size={18} />
          {t("restoreDefaults")}
        </button>
      </header>

      <section className="settings-grid">
        <label className="setting-row">
          <span>{t("interfaceLanguage")}</span>
          <select
            value={settings.uiLanguage}
            onChange={(event) => void onChange({ ...settings, uiLanguage: event.target.value as AppSettings["uiLanguage"] })}
          >
            <option value="en">{t("english")}</option>
            <option value="zh-CN">{t("chinese")}</option>
          </select>
        </label>
        <div className="setting-row stacked">
          <span>{t("defaultSwitchScope")}</span>
          <div className="scope-picker inline">
            {(["cli", "vscode", "desktop"] as TargetEnvironment[]).map((environment) => (
              <label key={environment}>
                <input
                  type="checkbox"
                  checked={settings.defaultScope.includes(environment)}
                  onChange={(event) => toggleScope(environment, event.target.checked)}
                />
                <span>{environmentLabel(environment)}</span>
              </label>
            ))}
          </div>
        </div>
        <label className="setting-row">
          <span>{t("vscodePostSwitchAction")}</span>
          <select
            value={settings.vscodeReloadMode}
            onChange={(event) => void onChange({ ...settings, vscodeReloadMode: event.target.value as AppSettings["vscodeReloadMode"] })}
          >
            <option value="manual_reload_window">{t("manualReloadWindow")}</option>
            <option value="restart_app">{t("restartVscode")}</option>
            <option value="none">{t("noReload")}</option>
          </select>
        </label>
        <label className="setting-row">
          <span>{t("confirmBeforeClosingApps")}</span>
          <input
            type="checkbox"
            checked={settings.confirmBeforeClosingApps}
            onChange={(event) => void onChange({ ...settings, confirmBeforeClosingApps: event.target.checked })}
          />
        </label>
        <label className="setting-row">
          <span>{t("autoRestartApps")}</span>
          <input
            type="checkbox"
            checked={settings.autoRestartApps}
            onChange={(event) => void onChange({ ...settings, autoRestartApps: event.target.checked })}
          />
        </label>
        <label className="setting-row">
          <span>{t("restoreDefaultOnExit")}</span>
          <input
            type="checkbox"
            checked={settings.restoreDefaultOnExit}
            onChange={(event) => void onChange({ ...settings, restoreDefaultOnExit: event.target.checked })}
          />
        </label>
        <div className="setting-row stacked path-overrides">
          <div className="setting-section-title">
            <span>{t("customDetectorPaths")}</span>
            <button className="secondary-button compact" onClick={addCustomPath}>
              <Plus size={14} />
              {t("addPath")}
            </button>
          </div>
          <div className="path-override-list">
            {customPaths.length === 0 ? (
              <p>{t("noCustomPaths")}</p>
            ) : customPaths.map((override, index) => (
              <div className="path-override-row" key={`${override.environment}-${override.kind}-${index}`}>
                <select
                  value={override.environment}
                  onChange={(event) => updateCustomPath(index, "environment", event.target.value as TargetEnvironment)}
                >
                  <option value="cli">CLI</option>
                  <option value="vscode">VS Code</option>
                  <option value="desktop">Desktop</option>
                </select>
                <select
                  value={override.kind}
                  onChange={(event) => updateCustomPath(index, "kind", event.target.value as EnvironmentPathKind)}
                >
                  <option value="auth">{t("auth")}</option>
                  <option value="config">{t("config")}</option>
                  <option value="cache">{t("cache")}</option>
                  <option value="app">{t("app")}</option>
                </select>
                <input
                  value={override.path}
                  onChange={(event) => updateCustomPath(index, "path", event.target.value)}
                  placeholder="~/Library/Application Support/..."
                />
                <button
                  className="secondary-button compact icon-only"
                  onClick={() => removeCustomPath(index)}
                  aria-label={t("removeCustomPath")}
                >
                  <Trash2 size={14} />
                </button>
              </div>
            ))}
          </div>
        </div>
        <button className="danger-button" onClick={() => void onClearHistory()}>{t("clearLocalHistory")}</button>
      </section>
    </section>
  );
}

function EnvironmentSummary({ environment, t }: { environment: EnvironmentState; t: Translate }) {
  const Icon = environment.id === "CLI" ? SquareTerminal : environment.id === "VS Code" ? Database : Laptop;

  return (
    <article className="summary-card">
      <div className="summary-icon">
        <Icon size={20} />
      </div>
      <div>
        <h2>{environment.id}</h2>
        <p>{environment.installed ? t("installed") : t("notDetected")}</p>
        <span>{environment.running ? t("runningState") : t("notRunning")} · {environment.accountHint}</span>
      </div>
      <StatusBadge status={environment.support} t={t} />
    </article>
  );
}

function StatusBadge({ status, t }: { status: EnvironmentState["support"]; t: Translate }) {
  if (status === "detected") {
    return <span className="status detected"><CheckCircle2 size={14} />{t("detected")}</span>;
  }
  if (status === "partial") {
    return <span className="status partial"><AlertTriangle size={14} />{t("partial")}</span>;
  }
  return <span className="status missing"><Clock3 size={14} />{t("pending")}</span>;
}

function SwitchDialog({
  profiles,
  settings,
  scan,
  t,
  onSwitched,
  onClose
}: {
  profiles: ProfileMetadata[];
  settings: AppSettings;
  scan: EnvironmentScan;
  t: Translate;
  onSwitched: () => Promise<void>;
  onClose: () => void;
}) {
  const defaultProfile = profiles.find((profile) => profile.defaultProfile) ?? profiles[0];
  const [profileId, setProfileId] = useState(defaultProfile?.id ?? "");
  const [scope, setScope] = useState<Record<TargetEnvironment, boolean>>({
    cli: settings.defaultScope.includes("cli"),
    vscode: settings.defaultScope.includes("vscode"),
    desktop: settings.defaultScope.includes("desktop")
  });
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState<ProfileSwitchResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [restartMessage, setRestartMessage] = useState<string | null>(null);
  const [rollbackProfileId, setRollbackProfileId] = useState<string | null>(null);
  const [confirmProcessClose, setConfirmProcessClose] = useState(!settings.confirmBeforeClosingApps);
  const targetProfile = profiles.find((profile) => profile.id === profileId);
  const rollbackProfile = rollbackProfileId
    ? profiles.find((profile) => profile.id === rollbackProfileId)
    : undefined;
  const rollbackEnvironments = result && rollbackProfile
    ? result.switchedEnvironments.filter((environment) => profileSupportsEnvironment(rollbackProfile, environment))
    : [];
  const supportedEnvironments = (["cli", "vscode", "desktop"] as TargetEnvironment[])
    .filter((environment) => profileSupportsEnvironment(targetProfile, environment));
  const selectedEnvironments = (Object.entries(scope) as Array<[TargetEnvironment, boolean]>)
    .filter(([environment, enabled]) => enabled && supportedEnvironments.includes(environment))
    .map(([environment]) => environment);
  const closeApprovalRequired = settings.confirmBeforeClosingApps && (scope.vscode || scope.desktop);
  const steps = switchSteps(result, busy, settings, t);

  useEffect(() => {
    setScope((current) => {
      const next = { ...current };
      for (const environment of ["cli", "vscode", "desktop"] as TargetEnvironment[]) {
        if (!profileSupportsEnvironment(targetProfile, environment)) {
          next[environment] = false;
        }
      }
      if (!next.cli && !next.vscode && !next.desktop) {
        for (const environment of ["cli", "vscode", "desktop"] as TargetEnvironment[]) {
          if (settings.defaultScope.includes(environment) && profileSupportsEnvironment(targetProfile, environment)) {
            next[environment] = true;
          }
        }
      }
      return next;
    });
  }, [settings.defaultScope, targetProfile]);

  async function startSwitch() {
    setBusy(true);
    setError(null);
    setResult(null);
    setRestartMessage(null);
    setRollbackProfileId(previousUsableProfile(profiles, profileId)?.id ?? null);
    try {
      const response = await switchToProfile({
        profileId,
        environments: selectedEnvironments,
        autoRestartApps: settings.autoRestartApps,
        vscodeReloadMode: settings.vscodeReloadMode,
        confirmProcessClose,
        desktopAppPath: scan.environments.find((environment) => environment.id === "Desktop")?.executablePath ?? null,
        vscodeAppPath: scan.environments.find((environment) => environment.id === "VS Code")?.executablePath ?? null,
        quitTimeoutMs: 8000
      });
      setResult(response);
      await onSwitched();
    } catch (switchError) {
      setError(String(switchError));
    } finally {
      setBusy(false);
    }
  }

  async function rollbackToPreviousProfile() {
    if (!rollbackProfile || rollbackEnvironments.length === 0 || !result) {
      setError("Rollback unavailable: no previous Profile has captured state for the switched environments.");
      return;
    }
    if (
      settings.confirmBeforeClosingApps
      && rollbackEnvironments.some((environment) => environment === "desktop" || environment === "vscode")
      && !confirmProcessClose
    ) {
      setError("Rollback blocked: approve closing running Desktop / VS Code windows first.");
      return;
    }
    const currentProfileId = result.profile.id;
    setBusy(true);
    setError(null);
    setRestartMessage(null);
    try {
      const response = await switchToProfile({
        profileId: rollbackProfile.id,
        environments: rollbackEnvironments,
        autoRestartApps: settings.autoRestartApps,
        vscodeReloadMode: settings.vscodeReloadMode,
        confirmProcessClose,
        desktopAppPath: scan.environments.find((environment) => environment.id === "Desktop")?.executablePath ?? null,
        vscodeAppPath: scan.environments.find((environment) => environment.id === "VS Code")?.executablePath ?? null,
        quitTimeoutMs: 8000
      });
      setProfileId(response.profile.id);
      setRollbackProfileId(currentProfileId);
      setResult(response);
      await onSwitched();
    } catch (rollbackError) {
      setError(`Rollback failed: ${String(rollbackError)}`);
    } finally {
      setBusy(false);
    }
  }

  async function retryRestart(target: "desktop" | "vscode") {
    setRestartMessage(`Restarting ${target === "desktop" ? "Codex Desktop" : "VS Code"}.`);
    try {
      const response = target === "desktop"
        ? await restartDesktopApp(scan.environments.find((environment) => environment.id === "Desktop")?.executablePath ?? null)
        : await restartVscodeApp(scan.environments.find((environment) => environment.id === "VS Code")?.executablePath ?? null);
      setRestartMessage(response.message);
      await onSwitched();
    } catch (restartError) {
      setRestartMessage(`Restart failed: ${String(restartError)}`);
    }
  }

  return (
    <div className="dialog-backdrop" role="presentation">
      <section className="dialog" role="dialog" aria-modal="true" aria-label={t("switchProgress")}>
        <header>
          <div>
            <p className="eyebrow">{t("switchTransaction")}</p>
            <h2>{t("prepareProfileSwitch")}</h2>
          </div>
          <button className="icon-button" onClick={onClose} aria-label={t("close")}>x</button>
        </header>

        <label className="dialog-field">
          <span>{t("targetProfile")}</span>
          <select
            value={profileId}
            onChange={(event) => {
              setProfileId(event.target.value);
              setResult(null);
              setError(null);
            }}
          >
            {profiles.map((profile) => (
              <option value={profile.id} key={profile.id}>{profile.name}</option>
            ))}
          </select>
        </label>

        <div className="scope-picker">
          {(["cli", "vscode", "desktop"] as TargetEnvironment[]).map((item) => {
            const state = environmentProfileState(targetProfile, item);
            const available = state?.status === "available";
            return (
            <label className={!available ? "disabled" : undefined} key={item}>
              <input
                type="checkbox"
                checked={scope[item]}
                disabled={!available}
                onChange={(event) => setScope((current) => ({ ...current, [item]: event.target.checked }))}
              />
              <span>{environmentLabel(item)}</span>
              <em>{available ? t("available") : state?.completenessReason ?? t("notImported")}</em>
            </label>
            );
          })}
        </div>

        {selectedEnvironments.length === 0 && (
          <p className="dialog-error">{t("selectCapturedEnvironment")}</p>
        )}

        {closeApprovalRequired && (
          <label className="dialog-confirm">
            <input
              type="checkbox"
              checked={confirmProcessClose}
              onChange={(event) => setConfirmProcessClose(event.target.checked)}
            />
            <span>{t("closeApproval")}</span>
          </label>
        )}

        <ol className="step-list">
          {steps.map((step) => (
            <li className={step.status} key={step.label}>
              <ListChecks size={16} />
              <span>{step.label}</span>
              <em>{step.detail}</em>
            </li>
          ))}
        </ol>

        {error && <p className="dialog-error">{error}</p>}
        {restartMessage && <p className="dialog-info">{restartMessage}</p>}
        {result && (
          <ul className="dialog-result">
            <li>{t("transactionSummary", { id: result.transaction.id, phase: result.transaction.phase })}</li>
            <li>{t("identitySummary", { status: result.identityVerification.status, message: result.identityVerification.message })}</li>
            {result.closedProcesses.length > 0 && <li>{t("closedSummary", { value: result.closedProcesses.join(", ") })}</li>}
            {result.restartedApps.length > 0 && <li>{t("restartedSummary", { value: result.restartedApps.join(", ") })}</li>}
            {result.warnings.map((item) => <li key={`warning-${item}`}>{item}</li>)}
            {result.manualActions.map((item) => <li key={`action-${item}`}>{item}</li>)}
            {result.transaction.events.map((event, index) => (
              <li key={`${event.phase}-${index}`}>{`${event.phase}: ${event.message}`}</li>
            ))}
          </ul>
        )}

        {result && (
          <div className="dialog-restart-actions">
            {rollbackProfile && rollbackEnvironments.length > 0 && (
              <button className="secondary-button compact" onClick={() => void rollbackToPreviousProfile()} disabled={busy}>
                <RotateCcw size={14} />
                {t("rollbackToProfile", { name: rollbackProfile.name })}
              </button>
            )}
            {result.switchedEnvironments.includes("desktop") && (
              <button className="secondary-button compact" onClick={() => void retryRestart("desktop")}>
                <RefreshCw size={14} />
                {t("restartDesktop")}
              </button>
            )}
            {result.switchedEnvironments.includes("vscode") && (
              <button className="secondary-button compact" onClick={() => void retryRestart("vscode")}>
                <RefreshCw size={14} />
                {t("restartVscode")}
              </button>
            )}
          </div>
        )}

        <footer>
          <button className="secondary-button" onClick={onClose}>{t("cancel")}</button>
          <button className="primary-button" onClick={startSwitch} disabled={busy || !profileId || selectedEnvironments.length === 0 || (closeApprovalRequired && !confirmProcessClose)}>
            <RefreshCw size={18} />
            {busy ? t("switching") : t("startSwitch")}
          </button>
        </footer>
      </section>
    </div>
  );
}

type SwitchStep = {
  label: string;
  status: "waiting" | "running" | "done" | "skipped" | "warning" | "failed";
  detail: string;
};

function switchSteps(result: ProfileSwitchResult | null, busy: boolean, settings: AppSettings, t: Translate): SwitchStep[] {
  const hasPhase = (phase: string) => Boolean(result?.transaction.events.some((event) => event.phase === phase));
  const terminalPhase = result?.transaction.phase;
  const failed = terminalPhase === "failed";
  const rolledBack = terminalPhase === "rolled_back";

  if (!result) {
    return [
      { label: t("checkingProfile"), status: busy ? "running" : "waiting", detail: busy ? t("running") : t("waiting") },
      { label: t("closingApps"), status: "waiting", detail: t("waiting") },
      { label: t("backingUp"), status: "waiting", detail: t("waiting") },
      { label: t("restoringProfile"), status: "waiting", detail: t("waiting") },
      { label: t("restartingApps"), status: "waiting", detail: t("waiting") },
      { label: t("verifyingAccount"), status: "waiting", detail: t("waiting") },
      { label: t("recordingHistory"), status: "waiting", detail: t("waiting") }
    ];
  }

  const identityStatus = result.identityVerification.status;
  return [
    { label: t("checkingProfile"), status: "done", detail: t("done") },
    {
      label: t("closingApps"),
      status: result.closedProcesses.length > 0 ? "done" : "skipped",
      detail: result.closedProcesses.length > 0 ? result.closedProcesses.join(", ") : t("noRunningGuiClosed")
    },
    {
      label: t("backingUp"),
      status: hasPhase("backup_complete") ? "done" : failed || rolledBack ? "failed" : "waiting",
      detail: hasPhase("backup_complete") ? t("backupComplete") : t("notCompleted")
    },
    {
      label: t("restoringProfile"),
      status: hasPhase("restore_complete") ? "done" : failed || rolledBack ? "failed" : "waiting",
      detail: hasPhase("restore_complete") ? t("restoreComplete") : t("notCompleted")
    },
    {
      label: t("restartingApps"),
      status: result.restartedApps.length > 0 ? "done" : settings.autoRestartApps ? "skipped" : "skipped",
      detail: result.restartedApps.length > 0 ? result.restartedApps.join(", ") : t("noRestartPerformed")
    },
    {
      label: t("verifyingAccount"),
      status: identityStatus === "verified" ? "done" : identityStatus === "mismatch" ? "failed" : "warning",
      detail: identityStatus
    },
    {
      label: t("recordingHistory"),
      status: terminalPhase === "completed" ? "done" : rolledBack ? "warning" : "failed",
      detail: terminalPhase ?? t("unknown")
    }
  ];
}
