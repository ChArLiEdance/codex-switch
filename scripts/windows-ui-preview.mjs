import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import path from "node:path";
import process from "node:process";

const rootDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const viteBin = path.join(
  rootDir,
  "node_modules",
  ".bin",
  process.platform === "win32" ? "vite.cmd" : "vite",
);

const buildMode = process.argv.includes("--build");
const args = buildMode
  ? ["build"]
  : ["--host", "127.0.0.1", "--port", "1421"];

const env = {
  ...process.env,
  CODEX_UI_TARGET: "windows",
  CODEX_PREVIEW_MOCKS: "1",
};

if (buildMode) {
  env.CODEX_PREVIEW_OUT_DIR = "../../../dist/windows-preview";
}

if (!buildMode) {
  console.log("Windows UI Preview: http://127.0.0.1:1421");
  console.log("Preview mode uses mock data and does not touch local Codex credentials.");
}

const child = spawn(viteBin, args, {
  cwd: rootDir,
  env,
  stdio: "inherit",
});

child.on("exit", (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal);
    return;
  }
  process.exit(code ?? 0);
});
