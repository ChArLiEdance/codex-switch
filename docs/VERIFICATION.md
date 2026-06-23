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
- `cargo test` succeeds with tests covering profile metadata, profile update/delete/last-used behavior, secret-store abstraction, redacted account-hint parsing, import, transaction backup/rollback/journaling, Desktop coordinator, CLI coordinator, VS Code coordinator, switch identity verification, settings, history, and recovery detection/resolution state.
- `npm run tauri:build -- --bundles app` succeeds and produces `src-tauri/target/release/bundle/macos/Codex Switch.app`.

Known packaging limitation:

- Default Tauri bundling that includes DMG packaging has failed on this machine during `bundle_dmg.sh`.
- Tauri's exposed `bundle.macOS.dmg` schema supports layout/background settings, but not the `create-dmg` `--skip-jenkins` or `--sandbox-safe` options needed for this non-GUI packaging path.
- App-only bundling is the verified build target for this development pass.

Real-environment limitations:

- Read-only detection is implemented, including bounded redacted account-hint extraction, but real Codex Desktop and VS Code extension auth paths remain detector candidates until validated on machines with those apps and accounts.
- Post-switch identity verification compares redacted local account hints when they are discoverable. If no hint is available, status remains incomplete; CLI command validation still proves CLI availability only.
- Tests use mock/simulated environments and do not require real tokens.
