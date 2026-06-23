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
- Last-used Profile tracking and previous-Profile switch history
- Home actions for restoring the default Profile and switching back to the previous Profile
- Local settings persistence
- Local switch history persistence and clearing
- Switch transaction journal persistence plus startup recovery inspection and Mark reviewed handling
- Post-switch redacted account-hint comparison with verified, incomplete, or mismatched identity status
- Restore-default-on-exit hook backed by the normal switch transaction path
- Functional Settings restore-defaults button
- Transaction-aware switch dialog progress rows and Home account verification badge
- Manual restart retry commands and UI buttons for Codex Desktop and VS Code
- App-only macOS Tauri bundle

### Not Yet Verified As Complete Production Switching

- Real Codex Desktop authentication path semantics
- Real VS Code Codex/OpenAI extension authentication path semantics
- Strong account identity verification when no redacted local account hint is discoverable
- Automatic Desktop/VS Code close and restart against real running apps on this machine
- Restore-default-on-exit against real running Desktop/VS Code apps when close confirmation is enabled
- Manual restart retry against real Codex Desktop and VS Code app paths on this machine
- DMG packaging on this machine; default Tauri bundling fails during `bundle_dmg.sh`

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
