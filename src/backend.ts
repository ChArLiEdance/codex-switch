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
