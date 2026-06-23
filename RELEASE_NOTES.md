# Release Notes

## 0.1.0 Developer Preview

This developer preview implements the project foundation and the backend pieces needed for safe local Codex profile switching.

### Verified

- Tauri + React + TypeScript desktop shell
- Privacy-focused `.gitignore`
- Read-only detection for Codex CLI, VS Code, and Codex Desktop App candidates
- Redacted account-hint extraction from bounded read-only auth/config scans
- Profile metadata model
- OS keychain-backed secret-store abstraction
- Current local state import into profiles
- Metadata persistence at `~/.codex-switch/profiles.json`
- Profile edit, delete, and set-default actions from the Profiles UI
- Keychain payload cleanup when a saved Profile is deleted
- Switch transaction core with backup, atomic restore, and rollback
- Desktop App switch coordinator with graceful quit and optional restart
- Codex CLI switch coordinator with running-task detection and availability validation
- VS Code switch coordinator with manual reload or explicit restart behavior
- UI-facing saved Profile switch command backed by one combined close, restore, restart, and rollback transaction
- Switch dialog close confirmation for running Desktop/VS Code windows
- Local settings persistence
- Local switch history persistence and clearing
- Startup recovery inspection for unfinished transaction journals
- App-only macOS Tauri bundle

### Not Yet Verified As Complete Production Switching

- Real Codex Desktop authentication path semantics
- Real VS Code Codex/OpenAI extension authentication path semantics
- Account identity verification after switching
- Automatic Desktop/VS Code close and restart against real running apps on this machine
- DMG packaging on this machine

### Verification Commands

```bash
scripts/verify.sh
```

Manual equivalent:

```bash
npm run build
(cd src-tauri && cargo test)
npm run tauri:build -- --bundles app
```
