# Codex Switch

Codex Switch is a local desktop app for managing multiple already-authorized Codex login profiles and switching the local Codex environment between them. The app is designed around official user login flows only: it does not collect passwords, automate web login, bypass MFA, scrape browser cookies, or share accounts.

## Current Status

Milestone 1 created the project structure, Tauri + React shell, documentation, and privacy-focused repository hygiene. Milestone 2 adds the read-only detector contract and UI wiring for Codex CLI, VS Code, and Codex Desktop App evidence. Milestone 4 can import selected current-environment state into a saved local Profile.

The React frontend builds successfully on this machine. The Tauri backend compiles, Rust detector tests pass, and an app-only macOS bundle is produced at `src-tauri/target/release/bundle/macos/Codex Switch.app`.

## Target Environments

- Codex CLI
- VS Code Codex / OpenAI related login environment
- Codex Desktop App

Each environment will have its own adapter. Profiles may support one, two, or all three targets, and incomplete coverage must be visible in the UI.

## Requirements

- Node.js 20 or newer
- npm 10 or newer
- Rust stable toolchain for Tauri development
- Platform-specific secure credential store:
  - macOS Keychain
  - Windows Credential Manager
  - Linux Secret Service / keyring

This workspace verifies the frontend with npm and the Tauri backend with Cargo. Full installer packaging may require additional host-specific macOS packaging prerequisites.

## Development

```bash
npm install
npm run dev
```

For a desktop window after Rust is available:

```bash
npm run tauri:dev
```

## Build

```bash
npm run build
npm run tauri:build
```

Verified app-only bundle:

```bash
npm run tauri:build -- --bundles app
```

Full local verification:

```bash
scripts/verify.sh
```

## Usage Model

1. Sign in through the official Codex login flow outside this app.
2. Open Codex Switch and run environment detection.
3. Save the current authorized local state as a named profile.
4. Select a target profile and choose which environments to switch.
5. The app stops relevant processes, backs up the current state, restores the target profile, restarts supported apps, verifies the result, and rolls back on failure.

## Privacy Rules

Sensitive auth material must never be stored in Git, logs, localStorage, crash reports, ordinary JSON metadata, or frontend state stores. Metadata may contain only redacted account hints, support status, timestamps, labels, and notes. Secret payloads belong in the OS secure credential store or encrypted local storage with keys protected by that store.

Profile metadata is stored at `~/.codex-switch/profiles.json`. Captured auth/config/cache artifacts are serialized through the backend and stored behind opaque keychain references; React receives only metadata, counts, warnings, and secret reference IDs.

## Repository Layout

```text
codex_switch/
  src/              React UI
  src-tauri/        Rust desktop backend
  docs/             Product and engineering notes
  scripts/          Development helper scripts
  tests/            Integration and mock-environment tests
```

## Known Limits

- Read-only environment detection code is present in the Tauri backend and is wired into the UI.
- Current environment import is implemented for selected CLI, VS Code, and Desktop detector results. Multi-environment import requires explicit same-account confirmation.
- Account identity verification is not implemented yet; detector account hints are `Unknown`.
- Backup, restore, rollback, and per-environment switch coordinators are implemented in the backend and covered by simulated tests.
- Saved Profile switching is wired from the UI to a combined backend transaction. With explicit user confirmation it closes running Desktop/VS Code processes, restores selected environments, restarts supported apps, records history, and rolls back if restore or restart fails.
- Automatic Desktop/VS Code close and restart are covered by mock process-controller tests. Real Codex Desktop and VS Code extension authentication path semantics are not yet verified.
- `npm run tauri:build -- --bundles app` succeeds. Full default bundling currently fails at the DMG packaging step on this machine.
