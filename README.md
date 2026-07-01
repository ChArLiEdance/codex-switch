# Codex Switch

<p align="center">
  <a href="README.zh-CN.md">简体中文</a> |
  <a href="https://charliedance.github.io/codex-switch/">Website</a> |
  <a href="CHANGELOG.md">Changelog</a> |
  <a href="https://github.com/ChArLiEdance/codex-switch/releases">Releases</a>
</p>

<p align="center">
  <a href="https://github.com/ChArLiEdance/codex-switch/stargazers"><img alt="GitHub stars" src="https://img.shields.io/github/stars/ChArLiEdance/codex-switch?style=flat&amp;logo=github"></a>
  <a href="LICENSE"><img alt="License" src="https://img.shields.io/badge/dynamic/json?color=blue&amp;label=license&amp;query=$.license.spdx_id&amp;url=https%3A%2F%2Fapi.github.com%2Frepos%2FChArLiEdance%2Fcodex-switch"></a>
  <a href="https://github.com/ChArLiEdance/codex-switch/releases"><img alt="Downloads" src="https://img.shields.io/github/downloads/ChArLiEdance/codex-switch/total?cacheSeconds=60"></a>
  <img alt="Rust" src="https://img.shields.io/badge/Rust-stable-orange?logo=rust">
  <img alt="Tauri" src="https://img.shields.io/badge/Tauri-2.x-24C8DB?logo=tauri">
  <img alt="TypeScript" src="https://img.shields.io/badge/TypeScript-5.x-3178C6?logo=typescript&logoColor=white">
</p>

**Codex Switch** is a local desktop app for managing multiple OpenAI Codex login profiles on one computer. It lets you view the active account, switch between saved local profiles, refresh quota information, and keep each account's local Codex state separated.

Version `1.1.5` adds the signed Tauri auto-updater path, so future releases can be downloaded, verified, installed, and relaunched from inside the app instead of requiring a manual drag-install step. It keeps the refined macOS and Windows tray experience, usable **Skills** and **Prompts** management flow, account switching, login, quota, settings, usage statistics, and session history foundation.

> This is not an official OpenAI project. It only manages local, already-authorized Codex login state. It does not collect passwords, bypass MFA, scrape browser cookies, or provide account sharing.

## Software Features

- **Account cards**: show nickname, detailed account name, plan badge, active login state, 5-hour quota, and weekly quota.
- **Profile management**: add, log in, switch, rename, delete, and reorder local Codex account profiles.
- **Quota lookup**: refresh and expand account quota details, with per-account usage query settings.
- **Usage statistics**: read local Codex session usage, summarize token usage, and show usage trends.
- **Session history**: browse local Codex sessions and resume or inspect previous conversations.
- **Skills and prompts**: manage local Codex skills and reusable prompts with English/Chinese interface labels.
- **Tray experience**: macOS and Windows tray surfaces show the current account, quota state, and quick actions.
- **Signed auto-updates**: checks the latest GitHub Release, verifies Tauri updater signatures, installs supported updates, and relaunches the app.
- **Settings**: switch language, choose light/dark/system theme, configure update URL, restart targets, quota alerts, and Codex CLI path.
- **Privacy-first local flow**: uses local Codex/OAuth state and does not store passwords or browser cookies in the repository.

## Installation

Download installers from the [GitHub Releases](https://github.com/ChArLiEdance/codex-switch/releases) page. The current `1.1.5` release provides macOS Apple Silicon and Windows x64 installers.

### macOS

1. Download [`codex_switch_1.1.5_aarch64.dmg`](https://github.com/ChArLiEdance/codex-switch/releases/download/v1.1.5/codex_switch_1.1.5_aarch64.dmg).
2. Open the `.dmg`.
3. Drag `codex_switch.app` into `Applications`.
4. Launch Codex Switch.

You can also download it from Terminal:

```bash
curl -L -o ~/Downloads/codex_switch_1.1.5_aarch64.dmg \
  https://github.com/ChArLiEdance/codex-switch/releases/download/v1.1.5/codex_switch_1.1.5_aarch64.dmg
```

If you prefer a package installer, download [`codex_switch_1.1.5_aarch64.pkg`](https://github.com/ChArLiEdance/codex-switch/releases/download/v1.1.5/codex_switch_1.1.5_aarch64.pkg) and open it.

macOS x64 is not included in this release.

The current local package is unsigned or ad-hoc signed depending on the build environment. If macOS blocks the first launch, open **System Settings -> Privacy & Security** and allow the app.

### Windows

1. Download [`codex_switch_1.1.5_x64-setup.exe`](https://github.com/ChArLiEdance/codex-switch/releases/download/v1.1.5/codex_switch_1.1.5_x64-setup.exe).
2. Run the installer.
3. Open Codex Switch from the Start menu or desktop shortcut.

You can also download it from PowerShell:

```powershell
Invoke-WebRequest `
  -Uri "https://github.com/ChArLiEdance/codex-switch/releases/download/v1.1.5/codex_switch_1.1.5_x64-setup.exe" `
  -OutFile "$env:USERPROFILE\Downloads\codex_switch_1.1.5_x64-setup.exe"
```

The Windows build uses the Tauri NSIS installer.

## Usage

1. Install and verify Codex CLI on your machine.
2. Open Codex Switch.
3. Go to Settings and confirm the Codex CLI path.
4. Click the plus button to add an account profile.
5. Click Login and finish the official browser OAuth flow.
6. Refresh the account card to load plan and quota information.
7. Add another profile and use Switch to move the active local Codex state between accounts.

Codex Switch works with local account state around:

```text
~/.codex/
~/.codex/account_backup/
~/.codex-switch/
```

## Repository Layout

Current GitHub repository structure:

```text
codex-switch/
  .github/workflows/      GitHub Actions build and release workflow
  macOS-backup/           Legacy shell-based macOS switching scripts
  scripts/                Version sync, macOS artifact, and package helper scripts
  website/                Static product website published by GitHub Pages
  src-tauri/
    capabilities/         Tauri permission capability files
    icons/                App icon assets for macOS and Windows
    mac/                  macOS frontend shell and platform-specific runtime
    shared/               Shared frontend, Tauri commands, metadata, quota, history, and switching logic
    src/                  Tauri Rust entrypoint
    win/                  Windows frontend shell and platform-specific runtime
    Cargo.toml            Rust crate manifest
    tauri.conf.json       Base Tauri configuration
    tauri.macos.conf.json macOS bundle targets
    tauri.windows.conf.json Windows NSIS installer target
  CHANGELOG.md
  LICENSE
  README.md
  README.zh-CN.md
  package.json
  package-lock.json
  tsconfig.json
  vite.config.ts
```

Generated build outputs such as `dist/`, `node_modules/`, and `src-tauri/target/` are intentionally ignored by Git.

## Development

```bash
npm install
npm run build
npm run test:rust
npm run tauri:dev
```

Preview the Windows UI in a browser with mocked Tauri commands:

```bash
npm run dev:windows-preview
```

Then open `http://127.0.0.1:1421`. This preview uses mock profiles, quota,
usage statistics, session history, settings, and account actions. It does not
read or write real Codex credentials and does not switch local accounts.

Build static Windows preview assets without starting a dev server:

```bash
npm run build:windows-preview
```

The static output is written to `dist/windows-preview`, separate from the
production Tauri front-end output in `dist/web`.

Build local macOS packages:

```bash
npm run tauri:build:macos-release
```

Build Windows installer on a Windows runner:

```bash
npm run tauri:build:windows
```

## Release Packaging

Installers should be uploaded as GitHub Release assets. They should not be committed into the source repository and do not belong inside `package.json`.

Current release assets for `1.1.5`:

```text
codex_switch_1.1.5_aarch64.dmg
codex_switch_1.1.5_aarch64.pkg
codex_switch_1.1.5_x64-setup.exe
latest.json
codex_switch.app.tar.gz
codex_switch.app.tar.gz.sig
codex_switch_1.1.5_x64-setup.exe.sig
```

The repository also has GitHub Actions configured to build release artifacts from version tags and publish the signed updater manifest. The `1.1.5` public release intentionally does not include macOS x64.

## Privacy And Safety

- Does not collect OpenAI passwords.
- Does not automate web login.
- Does not bypass MFA.
- Does not scrape browser cookies.
- Does not write tokens, API keys, passwords, or cookies to Git.
- Login state is used only for local profile switching and quota lookup.

## Stack

- Rust + Tauri 2
- TypeScript + Vite
- Native HTML/CSS frontend
- Local Codex account metadata and quota lookup
- GitHub Actions multi-platform build workflow

## License

MIT License. See [LICENSE](LICENSE).
