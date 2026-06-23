# Codex Switch

Codex Switch is a local desktop app for managing multiple already-authorized Codex login profiles and switching the local Codex environment between them. The app is designed around official user login flows only: it does not collect passwords, automate web login, bypass MFA, scrape browser cookies, or share accounts.

## Current Status

Milestone 1 creates the project structure, Tauri + React shell, documentation, and privacy-focused repository hygiene. The UI currently uses mock data until the Rust environment detectors are implemented in milestone 2.

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

This workspace currently verifies the frontend with npm. Full Tauri execution requires installing Rust and the Tauri prerequisites for the host OS.

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

## Usage Model

1. Sign in through the official Codex login flow outside this app.
2. Open Codex Switch and run environment detection.
3. Save the current authorized local state as a named profile.
4. Select a target profile and choose which environments to switch.
5. The app stops relevant processes, backs up the current state, restores the target profile, restarts supported apps, verifies the result, and rolls back on failure.

## Privacy Rules

Sensitive auth material must never be stored in Git, logs, localStorage, crash reports, ordinary JSON metadata, or frontend state stores. Metadata may contain only redacted account hints, support status, timestamps, labels, and notes. Secret payloads belong in the OS secure credential store or encrypted local storage with keys protected by that store.

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

- Real environment detection is not implemented yet.
- No profile import, backup, restore, process control, or rollback is implemented yet.
- Tauri build has not been verified on this machine because the Rust toolchain is not currently installed.

