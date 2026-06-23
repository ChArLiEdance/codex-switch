# Verification

Run the full local verification suite:

```bash
scripts/verify.sh
```

The script checks:

- TypeScript and Vite production build
- Rust unit tests
- macOS app-only Tauri bundle

Current expected results:

- `npm run build` succeeds.
- `cargo test` succeeds with tests covering profile metadata, profile update/delete/last-used behavior, secret-store abstraction, typed VS Code/Desktop detector candidates, settings-managed custom detector paths, redacted account-hint parsing, import, transaction backup/rollback/journaling, transactional cache refresh, restored auth/config byte and Unix-mode readback verification and rollback, Desktop coordinator, CLI coordinator including active-task process filtering, VS Code coordinator, post-restart process verification and rollback, switch identity verification, manual restart retry, settings, history, restore-default-on-exit, and recovery detection/resolution state.
- `npm run build` type-checks the switch dialog contract for transaction events, identity verification, and Home account verification status rendering.
- `npm run build` type-checks the switch result dialog rollback action to the previous usable Profile.
- `npm run build` type-checks that switch scope controls read selected Profile environment state and cannot submit unsupported targets.
- `npm run build` type-checks the guided official-login import controls, current-state evidence display, and frontend same-account import preflight.
- `npm run build` type-checks Settings controls for custom detector path overrides.
- `npm run tauri:build -- --bundles app` succeeds and produces `src-tauri/target/release/bundle/macos/Codex Switch.app`.

Known packaging limitation:

- Default Tauri bundling that includes DMG packaging has failed on this machine during `bundle_dmg.sh`.
- Tauri's exposed `bundle.macOS.dmg` schema supports layout/background settings, but not the `create-dmg` `--skip-jenkins` or `--sandbox-safe` options needed for this non-GUI packaging path.
- App-only bundling is the verified build target for this development pass.

Real-environment limitations:

- Read-only detection is implemented, including bounded redacted account-hint extraction, but real Codex Desktop and VS Code extension auth paths remain detector candidates until validated on machines with those apps and accounts.
- Post-switch identity verification compares redacted local account hints when they are discoverable. If no hint is available, status remains incomplete; CLI command validation still proves CLI availability only.
- Tests use mock/simulated environments and do not require real tokens.
