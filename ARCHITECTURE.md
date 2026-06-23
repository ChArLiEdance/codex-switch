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
- Secret-store failure
- Backup failure
- Restore failure

Unknown or inconclusive account identity must be reported as "configuration switched, identity verification incomplete" rather than as verified support.

