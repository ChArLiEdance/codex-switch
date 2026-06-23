import { invoke } from "@tauri-apps/api/core";

export type EnvironmentId = "CLI" | "VS Code" | "Desktop";

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

