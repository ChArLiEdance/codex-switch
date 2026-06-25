# Codex Switch

<p align="center">
  <a href="README.zh-CN.md">简体中文</a> |
  <a href="CHANGELOG.md">Changelog</a> |
  <a href="https://github.com/ChArLiEdance/codex-switch/releases">Releases</a>
</p>

<p align="center">
  <a href="https://github.com/ChArLiEdance/codex-switch/stargazers"><img alt="GitHub stars" src="https://img.shields.io/github/stars/ChArLiEdance/codex-switch?style=social"></a>
  <a href="LICENSE"><img alt="License" src="https://img.shields.io/github/license/ChArLiEdance/codex-switch"></a>
  <img alt="Rust" src="https://img.shields.io/badge/Rust-stable-orange?logo=rust">
  <img alt="Tauri" src="https://img.shields.io/badge/Tauri-2.x-24C8DB?logo=tauri">
  <img alt="TypeScript" src="https://img.shields.io/badge/TypeScript-5.x-3178C6?logo=typescript&logoColor=white">
</p>

**Codex Switch** is a local desktop app for managing multiple OpenAI Codex login profiles on one computer. It lets you view the active account, switch between saved local profiles, refresh quota information, and keep each account's local Codex state separated.

Version `1.0.0` focuses on the account switching, login, quota, settings, and usage/history foundation. The **Skills** and **Prompts** pages are visible in the interface, but these two features are not finished yet and will be completed in a later release.

> This is not an official OpenAI project. It only manages local, already-authorized Codex login state. It does not collect passwords, bypass MFA, scrape browser cookies, or provide account sharing.

## Software Features

- **Account cards**: show nickname, detailed account name, plan badge, active login state, 5-hour quota, and weekly quota.
- **Profile management**: add, log in, switch, rename, delete, and reorder local Codex account profiles.
- **Quota lookup**: refresh and expand account quota details, with per-account usage query settings.
- **Usage statistics**: read local Codex session usage, summarize token usage, and show usage trends.
- **Session history**: browse local Codex sessions and resume or inspect previous conversations.
- **Settings**: switch language, choose light/dark/system theme, configure update URL and Codex CLI path.
- **Privacy-first local flow**: uses local Codex/OAuth state and does not store passwords or browser cookies in the repository.

## Installation

Download installers from the [GitHub Releases](https://github.com/ChArLiEdance/codex-switch/releases) page.

### macOS

1. Download the `.dmg` file for your Mac architecture.
2. Open the `.dmg`.
3. Drag `codex_switch.app` into `Applications`.
4. Launch Codex Switch.

The current local package is unsigned or ad-hoc signed depending on the build environment. If macOS blocks the first launch, open **System Settings -> Privacy & Security** and allow the app.

### Windows

1. Download the Windows `.exe` installer from Releases.
2. Run the installer.
3. Open Codex Switch from the Start menu or desktop shortcut.

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

Expected release assets for `1.0.0`:

```text
codex_switch_1.0.0_*.dmg
codex_switch_1.0.0_*.pkg
codex_switch_*_x64-setup.exe
```

The repository also has GitHub Actions configured to build macOS, Windows, and Linux artifacts from version tags.

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
