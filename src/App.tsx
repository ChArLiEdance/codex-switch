import {
  AlertTriangle,
  CheckCircle2,
  Clock3,
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
  EnvironmentId,
  EnvironmentPathKind,
  EnvironmentScan,
  EnvironmentState,
  getSettings,
  importCurrentProfile,
  listProfiles,
  listSwitchHistory,
  ProfileMetadata,
  ProfileSwitchResult,
  RecoveryStatus,
  resolveRecoveryStatus,
  restoreDefaultOnExit,
  restartDesktopApp,
  restartVscodeApp,
  saveSettings,
  SwitchHistoryEntry,
  switchToProfile,
  TargetEnvironment,
  updateProfile
} from "./backend";

type Tab = "home" | "profiles" | "environment" | "settings";

const defaultSettings: AppSettings = {
  defaultScope: ["cli", "vscode", "desktop"],
  confirmBeforeClosingApps: true,
  autoRestartApps: true,
  restoreDefaultOnExit: false,
  vscodeReloadMode: "manual_reload_window",
  customPaths: []
};

const defaultRecoveryStatus: RecoveryStatus = {
  needsRecovery: false,
  transactionId: null,
  phase: null,
  message: "Recovery check has not run"
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
  const [quickSwitchMessage, setQuickSwitchMessage] = useState<string | null>(null);
  const closeAfterExitRestore = useRef(false);

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
      item.toProfile === currentProfile.name && item.switchedAt === currentProfile.lastUsedAt
    ) ?? history.find((item) => item.toProfile === currentProfile.name);
  }, [currentProfile, history]);
  const previousProfile = useMemo(() => {
    const previousName = history.find((item) => item.fromProfile)?.fromProfile;
    return previousName ? profiles.find((profile) => profile.name === previousName) : undefined;
  }, [history, profiles]);

  useEffect(() => {
    void runScan();
    void refreshProfiles();
    void refreshHistory();
    void refreshSettings();
    void refreshRecovery();
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

  async function resolveRecovery() {
    const nextRecovery = await resolveRecoveryStatus();
    setRecovery(nextRecovery);
    setQuickSwitchMessage("Recovery journal marked reviewed. Use Restore default or Switch back if the local account state still needs correction.");
  }

  async function updateSettings(next: AppSettings) {
    setSettings(await saveSettings(next));
  }

  async function clearHistory() {
    await clearSwitchHistory();
    await refreshHistory();
  }

  async function quickSwitchProfile(profile: ProfileMetadata, label: string) {
    const environments = settings.defaultScope.filter((environment) =>
      profile.environments.some((state) => state.environment === environment && state.status === "available")
    );
    if (environments.length === 0) {
      setQuickSwitchMessage(`${profile.name} has no available environments in the default switch scope.`);
      return;
    }
    const needsCloseApproval = settings.confirmBeforeClosingApps && environments.some((environment) => environment === "vscode" || environment === "desktop");
    if (needsCloseApproval && !window.confirm(`Close running Desktop / VS Code windows if needed, then ${label}?`)) {
      return;
    }
    setQuickSwitchMessage(`${label} started for ${profile.name}.`);
    try {
      const response = await switchToProfile({
        profileId: profile.id,
        environments,
        autoRestartApps: settings.autoRestartApps,
        vscodeReloadMode: settings.vscodeReloadMode,
        confirmProcessClose: !settings.confirmBeforeClosingApps || needsCloseApproval,
        desktopAppPath: scan.environments.find((environment) => environment.id === "Desktop")?.executablePath ?? null,
        vscodeAppPath: scan.environments.find((environment) => environment.id === "VS Code")?.executablePath ?? null,
        quitTimeoutMs: 8000
      });
      await refreshProfiles();
      await refreshHistory();
      await refreshRecovery();
      await runScan();
      setQuickSwitchMessage(`${label} finished: ${response.transaction.phase}.`);
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
            <span>Local profile manager</span>
          </div>
        </div>
        <nav className="nav-list" aria-label="Primary">
          <NavButton icon={<Laptop size={18} />} label="Home" active={tab === "home"} onClick={() => setTab("home")} />
          <NavButton icon={<KeyRound size={18} />} label="Profiles" active={tab === "profiles"} onClick={() => setTab("profiles")} />
          <NavButton icon={<FolderSearch size={18} />} label="Environment" active={tab === "environment"} onClick={() => setTab("environment")} />
          <NavButton icon={<Settings size={18} />} label="Settings" active={tab === "settings"} onClick={() => setTab("settings")} />
        </nav>
      </aside>

      <main className="content">
        {tab === "home" && (
          <Home
            currentProfile={currentProfile}
            currentSwitchHistory={currentSwitchHistory}
            defaultProfile={defaultProfile}
            previousProfile={previousProfile}
            scan={scan}
            history={history}
            recovery={recovery}
            quickSwitchMessage={quickSwitchMessage}
            onSwitch={() => setSwitchOpen(true)}
            onResolveRecovery={() => void resolveRecovery()}
            onRestoreDefault={() => {
              if (defaultProfile) {
                void quickSwitchProfile(defaultProfile, "Restore default account");
              }
            }}
            onSwitchPrevious={() => {
              if (previousProfile) {
                void quickSwitchProfile(previousProfile, "Switch back to previous account");
              }
            }}
          />
        )}
        {tab === "profiles" && (
          <Profiles
            profiles={profiles}
            scan={scan}
            scanBusy={scanBusy}
            onScan={runScan}
            onProfilesChanged={refreshProfiles}
          />
        )}
        {tab === "environment" && <Environment scan={scan} busy={scanBusy} onScan={runScan} />}
        {tab === "settings" && (
          <SettingsView
            settings={settings}
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
  previousProfile,
  scan,
  history,
  recovery,
  quickSwitchMessage,
  onSwitch,
  onResolveRecovery,
  onRestoreDefault,
  onSwitchPrevious
}: {
  currentProfile?: ProfileMetadata;
  currentSwitchHistory?: SwitchHistoryEntry;
  defaultProfile?: ProfileMetadata;
  previousProfile?: ProfileMetadata;
  scan: EnvironmentScan;
  history: SwitchHistoryEntry[];
  recovery: RecoveryStatus;
  quickSwitchMessage: string | null;
  onSwitch: () => void;
  onResolveRecovery: () => void;
  onRestoreDefault: () => void;
  onSwitchPrevious: () => void;
}) {
  return (
    <section className="view">
      <header className="view-header">
        <div>
          <p className="eyebrow">Current account</p>
          <h1>{currentProfile?.accountHint ?? "No verified account"}</h1>
          <span className="scan-meta">{currentProfile ? `${currentProfile.name}${currentProfile.lastUsedAt ? ` · last used ${currentProfile.lastUsedAt}` : ""}` : "No profile has been switched yet"}</span>
          {currentProfile && (
            <span className={`account-verification ${verificationClass(currentSwitchHistory)}`}>
              {verificationLabel(currentSwitchHistory)}
            </span>
          )}
        </div>
        <div className="header-actions">
          <button className="primary-button" onClick={onSwitch}>
            <RefreshCw size={18} />
            One-click switch
          </button>
          <button className="secondary-button" onClick={onRestoreDefault} disabled={!defaultProfile}>
            <RotateCcw size={18} />
            Restore default
          </button>
          <button className="secondary-button" onClick={onSwitchPrevious} disabled={!previousProfile}>
            <History size={18} />
            Switch back
          </button>
        </div>
      </header>

      <div className="status-grid">
        {scan.environments.map((environment) => (
          <EnvironmentSummary key={environment.id} environment={environment} />
        ))}
      </div>

      {recovery.needsRecovery && (
        <section className="recovery-banner">
          <AlertTriangle size={18} />
          <span>{recovery.message}</span>
          <button className="secondary-button compact" onClick={onResolveRecovery}>Mark reviewed</button>
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
          <h2>Recent switch history</h2>
        </div>
        <div className="history-list">
          {history.length === 0 && (
            <div className="history-row">
              <span>No switch history</span>
              <strong>- {"->"} -</strong>
              <em>Waiting for first verified transaction</em>
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
  onScan,
  onProfilesChanged
}: {
  profiles: ProfileMetadata[];
  scan: EnvironmentScan;
  scanBusy: boolean;
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
          <p className="eyebrow">Profile management</p>
          <h1>Authorized local profiles</h1>
        </div>
        <button className="secondary-button" onClick={saveCurrentLogin} disabled={importing}>
          <UserPlus size={18} />
          {importing ? "Saving" : "Save current login"}
        </button>
      </header>

      <section className="import-panel">
        <div className="import-guide">
          <div>
            <p className="eyebrow">New account workflow</p>
            <h2>Use official login first, then capture local state</h2>
          </div>
          <ol>
            <li>Sign in through Codex CLI, VS Code, or Codex Desktop using their official login flows.</li>
            <li>Return here and rescan so this app can inspect local state read-only.</li>
            <li>Select only environments that belong to the same account, then save the Profile.</li>
          </ol>
          <button className="secondary-button compact" onClick={() => void onScan()} disabled={scanBusy}>
            <FolderSearch size={14} />
            {scanBusy ? "Scanning" : "Rescan current state"}
          </button>
        </div>
        <div className="import-detected-grid">
          {(["cli", "vscode", "desktop"] as TargetEnvironment[]).map((environment) => {
            const detected = scanEnvironmentForTarget(scan, environment);
            const existingPaths = detected?.discoveredPaths.filter((path) => path.exists).length ?? 0;
            return (
              <article key={environment}>
                <strong>{environmentLabel(environment)}</strong>
                <span>{detected?.support ?? "not-detected"} - {detected?.accountHint ?? "Unknown"}</span>
                <em>{existingPaths} existing candidate paths</em>
              </article>
            );
          })}
        </div>
        <div className="import-fields">
          <label>
            <span>Profile name</span>
            <input value={name} onChange={(event) => setName(event.target.value)} />
          </label>
          <label>
            <span>Tags</span>
            <input value={tags} onChange={(event) => setTags(event.target.value)} />
          </label>
          <label>
            <span>Note</span>
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
              <em>{scanEnvironmentForTarget(scan, environment)?.accountHint ?? "Unknown"}</em>
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
            <span>Selected environments belong to the same authorized account</span>
          </label>
          <label>
            <input
              type="checkbox"
              checked={makeDefault}
              onChange={(event) => setMakeDefault(event.target.checked)}
            />
            <span>Set as default profile</span>
          </label>
        </div>
        {requiresSameAccountConfirmation && (
          <p className="import-warning">Multi-environment imports require same-account confirmation before saving.</p>
        )}
        {message && <p className="import-message">{message}</p>}
      </section>

      <div className="profile-grid">
        {profiles.length === 0 && (
          <article className="empty-state">
            <h2>No saved profiles</h2>
            <p>Run read-only detection, confirm the selected environments belong to the same account, then save the current login.</p>
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
                  <span>Name</span>
                  <input value={editName} onChange={(event) => setEditName(event.target.value)} />
                </label>
                <label>
                  <span>Tags</span>
                  <input value={editTags} onChange={(event) => setEditTags(event.target.value)} />
                </label>
                <label>
                  <span>Note</span>
                  <textarea value={editNote} onChange={(event) => setEditNote(event.target.value)} />
                </label>
              </div>
            ) : (
              <>
                <div className="profile-topline">
                  <div>
                    <h2>{profile.name}</h2>
                    <span>{profile.accountHint}</span>
                    <span>{profile.lastUsedAt ? `Last used ${profile.lastUsedAt}` : "Never switched"}</span>
                  </div>
                  {profile.defaultProfile && <strong className="pill">Default</strong>}
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
                    Save
                  </button>
                  <button onClick={() => setEditingProfileId(null)} disabled={busy}>
                    <X size={14} />
                    Cancel
                  </button>
                </>
              ) : (
                <>
                  <button onClick={() => startEdit(profile)} disabled={busy}>
                    <Pencil size={14} />
                    Edit
                  </button>
                  {!profile.defaultProfile && (
                    <button onClick={() => void setDefaultProfile(profile)} disabled={busy}>
                      <Star size={14} />
                      Set default
                    </button>
                  )}
                  <button onClick={() => void removeProfile(profile)} disabled={busy} className="danger-card-button">
                    <Trash2 size={14} />
                    Delete
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

function verificationLabel(history?: SwitchHistoryEntry) {
  if (!history) {
    return "No switch verification recorded";
  }
  if (history.status === "success") {
    return "Verified by post-switch account hint";
  }
  if (history.status === "incomplete") {
    return history.errorType === "IdentityMismatch"
      ? "Account hint mismatch after switch"
      : "Identity verification incomplete";
  }
  if (history.status === "rolled_back") {
    return "Last switch rolled back";
  }
  return "Last switch failed";
}

function Environment({ scan, busy, onScan }: { scan: EnvironmentScan; busy: boolean; onScan: () => void }) {
  const [restartMessage, setRestartMessage] = useState<string | null>(null);

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

  return (
    <section className="view">
      <header className="view-header">
        <div>
          <p className="eyebrow">Read-only detection</p>
          <h1>Environment status</h1>
          <span className="scan-meta">{scan.os} · {scan.scannedAt} · {scan.readOnly ? "read-only" : "write-enabled"}</span>
        </div>
        <button className="secondary-button" onClick={onScan} disabled={busy}>
          <FolderSearch size={18} />
          {busy ? "Scanning" : "Rescan"}
        </button>
      </header>

      {restartMessage && (
        <section className="recovery-banner">
          <RefreshCw size={18} />
          <span>{restartMessage}</span>
        </section>
      )}

      <div className="environment-list">
        {scan.environments.map((environment) => (
          <article className="environment-row" key={environment.id}>
            <div className="environment-row-heading">
              <EnvironmentSummary environment={environment} />
              {(environment.id === "Desktop" || environment.id === "VS Code") && (
                <button
                  className="secondary-button compact"
                  onClick={() => void restartEnvironment(environment)}
                  disabled={!environment.executablePath}
                >
                  <RefreshCw size={14} />
                  Restart
                </button>
              )}
            </div>
            <dl>
              <div>
                <dt>App path</dt>
                <dd>{environment.executablePath ?? "Not detected"}</dd>
              </div>
              <div>
                <dt>Running</dt>
                <dd>{environment.runningProcesses.length > 0 ? environment.runningProcesses.join(", ") : "No matching process"}</dd>
              </div>
              <div>
                <dt>Status</dt>
                <dd>{environment.statusMessage}</dd>
              </div>
              <div>
                <dt>Permission</dt>
                <dd>{environment.permission}</dd>
              </div>
              <div className="path-list">
                <dt>Discovered paths</dt>
                <dd>
                  {environment.discoveredPaths.length === 0 ? (
                    "No candidate paths found"
                  ) : (
                    <ul>
                      {environment.discoveredPaths.map((item) => (
                        <li key={`${item.kind}-${item.path}`}>
                          <strong>{item.kind}</strong>
                          <span>{item.path}</span>
                          <em>{item.exists ? item.permission : "missing"}</em>
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
  onChange,
  onClearHistory
}: {
  settings: AppSettings;
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
          <p className="eyebrow">Switch policy</p>
          <h1>Settings</h1>
        </div>
        <button className="secondary-button" onClick={() => void onChange(defaultSettings)}>
          <RotateCcw size={18} />
          Restore defaults
        </button>
      </header>

      <section className="settings-grid">
        <div className="setting-row stacked">
          <span>Default switch scope</span>
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
          <span>VS Code post-switch action</span>
          <select
            value={settings.vscodeReloadMode}
            onChange={(event) => void onChange({ ...settings, vscodeReloadMode: event.target.value as AppSettings["vscodeReloadMode"] })}
          >
            <option value="manual_reload_window">Manual Reload Window</option>
            <option value="restart_app">Restart VS Code</option>
            <option value="none">No reload</option>
          </select>
        </label>
        <label className="setting-row">
          <span>Confirm before closing apps</span>
          <input
            type="checkbox"
            checked={settings.confirmBeforeClosingApps}
            onChange={(event) => void onChange({ ...settings, confirmBeforeClosingApps: event.target.checked })}
          />
        </label>
        <label className="setting-row">
          <span>Auto-restart supported apps</span>
          <input
            type="checkbox"
            checked={settings.autoRestartApps}
            onChange={(event) => void onChange({ ...settings, autoRestartApps: event.target.checked })}
          />
        </label>
        <label className="setting-row">
          <span>Restore default account on exit</span>
          <input
            type="checkbox"
            checked={settings.restoreDefaultOnExit}
            onChange={(event) => void onChange({ ...settings, restoreDefaultOnExit: event.target.checked })}
          />
        </label>
        <div className="setting-row stacked path-overrides">
          <div className="setting-section-title">
            <span>Custom detector paths</span>
            <button className="secondary-button compact" onClick={addCustomPath}>
              <Plus size={14} />
              Add path
            </button>
          </div>
          <div className="path-override-list">
            {customPaths.length === 0 ? (
              <p>No custom paths configured.</p>
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
                  <option value="auth">Auth</option>
                  <option value="config">Config</option>
                  <option value="cache">Cache</option>
                  <option value="app">App</option>
                </select>
                <input
                  value={override.path}
                  onChange={(event) => updateCustomPath(index, "path", event.target.value)}
                  placeholder="~/Library/Application Support/..."
                />
                <button
                  className="secondary-button compact icon-only"
                  onClick={() => removeCustomPath(index)}
                  aria-label="Remove custom detector path"
                >
                  <Trash2 size={14} />
                </button>
              </div>
            ))}
          </div>
        </div>
        <button className="danger-button" onClick={() => void onClearHistory()}>Clear local history</button>
      </section>
    </section>
  );
}

function EnvironmentSummary({ environment }: { environment: EnvironmentState }) {
  const Icon = environment.id === "CLI" ? SquareTerminal : environment.id === "VS Code" ? Database : Laptop;

  return (
    <article className="summary-card">
      <div className="summary-icon">
        <Icon size={20} />
      </div>
      <div>
        <h2>{environment.id}</h2>
        <p>{environment.installed ? "Installed" : "Not detected"}</p>
        <span>{environment.running ? "Running" : "Not running"} · {environment.accountHint}</span>
      </div>
      <StatusBadge status={environment.support} />
    </article>
  );
}

function StatusBadge({ status }: { status: EnvironmentState["support"] }) {
  if (status === "detected") {
    return <span className="status detected"><CheckCircle2 size={14} />Detected</span>;
  }
  if (status === "partial") {
    return <span className="status partial"><AlertTriangle size={14} />Partial</span>;
  }
  return <span className="status missing"><Clock3 size={14} />Pending</span>;
}

function SwitchDialog({
  profiles,
  settings,
  scan,
  onSwitched,
  onClose
}: {
  profiles: ProfileMetadata[];
  settings: AppSettings;
  scan: EnvironmentScan;
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
  const steps = switchSteps(result, busy, settings);

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
      <section className="dialog" role="dialog" aria-modal="true" aria-label="Switch progress">
        <header>
          <div>
            <p className="eyebrow">Switch transaction</p>
            <h2>Prepare profile switch</h2>
          </div>
          <button className="icon-button" onClick={onClose} aria-label="Close">x</button>
        </header>

        <label className="dialog-field">
          <span>Target profile</span>
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
              <em>{available ? "Available" : state?.completenessReason ?? "Not imported"}</em>
            </label>
            );
          })}
        </div>

        {selectedEnvironments.length === 0 && (
          <p className="dialog-error">Select at least one environment that this Profile has captured.</p>
        )}

        {closeApprovalRequired && (
          <label className="dialog-confirm">
            <input
              type="checkbox"
              checked={confirmProcessClose}
              onChange={(event) => setConfirmProcessClose(event.target.checked)}
            />
            <span>I saved work and approve closing running Desktop / VS Code windows for this switch</span>
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
            <li>{`Transaction ${result.transaction.id}: ${result.transaction.phase}`}</li>
            <li>{`Identity ${result.identityVerification.status}: ${result.identityVerification.message}`}</li>
            {result.closedProcesses.length > 0 && <li>{`Closed: ${result.closedProcesses.join(", ")}`}</li>}
            {result.restartedApps.length > 0 && <li>{`Restarted: ${result.restartedApps.join(", ")}`}</li>}
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
                Roll back to {rollbackProfile.name}
              </button>
            )}
            {result.switchedEnvironments.includes("desktop") && (
              <button className="secondary-button compact" onClick={() => void retryRestart("desktop")}>
                <RefreshCw size={14} />
                Restart Desktop
              </button>
            )}
            {result.switchedEnvironments.includes("vscode") && (
              <button className="secondary-button compact" onClick={() => void retryRestart("vscode")}>
                <RefreshCw size={14} />
                Restart VS Code
              </button>
            )}
          </div>
        )}

        <footer>
          <button className="secondary-button" onClick={onClose}>Cancel</button>
          <button className="primary-button" onClick={startSwitch} disabled={busy || !profileId || selectedEnvironments.length === 0 || (closeApprovalRequired && !confirmProcessClose)}>
            <RefreshCw size={18} />
            {busy ? "Switching" : "Start switch"}
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

function switchSteps(result: ProfileSwitchResult | null, busy: boolean, settings: AppSettings): SwitchStep[] {
  const hasPhase = (phase: string) => Boolean(result?.transaction.events.some((event) => event.phase === phase));
  const terminalPhase = result?.transaction.phase;
  const failed = terminalPhase === "failed";
  const rolledBack = terminalPhase === "rolled_back";

  if (!result) {
    return [
      { label: "Checking profile", status: busy ? "running" : "waiting", detail: busy ? "Running" : "Waiting" },
      { label: "Closing apps", status: "waiting", detail: "Waiting" },
      { label: "Backing up", status: "waiting", detail: "Waiting" },
      { label: "Restoring profile", status: "waiting", detail: "Waiting" },
      { label: "Restarting apps", status: "waiting", detail: "Waiting" },
      { label: "Verifying account", status: "waiting", detail: "Waiting" },
      { label: "Recording history", status: "waiting", detail: "Waiting" }
    ];
  }

  const identityStatus = result.identityVerification.status;
  return [
    { label: "Checking profile", status: "done", detail: "Done" },
    {
      label: "Closing apps",
      status: result.closedProcesses.length > 0 ? "done" : "skipped",
      detail: result.closedProcesses.length > 0 ? result.closedProcesses.join(", ") : "No running GUI apps closed"
    },
    {
      label: "Backing up",
      status: hasPhase("backup_complete") ? "done" : failed || rolledBack ? "failed" : "waiting",
      detail: hasPhase("backup_complete") ? "Backup complete" : "Not completed"
    },
    {
      label: "Restoring profile",
      status: hasPhase("restore_complete") ? "done" : failed || rolledBack ? "failed" : "waiting",
      detail: hasPhase("restore_complete") ? "Restore complete" : "Not completed"
    },
    {
      label: "Restarting apps",
      status: result.restartedApps.length > 0 ? "done" : settings.autoRestartApps ? "skipped" : "skipped",
      detail: result.restartedApps.length > 0 ? result.restartedApps.join(", ") : "No restart performed"
    },
    {
      label: "Verifying account",
      status: identityStatus === "verified" ? "done" : identityStatus === "mismatch" ? "failed" : "warning",
      detail: identityStatus
    },
    {
      label: "Recording history",
      status: terminalPhase === "completed" ? "done" : rolledBack ? "warning" : "failed",
      detail: terminalPhase ?? "unknown"
    }
  ];
}
