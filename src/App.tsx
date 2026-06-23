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
  RefreshCw,
  RotateCcw,
  Settings,
  ShieldCheck,
  SquareTerminal,
  UserPlus
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import {
  detectEnvironments,
  emptyEnvironmentScan,
  EnvironmentId,
  EnvironmentScan,
  EnvironmentState,
  importCurrentProfile,
  listProfiles,
  ProfileMetadata,
  TargetEnvironment
} from "./backend";

type Tab = "home" | "profiles" | "environment" | "settings";

const recentHistory = [
  {
    time: "No switch history",
    from: "-",
    to: "-",
    status: "Waiting for first verified transaction"
  }
];

export default function App() {
  const [tab, setTab] = useState<Tab>("home");
  const [switchOpen, setSwitchOpen] = useState(false);
  const [scan, setScan] = useState<EnvironmentScan>(emptyEnvironmentScan);
  const [scanBusy, setScanBusy] = useState(false);
  const [profiles, setProfiles] = useState<ProfileMetadata[]>([]);
  const [scope, setScope] = useState<Record<EnvironmentId, boolean>>({
    CLI: true,
    "VS Code": true,
    Desktop: true
  });

  const activeProfile = useMemo(() => profiles.find((profile) => profile.defaultProfile), [profiles]);

  useEffect(() => {
    void runScan();
    void refreshProfiles();
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
        {tab === "home" && <Home activeProfile={activeProfile} scan={scan} onSwitch={() => setSwitchOpen(true)} />}
        {tab === "profiles" && <Profiles profiles={profiles} onImported={refreshProfiles} />}
        {tab === "environment" && <Environment scan={scan} busy={scanBusy} onScan={runScan} />}
        {tab === "settings" && <SettingsView />}
      </main>

      {switchOpen && (
        <SwitchDialog
          scope={scope}
          setScope={setScope}
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

function Home({ activeProfile, scan, onSwitch }: { activeProfile?: ProfileMetadata; scan: EnvironmentScan; onSwitch: () => void }) {
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

      <section className="panel">
        <div className="panel-title">
          <History size={18} />
          <h2>Recent switch history</h2>
        </div>
        <div className="history-list">
          {recentHistory.map((item) => (
            <div className="history-row" key={item.status}>
              <span>{item.time}</span>
              <strong>{item.from} {"->"} {item.to}</strong>
              <em>{item.status}</em>
            </div>
          ))}
        </div>
      </section>
    </section>
  );
}

function Profiles({ profiles, onImported }: { profiles: ProfileMetadata[]; onImported: () => Promise<void> }) {
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
      await onImported();
      const importedCount = result.importedEnvironments
        .reduce((sum, item) => sum + item.artifactCount, 0);
      setMessage(`Saved ${result.profile.name}; captured ${importedCount} local artifacts into the secret vault.`);
    } catch (error) {
      setMessage(`Import failed: ${String(error)}`);
    } finally {
      setImporting(false);
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
        {profiles.map((profile) => (
          <article className="profile-card" key={profile.id}>
            <div className="profile-topline">
              <div>
                <h2>{profile.name}</h2>
                <span>{profile.accountHint}</span>
              </div>
              {profile.defaultProfile && <strong className="pill">Default</strong>}
            </div>
            <p>{profile.note}</p>
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
              <button>Edit</button>
              <button>Rename</button>
              <button>Delete</button>
            </div>
          </article>
        ))}
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

function SettingsView() {
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
        <label className="setting-row">
          <span>Default switch scope</span>
          <select defaultValue="all">
            <option value="all">All supported environments</option>
            <option value="cli">CLI only</option>
            <option value="vscode">VS Code only</option>
            <option value="desktop">Desktop only</option>
          </select>
        </label>
        <label className="setting-row">
          <span>Confirm before closing apps</span>
          <input type="checkbox" defaultChecked />
        </label>
        <label className="setting-row">
          <span>Auto-restart supported apps</span>
          <input type="checkbox" defaultChecked />
        </label>
        <label className="setting-row">
          <span>Restore default account on exit</span>
          <input type="checkbox" />
        </label>
        <button className="danger-button">Clear local history</button>
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
  scope,
  setScope,
  onClose
}: {
  scope: Record<EnvironmentId, boolean>;
  setScope: React.Dispatch<React.SetStateAction<Record<EnvironmentId, boolean>>>;
  onClose: () => void;
}) {
  const steps = ["Closing processes", "Backing up", "Restoring profile", "Restarting", "Verifying"];

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

        <div className="scope-picker">
          {(["CLI", "VS Code", "Desktop"] as EnvironmentId[]).map((item) => (
            <label key={item}>
              <input
                type="checkbox"
                checked={scope[item]}
                onChange={(event) => setScope((current) => ({ ...current, [item]: event.target.checked }))}
              />
              <span>{item}</span>
            </label>
          ))}
        </div>

        <ol className="step-list">
          {steps.map((step) => (
            <li key={step}>
              <ListChecks size={16} />
              <span>{step}</span>
              <em>Waiting</em>
            </li>
          ))}
        </ol>

        <footer>
          <button className="secondary-button" onClick={onClose}>Cancel</button>
          <button className="primary-button">
            <RefreshCw size={18} />
            Start simulated flow
          </button>
        </footer>
      </section>
    </div>
  );
}
