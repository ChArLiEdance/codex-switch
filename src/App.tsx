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
import { useEffect, useMemo, useState } from "react";
import {
  AppSettings,
  checkRecoveryStatus,
  clearSwitchHistory,
  deleteProfile,
  detectEnvironments,
  emptyEnvironmentScan,
  EnvironmentId,
  EnvironmentScan,
  EnvironmentState,
  getSettings,
  importCurrentProfile,
  listProfiles,
  listSwitchHistory,
  ProfileMetadata,
  RecoveryStatus,
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
  vscodeReloadMode: "manual_reload_window"
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

  const activeProfile = useMemo(() => profiles.find((profile) => profile.defaultProfile), [profiles]);

  useEffect(() => {
    void runScan();
    void refreshProfiles();
    void refreshHistory();
    void refreshSettings();
    void refreshRecovery();
  }, []);

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

  async function updateSettings(next: AppSettings) {
    setSettings(await saveSettings(next));
  }

  async function clearHistory() {
    await clearSwitchHistory();
    await refreshHistory();
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
            activeProfile={activeProfile}
            scan={scan}
            history={history}
            recovery={recovery}
            onSwitch={() => setSwitchOpen(true)}
          />
        )}
        {tab === "profiles" && <Profiles profiles={profiles} onProfilesChanged={refreshProfiles} />}
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
  activeProfile,
  scan,
  history,
  recovery,
  onSwitch
}: {
  activeProfile?: ProfileMetadata;
  scan: EnvironmentScan;
  history: SwitchHistoryEntry[];
  recovery: RecoveryStatus;
  onSwitch: () => void;
}) {
  return (
    <section className="view">
      <header className="view-header">
        <div>
          <p className="eyebrow">Current account</p>
          <h1>{activeProfile?.accountHint ?? "No verified account"}</h1>
        </div>
        <button className="primary-button" onClick={onSwitch}>
          <RefreshCw size={18} />
          One-click switch
        </button>
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

function Profiles({ profiles, onProfilesChanged }: { profiles: ProfileMetadata[]; onProfilesChanged: () => Promise<void> }) {
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

  async function saveCurrentLogin() {
    setImporting(true);
    setMessage(null);
    try {
      const environments = (Object.entries(selected) as Array<[TargetEnvironment, boolean]>)
        .filter(([, enabled]) => enabled)
        .map(([environment]) => environment);
      const result = await importCurrentProfile({
        name,
        tags: tags.split(",").map((tag) => tag.trim()).filter(Boolean),
        note,
        environments,
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

function Environment({ scan, busy, onScan }: { scan: EnvironmentScan; busy: boolean; onScan: () => void }) {
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

      <div className="environment-list">
        {scan.environments.map((environment) => (
          <article className="environment-row" key={environment.id}>
            <EnvironmentSummary environment={environment} />
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
  function toggleScope(environment: TargetEnvironment, enabled: boolean) {
    const nextScope = enabled
      ? Array.from(new Set([...settings.defaultScope, environment]))
      : settings.defaultScope.filter((item) => item !== environment);
    void onChange({ ...settings, defaultScope: nextScope });
  }

  return (
    <section className="view">
      <header className="view-header">
        <div>
          <p className="eyebrow">Switch policy</p>
          <h1>Settings</h1>
        </div>
        <button className="secondary-button">
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
  const [result, setResult] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [confirmProcessClose, setConfirmProcessClose] = useState(!settings.confirmBeforeClosingApps);
  const closeApprovalRequired = settings.confirmBeforeClosingApps && (scope.vscode || scope.desktop);
  const steps = ["Checking profile", "Closing apps", "Backing up", "Restoring profile", "Restarting apps", "Recording history"];

  async function startSwitch() {
    const environments = (Object.entries(scope) as Array<[TargetEnvironment, boolean]>)
      .filter(([, enabled]) => enabled)
      .map(([environment]) => environment);
    setBusy(true);
    setError(null);
    setResult([]);
    try {
      const response = await switchToProfile({
        profileId,
        environments,
        autoRestartApps: settings.autoRestartApps,
        vscodeReloadMode: settings.vscodeReloadMode,
        confirmProcessClose,
        desktopAppPath: scan.environments.find((environment) => environment.id === "Desktop")?.executablePath ?? null,
        vscodeAppPath: scan.environments.find((environment) => environment.id === "VS Code")?.executablePath ?? null,
        quitTimeoutMs: 8000
      });
      setResult([
        `Transaction ${response.transaction.id}: ${response.transaction.phase}`,
        response.closedProcesses.length > 0 ? `Closed: ${response.closedProcesses.join(", ")}` : "No running GUI apps closed",
        response.restartedApps.length > 0 ? `Restarted: ${response.restartedApps.join(", ")}` : "No app restart performed",
        ...response.warnings,
        ...response.manualActions
      ]);
      await onSwitched();
    } catch (switchError) {
      setError(String(switchError));
    } finally {
      setBusy(false);
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
          <select value={profileId} onChange={(event) => setProfileId(event.target.value)}>
            {profiles.map((profile) => (
              <option value={profile.id} key={profile.id}>{profile.name}</option>
            ))}
          </select>
        </label>

        <div className="scope-picker">
          {(["cli", "vscode", "desktop"] as TargetEnvironment[]).map((item) => (
            <label key={item}>
              <input
                type="checkbox"
                checked={scope[item]}
                onChange={(event) => setScope((current) => ({ ...current, [item]: event.target.checked }))}
              />
              <span>{environmentLabel(item)}</span>
            </label>
          ))}
        </div>

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
            <li key={step}>
              <ListChecks size={16} />
              <span>{step}</span>
              <em>Waiting</em>
            </li>
          ))}
        </ol>

        {error && <p className="dialog-error">{error}</p>}
        {result.length > 0 && (
          <ul className="dialog-result">
            {result.map((item) => <li key={item}>{item}</li>)}
          </ul>
        )}

        <footer>
          <button className="secondary-button" onClick={onClose}>Cancel</button>
          <button className="primary-button" onClick={startSwitch} disabled={busy || !profileId || (closeApprovalRequired && !confirmProcessClose)}>
            <RefreshCw size={18} />
            {busy ? "Switching" : "Start switch"}
          </button>
        </footer>
      </section>
    </div>
  );
}
