# Architecture

## Overview

Codex Switch uses a React frontend for the desktop interface and a Rust Tauri backend for filesystem access, process management, secure credential storage, environment detection, backup, restore, and transaction recovery.

The design separates the three target environments behind adapters:

- `cli`: Codex CLI
- `vscode`: VS Code Codex / OpenAI related login state
- `desktop`: Codex Desktop App

Adapters must discover paths and capabilities instead of assuming a single auth file.

## Profile Model

A profile contains non-secret metadata plus one secret payload per supported environment.

Metadata:

- ID
- Display name
- Redacted account hint
- Tags
- Notes
- Default flag
- Last-used timestamp
- Per-environment support status
- Per-environment completeness reason

Secret payload:

- Stored in the OS secure credential store or encrypted with a key protected by that store
- Never exposed to React as raw auth content
- Never logged

The backend models this split with `ProfileMetadata`, `EnvironmentProfileState`, and `SecretVault`. Metadata stores only `secret_ref` identifiers. Secret payloads are written through the `SecretStore` trait, whose production implementation uses the OS keychain through the Rust `keyring` crate.

Profile metadata is persisted by `ProfileRepository` as `~/.codex-switch/profiles.json`. The repository validates metadata before saving and clears older default flags when a newly imported or edited profile is marked default. Profile updates are metadata-only operations for name, tags, notes, and default status. Profile deletion removes metadata and the corresponding per-environment secret payloads from the OS keychain.

## Import Flow

The current import flow:

1. Directs the user to complete official Codex login outside the app before capture.
2. Runs read-only environment detection and shows current account hints plus candidate path counts in the Profiles UI.
3. Requires explicit same-account confirmation when importing more than one target environment into a single profile before invoking the backend command.
4. Recursively captures readable detector artifacts for selected auth, config, and cache paths with per-environment file and byte limits.
5. Skips symlinks and unreadable paths with non-secret skipped reasons.
6. Stores the serialized snapshot through `SecretVault`.
7. Saves only profile metadata and opaque `secret_ref` values to the metadata file.

Raw captured file contents are not returned to React.

## Environment Adapter Contract

Each adapter is expected to support:

- `detect(read_only: bool)`
- `capture_current_state(profile_id)`
- `backup_current_state(transaction_id)`
- `restore_profile(profile_id, transaction_id)`
- `clear_or_refresh_cache(transaction_id)`
- `verify(profile_id)`
- `restart_if_supported()`

Detection returns installed path, config paths, cache paths, running state, permission status, redacted account hint when safely available, and support confidence.

The macOS detector seeds explicit candidates for known VS Code extension storage (`openai.chatgpt`, `openai.codex`) and Codex Desktop browser-support locations such as local storage, session storage, network, partition, cache, and bundle-support directories. These are still read-only discovery records; auth contents remain bounded by the account-hint scanner and are not logged.

Settings may add custom detector paths per environment and path kind. These overrides are stored as non-secret path metadata in `settings.json`, are expanded for leading `~/`, and are appended to read-only detection before import snapshots are captured. This keeps unknown real-world VS Code or Desktop auth locations configurable without treating a guessed path as universally verified.

## Switch Transaction

A switch is modeled as an append-only transaction state machine:

1. Planned
2. Confirmed
3. ClosingProcesses
4. ProcessesClosed
5. BackingUp
6. BackupComplete
7. Restoring
8. RestoreComplete
9. Restarting
10. Verifying
11. Completed
12. RollingBack
13. RolledBack
14. Failed

Every transition records only non-secret metadata. If the app exits during a transaction, startup recovery checks the last transaction and offers rollback or completion verification.

The current `TransactionRunner` implements the filesystem core of this model for restore plans:

- Creates a per-transaction backup manifest before writing target files
- Writes restored files through a temporary file followed by rename
- Removes restored cache artifacts after restore so target apps refresh volatile cache state on next launch
- Rolls back completed writes from the backup manifest if a later restore step fails
- Removes files created during a failed transaction when no previous file existed
- Records transaction events without file contents

The runner is currently covered by simulated filesystem tests. It is not yet connected to real Codex profile switching commands.

## Desktop App Adapter

`DesktopAppCoordinator` wraps the transaction runner for Codex Desktop App switching:

1. Detect matching Desktop process names.
2. Request a graceful quit through the platform process controller.
3. Wait until the process has stopped, returning the still-running process list on timeout.
4. Restore Desktop profile artifacts through `TransactionRunner`.
5. Skip restart when restore fails and rollback has run.
6. Restart the app when `auto_restart` is enabled and an app path is available.
7. Poll for the Desktop process after restart and report a restart timeout if launch is not observed.

The macOS process controller uses application-level quit and open commands. Tests use a mock process controller, so simulated coverage does not kill or restart real apps.

## Codex CLI Adapter

`CliSwitchCoordinator` handles CLI profile switching:

1. Detect currently running Codex CLI tasks before restore.
2. Refuse to switch when CLI work appears active, returning the matching process lines.
3. Restore CLI profile artifacts through `TransactionRunner`.
4. Validate immediate CLI availability with a runtime validator.
5. Return a manual verification command for the user.

The current system validator runs `codex --version` or the detected executable path with `--version`. This proves the CLI responds after restore, but it does not prove account identity. The report therefore marks this as inconclusive until a real identity check is available.

The system CLI runtime filters process lines so only CLI-shaped `codex` commands block switching. It ignores harmless version/help checks and excludes Codex Desktop or Codex Switch app bundle paths to avoid confusing GUI processes with active CLI work.

## VS Code Adapter

`VscodeSwitchCoordinator` restores VS Code profile artifacts and then performs the configured post-switch action:

- `manual_reload_window`: returns an explicit "Developer: Reload Window" instruction without closing VS Code.
- `restart_app`: asks VS Code to quit, waits until it is stopped, reopens the configured app path, then polls for the VS Code process.
- `none`: records that reload/restart was skipped.

Restore failures skip all reload or restart actions. Timeout errors include the still-running process names. Tests use a mock controller and do not operate on a real VS Code instance.

## Manual Restart Retry

`restart_desktop_app` and `restart_vscode_app` expose narrow retry commands for cases where an app restart was skipped, failed, or needs to be repeated after the user has resolved a local issue. These commands call the same platform process controllers used by switch transactions, verify that the target process appears again, return a non-secret status message, and do not read or write Profile data. The switch dialog and Environment page pass the app paths discovered by read-only detection.

## Settings, History, And Recovery

`AppStateRepository` persists non-secret app state under `~/.codex-switch`:

- `settings.json`: default switch scope, close confirmation, app restart preference, default-on-exit preference, VS Code reload mode, and custom detector path overrides.
- `history.json`: local switch history with previous/target profile names, environment list, status, and error category only.
- `transactions/current.json`: current switch transaction journal written before restore starts, overwritten with the terminal transaction state after restore/restart completes, and inspected on startup for recovery.

`check_recovery_status` reports whether a transaction journal is unfinished. It does not read auth payloads or secret snapshots. A non-terminal journal means the app exited while a restore transaction was in progress or before terminal status was persisted. `resolve_recovery_status` lets the user mark a reviewed non-terminal journal as failed, then the Home view can launch a normal Restore default or Switch back operation to correct local account state through the regular switch path.

## Saved Profile Switch Command

`switch_to_profile` is the current UI-facing bridge from saved Profile metadata to restore execution:

1. Loads the selected Profile metadata.
2. Loads selected environment snapshots from the secret vault.
3. Builds a combined restore plan from captured artifact source paths.
4. Checks for active CLI tasks and blocks switching while a Codex CLI task is running.
5. Detects running Desktop and VS Code processes and requires explicit UI confirmation before asking them to quit.
6. Persists a planned transaction journal before filesystem restore begins.
7. Runs one `TransactionRunner` backup/restore/cache-refresh/rollback transaction.
8. Runs Desktop restart and VS Code restart, when enabled, inside a post-restore transaction hook, then verifies that the restarted process is observed.
9. Persists the terminal transaction journal returned by the runner.
10. Rolls back restored files and refreshed cache paths if restore, cache refresh, post-restore restart, or post-restart process verification fails.
11. Marks the target Profile with `lastUsedAt` on success.
12. Reads restored target files with the same bounded account-hint scanner used by read-only detection.
13. Compares discovered redacted hints with the target Profile's redacted hint and marks identity as verified, incomplete, or mismatched.
14. Appends local switch history, including the previously most recently used Profile when known. Completed restore transactions are recorded as `success` only when the redacted identity hint matches; otherwise they are recorded as `incomplete` with an identity error category.
15. Returns closed-process, restarted-app, identity-verification, warning, manual-verification details, and non-secret transaction events to the dialog.

This command now makes saved Profiles switchable from the UI and coordinates process close/restart for Desktop and VS Code after explicit confirmation. The switch dialog disables unsupported target environments for the selected Profile, shows each unavailable environment's completeness reason, and maps the returned transaction events into phase rows for closing apps, backup, restore, restart, verification, and history recording. The process behavior is covered by mock process-controller tests; real Codex Desktop and VS Code extension auth-path semantics still require machine-specific validation.

The Home view derives the current Profile from the latest `lastUsedAt` value, then offers quick actions to restore the default Profile or switch back to the previous Profile recorded in history. These actions reuse `switch_to_profile` and the configured default switch scope.

The switch result dialog also exposes a direct rollback button when the previous usable Profile has captured state for at least one just-switched environment. This is still a normal `switch_to_profile` call, so process confirmation, backup, restore, restart, identity verification, and history recording stay on the same transaction path.

## Restore Default On Exit

The frontend registers a Tauri window close handler. When `restoreDefaultOnExit` is enabled, the handler prevents immediate close, calls `restore_default_on_exit`, and closes the window only after the backend returns successfully. If the backend reports an error, the window remains open and the Home status message shows the failure.

`restore_default_on_exit` reads persisted settings and Profiles, then skips safely when the setting is disabled, no default Profile exists, the default Profile is already the latest used Profile, or the default Profile has no available environments inside the default scope. When a restore is needed, it reuses `switch_to_profile` semantics with app restart disabled and VS Code reload disabled because the app itself is exiting. If close confirmation is still enabled in settings, running Desktop or VS Code processes can still block the exit restore instead of being closed silently.

## Atomic Restore Strategy

Restores should write into temporary staging paths, verify permissions and checksums, then atomically rename into place where the platform supports it. If any target fails, completed targets are restored from the timestamped backup.

## Error Handling

Adapters must distinguish:

- Unknown path
- Missing file
- Permission denied
- Process still running
- Failed process shutdown
- Restart unavailable
- Verification inconclusive
- Restore-default-on-exit skipped or failed
- Secret-store failure
- Backup failure
- Restore failure

Unknown or inconclusive account identity must be reported as "configuration switched, identity verification incomplete" rather than as verified support. A mismatched redacted hint is reported separately and the history entry is not marked successful.
