import { invoke } from "@tauri-apps/api/core";

export type EnvironmentId = "CLI" | "VS Code" | "Desktop";
export type TargetEnvironment = "cli" | "vscode" | "desktop";

export type DiscoveredPath = {
  kind: "app" | "auth" | "config" | "cache" | "other";
  path: string;
  exists: boolean;
  permission: "read-write" | "read-only" | "missing" | "unknown";
};

export type EnvironmentState = {
  id: EnvironmentId;
  installed: boolean;
  executablePath: string | null;
  discoveredPaths: DiscoveredPath[];
  running: boolean;
  runningProcesses: string[];
  permission: "read-write" | "read-only" | "missing" | "unknown";
  accountHint: string;
  support: "detected" | "partial" | "not-detected";
  statusMessage: string;
};

export type EnvironmentScan = {
  os: string;
  scannedAt: string;
  readOnly: boolean;
  environments: EnvironmentState[];
};

export type ProfileAuthStatus = "available" | "possibly_expired" | "expired" | "not_detected";

export type EnvironmentProfileState = {
  environment: TargetEnvironment;
  status: ProfileAuthStatus;
  secretRef: string | null;
  completenessReason: string;
  capturedAt: string | null;
};

export type ProfileMetadata = {
  id: string;
  name: string;
  accountHint: string;
  tags: string[];
  note: string;
  defaultProfile: boolean;
  lastUsedAt: string | null;
  environments: EnvironmentProfileState[];
};

export type ProfileImportRequest = {
  name: string;
  tags: string[];
  note: string;
  environments: TargetEnvironment[];
  confirmSameAccount: boolean;
  defaultProfile: boolean;
};

export type ImportedEnvironmentSummary = {
  environment: TargetEnvironment;
  artifactCount: number;
  capturedBytes: number;
  skippedCount: number;
  secretRef: string | null;
};

export type ProfileImportResult = {
  profile: ProfileMetadata;
  importedEnvironments: ImportedEnvironmentSummary[];
  warnings: string[];
};

export type AppSettings = {
  defaultScope: TargetEnvironment[];
  confirmBeforeClosingApps: boolean;
  autoRestartApps: boolean;
  restoreDefaultOnExit: boolean;
  vscodeReloadMode: "manual_reload_window" | "restart_app" | "none";
};

export type SwitchHistoryEntry = {
  id: string;
  switchedAt: string;
  fromProfile: string | null;
  toProfile: string;
  environments: TargetEnvironment[];
  status: "success" | "failed" | "rolled_back" | "incomplete";
  errorType: string | null;
};

export type RecoveryStatus = {
  needsRecovery: boolean;
  transactionId: string | null;
  phase: string | null;
  message: string;
};

export const emptyEnvironmentScan: EnvironmentScan = {
  os: "unknown",
  scannedAt: "Not scanned",
  readOnly: true,
  environments: [
    emptyEnvironment("CLI"),
    emptyEnvironment("VS Code"),
    emptyEnvironment("Desktop")
  ]
};

export async function detectEnvironments(): Promise<EnvironmentScan> {
  try {
    return await invoke<EnvironmentScan>("detect_environments");
  } catch (error) {
    return {
      ...emptyEnvironmentScan,
      scannedAt: new Date().toISOString(),
      environments: emptyEnvironmentScan.environments.map((environment) => ({
        ...environment,
        statusMessage: `Backend unavailable in this runtime: ${String(error)}`
      }))
    };
  }
}

export async function listProfiles(): Promise<ProfileMetadata[]> {
  try {
    return await invoke<ProfileMetadata[]>("list_profiles");
  } catch {
    return [];
  }
}

export async function importCurrentProfile(request: ProfileImportRequest): Promise<ProfileImportResult> {
  return await invoke<ProfileImportResult>("import_current_profile", { request });
}

export async function getSettings(): Promise<AppSettings> {
  return await invoke<AppSettings>("get_settings");
}

export async function saveSettings(settings: AppSettings): Promise<AppSettings> {
  return await invoke<AppSettings>("save_settings", { settings });
}

export async function listSwitchHistory(): Promise<SwitchHistoryEntry[]> {
  try {
    return await invoke<SwitchHistoryEntry[]>("list_switch_history");
  } catch {
    return [];
  }
}

export async function clearSwitchHistory(): Promise<void> {
  await invoke<void>("clear_switch_history");
}

export async function checkRecoveryStatus(): Promise<RecoveryStatus> {
  try {
    return await invoke<RecoveryStatus>("check_recovery_status");
  } catch (error) {
    return {
      needsRecovery: false,
      transactionId: null,
      phase: null,
      message: `Recovery check unavailable in this runtime: ${String(error)}`
    };
  }
}

function emptyEnvironment(id: EnvironmentId): EnvironmentState {
  return {
    id,
    installed: false,
    executablePath: null,
    discoveredPaths: [],
    running: false,
    runningProcesses: [],
    permission: "unknown",
    accountHint: "Unknown",
    support: "not-detected",
    statusMessage: "Read-only detector has not run"
  };
}
