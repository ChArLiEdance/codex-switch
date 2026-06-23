# Roadmap

## Milestone 1

- Initialize `codex_switch`
- Add Tauri + React project shell
- Add base navigation and mock desktop UI
- Add README, SECURITY, ARCHITECTURE, and `.gitignore`

## Milestone 2

- Implement read-only environment detection for Codex CLI, VS Code, and Codex Desktop App
- Report installation path, running state, candidate auth/config/cache paths, and permissions
- Keep unknown paths explicit instead of hard-coding unsupported assumptions
- Wire detector output into the Environment page and Home status cards

## Milestone 3

- Add profile metadata model
- Add secure credential-store abstraction
- Add unit tests for profile metadata and secret-store interface behavior

## Milestone 4

- Import current environment state into a profile
- Support partial environment profiles and explicit merge confirmation
- Persist metadata locally while storing captured state through the secret vault

## Milestone 5

- Implement switch transaction state machine
- Add backup, restore, atomic write, and rollback tests with simulated environments

## Milestones 6-10

- Add app-specific process control and restart flows
- Add Codex CLI validation
- Add VS Code restart or reload flow
- Complete UI, logs, settings, abnormal-exit recovery
- Finalize tests, docs, build checks, and release notes
