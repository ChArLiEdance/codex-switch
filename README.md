# Codex Switch

Codex Switch is a local desktop app for managing multiple already-authorized Codex login profiles and switching the local Codex environment between them. The app is designed around official user login flows only: it does not collect passwords, automate web login, bypass MFA, scrape browser cookies, or share accounts.

## Current Status

Milestone 1 created the project structure, Tauri + React shell, documentation, and privacy-focused repository hygiene. Milestone 2 adds the read-only detector contract and UI wiring for Codex CLI, VS Code, and Codex Desktop App evidence. Milestone 4 can import selected current-environment state into a saved local Profile. The detector also supports user-configured path overrides for machine-specific auth, config, cache, or app paths, plus a copyable redacted diagnostics report for real-machine path validation.

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
3. If the detector misses a machine-specific path, add it in Settings as a custom detector path and rescan.
4. Review the read-only current-state evidence, including detected account hints and candidate paths. Use the Environment diagnostics report when collecting redacted evidence for real-machine validation.
5. Save the current authorized local state as a named profile.
6. Select a target profile and choose which environments to switch.
7. The app stops relevant processes, backs up the current state, restores the target profile, restarts supported apps, verifies the result, and rolls back on failure.

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

- Read-only environment detection code is present in the Tauri backend and is wired into the UI. VS Code and Codex Desktop detection now records typed auth, cache, and config candidates for known OpenAI/Codex support paths instead of relying only on broad support roots. Settings can add custom detector paths that are included in future scans and imports. The Environment page can generate a copyable `environment-diagnostics/v1` report with redacted paths, redacted account hints, permissions, and support state.
- Current environment import is implemented for selected CLI, VS Code, and Desktop detector results. The Profiles UI now guides the official-login-first workflow, shows read-only current account evidence beside the import controls, and blocks multi-environment imports until same-account confirmation is checked.
- Profile management supports editing names, tags, notes, setting a default Profile, and deleting Profile metadata with associated keychain payload cleanup.
- Successful restore transactions update Profile `lastUsedAt`, record the previous Profile in local history, and expose Home actions for restoring the default Profile or switching back to the previous Profile.
- Recovery detection surfaces unfinished transaction journals and lets the user mark a reviewed journal failed before using Restore default or Switch back for an explicit corrective switch.
- The Settings option to restore the default account on exit is wired to the normal transaction path. It runs only when enabled, skips when the default Profile is already current, and leaves the app open with an error if a default restore cannot be completed.
- Detector account hints use bounded read-only scanning and redaction when an email-like local identifier is safely discoverable. Post-switch verification now compares post-restore redacted hints with the target Profile and records verified, incomplete, or mismatched identity status without logging full emails.
- Backup, restore, rollback, and per-environment switch coordinators are implemented in the backend and covered by simulated tests.
- Codex CLI active-task detection filters process lines for CLI-shaped `codex` commands, ignores harmless version/help checks, and avoids blocking on Codex Desktop or Codex Switch app bundle paths.
- Saved Profile switching is wired from the UI to a combined backend transaction. With explicit user confirmation it closes running Desktop/VS Code processes, rejects duplicate restore targets, direct symlink targets, nearest existing symlink target ancestors, existing non-file restore targets, unsafe transaction IDs, symlink backup roots, and non-directory backup roots before writing, verifies copied backup bytes by readback, restores selected environments, refreshes captured cache artifacts, verifies restored auth/config bytes and Unix file modes by readback, verifies rollback results after failed switches, records rollback failures as terminal transaction events, restarts supported apps, verifies restarted apps by observing their processes, records history, and rolls back if backup verification, restore, cache refresh, readback verification, restart, or restart verification fails. The switch dialog displays transaction phase outcomes, identity verification status, the non-secret transaction event timeline returned by the backend, and a direct rollback action to the previous usable Profile when available.
- The switch dialog constrains environment choices to the selected Profile's captured environments and shows the completeness reason for unsupported CLI, VS Code, or Desktop targets.
- Manual restart retry buttons are available for Codex Desktop and VS Code from the switch result dialog and Environment page. They reuse the same platform restart controllers, verify that the process appears again, and report errors without modifying Profile state.
- Automatic Desktop/VS Code close, restart, and post-restart process verification are covered by mock process-controller tests. Real Codex Desktop and VS Code extension authentication path semantics are not yet verified.
- `npm run tauri:build -- --bundles app` succeeds. Full default bundling currently fails at the DMG packaging step on this machine; Tauri's exposed DMG config does not provide a skip-Finder option for this environment.
