import { invoke } from "@tauri-apps/api/core";

export type EnvironmentId = "CLI" | "VS Code" | "Desktop";
export type TargetEnvironment = "cli" | "vscode" | "desktop";
export type EnvironmentPathKind = "app" | "auth" | "config" | "cache";

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

export type DiagnosticPathSummary = {
  kind: "app" | "auth" | "config" | "cache";
  path: string;
  exists: boolean;
  permission: "read-write" | "read-only" | "missing" | "unknown";
};

export type EnvironmentDiagnosticsEntry = {
  id: EnvironmentId;
  installed: boolean;
  executablePath: string | null;
  running: boolean;
  runningProcesses: string[];
  permission: "read-write" | "read-only" | "missing" | "unknown";
  accountHint: string;
  support: "detected" | "partial" | "not-detected";
  statusMessage: string;
  discoveredPaths: DiagnosticPathSummary[];
};

export type EnvironmentDiagnosticsReport = {
  schemaVersion: "environment-diagnostics/v1";
  generatedAt: string;
  os: string;
  readOnly: boolean;
  environments: EnvironmentDiagnosticsEntry[];
  notes: string[];
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

export type ProfileImportPreflightRequest = {
  environments: TargetEnvironment[];
};

export type ImportReadiness = "ready" | "not_selected" | "scan_missing" | "no_readable_artifacts";

export type SkippedReasonSummary = {
  reason: string;
  count: number;
};

export type ImportPreflightEnvironment = {
  environment: TargetEnvironment;
  selected: boolean;
  scanAvailable: boolean;
  support: "detected" | "partial" | "not-detected";
  accountHint: string;
  candidatePathCount: number;
  existingCandidatePathCount: number;
  capturedArtifactCount: number;
  capturedBytes: number;
  skippedArtifactCount: number;
  skippedReasons: SkippedReasonSummary[];
  readiness: ImportReadiness;
};

export type ProfileImportPreflightResult = {
  environments: ImportPreflightEnvironment[];
  warnings: string[];
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

export type ProfileUpdateRequest = {
  profileId: string;
  name: string;
  tags: string[];
  note: string;
  defaultProfile: boolean;
};

export type AppSettings = {
  defaultScope: TargetEnvironment[];
  confirmBeforeClosingApps: boolean;
  autoRestartApps: boolean;
  restoreDefaultOnExit: boolean;
  vscodeReloadMode: "manual_reload_window" | "restart_app" | "none";
  customPaths: EnvironmentPathOverride[];
};

export type EnvironmentPathOverride = {
  environment: TargetEnvironment;
  kind: EnvironmentPathKind;
  path: string;
};

export type SwitchHistoryEntry = {
  id: string;
  switchedAt: string;
  fromProfileId: string | null;
  fromProfile: string | null;
  toProfileId: string | null;
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
  backupManifestFound: boolean;
  backupEntryCount: number | null;
  rollbackAvailable: boolean;
  latestEventMessage: string | null;
};

export type RecoveryRollbackResult = {
  transaction: SwitchTransaction;
  status: RecoveryStatus;
  message: string;
};

export type ProfileSwitchRequest = {
  profileId: string;
  environments: TargetEnvironment[];
  autoRestartApps: boolean;
  vscodeReloadMode: AppSettings["vscodeReloadMode"];
  confirmProcessClose: boolean;
  desktopAppPath: string | null;
  vscodeAppPath: string | null;
  quitTimeoutMs: number;
};

export type ProfileRecoverySwitchRequest = {
  autoRestartApps: boolean;
  vscodeReloadMode: AppSettings["vscodeReloadMode"];
  confirmProcessClose: boolean;
  desktopAppPath: string | null;
  vscodeAppPath: string | null;
  quitTimeoutMs: number;
};

export type SwitchTransactionEvent = {
  phase: string;
  message: string;
};

export type SwitchTransaction = {
  id: string;
  targetProfileId: string;
  phase: string;
  events: SwitchTransactionEvent[];
};

export type SwitchIdentityVerification = {
  status: "verified" | "incomplete" | "mismatch" | "not_checked";
  targetAccountHint: string;
  observed: Array<{
    environment: TargetEnvironment;
    accountHint: string | null;
  }>;
  message: string;
};

export type ProfileSwitchResult = {
  profile: ProfileMetadata;
  transaction: SwitchTransaction;
  identityVerification: SwitchIdentityVerification;
  switchedEnvironments: TargetEnvironment[];
  manualActions: string[];
  warnings: string[];
  closedProcesses: string[];
  restartedApps: string[];
};

export type ProfileRecoverySwitchResult = {
  attempted: boolean;
  action: "restore_default" | "switch_previous" | string;
  reason: string;
  targetProfile: ProfileMetadata | null;
  environments: TargetEnvironment[];
  switchResult: ProfileSwitchResult | null;
};

export type RestoreDefaultOnExitResult = {
  attempted: boolean;
  reason: string;
  switchResult: ProfileSwitchResult | null;
};

export type RestartAppResult = {
  target: "desktop" | "vscode";
  restarted: boolean;
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

export async function environmentDiagnosticsReport(): Promise<EnvironmentDiagnosticsReport> {
  return await invoke<EnvironmentDiagnosticsReport>("environment_diagnostics_report");
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

export async function previewCurrentImport(request: ProfileImportPreflightRequest): Promise<ProfileImportPreflightResult> {
  return await invoke<ProfileImportPreflightResult>("preview_current_import", { request });
}

export async function updateProfile(request: ProfileUpdateRequest): Promise<ProfileMetadata> {
  return await invoke<ProfileMetadata>("update_profile", { request });
}

export async function deleteProfile(profileId: string): Promise<void> {
  await invoke<void>("delete_profile", { request: { profileId } });
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
      message: `Recovery check unavailable in this runtime: ${String(error)}`,
      backupManifestFound: false,
      backupEntryCount: null,
      rollbackAvailable: false,
      latestEventMessage: null
    };
  }
}

export async function resolveRecoveryStatus(): Promise<RecoveryStatus> {
  return await invoke<RecoveryStatus>("resolve_recovery_status");
}

export async function rollbackUnfinishedTransaction(): Promise<RecoveryRollbackResult> {
  return await invoke<RecoveryRollbackResult>("rollback_unfinished_transaction");
}

export async function switchToProfile(request: ProfileSwitchRequest): Promise<ProfileSwitchResult> {
  return await invoke<ProfileSwitchResult>("switch_to_profile", { request });
}

export async function restoreDefaultProfile(request: ProfileRecoverySwitchRequest): Promise<ProfileRecoverySwitchResult> {
  return await invoke<ProfileRecoverySwitchResult>("restore_default_profile", { request });
}

export async function switchPreviousProfile(request: ProfileRecoverySwitchRequest): Promise<ProfileRecoverySwitchResult> {
  return await invoke<ProfileRecoverySwitchResult>("switch_previous_profile", { request });
}

export async function restoreDefaultOnExit(): Promise<RestoreDefaultOnExitResult> {
  return await invoke<RestoreDefaultOnExitResult>("restore_default_on_exit");
}

export async function restartDesktopApp(appPath: string | null): Promise<RestartAppResult> {
  return await invoke<RestartAppResult>("restart_desktop_app", { request: { appPath } });
}

export async function restartVscodeApp(appPath: string | null): Promise<RestartAppResult> {
  return await invoke<RestartAppResult>("restart_vscode_app", { request: { appPath } });
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
