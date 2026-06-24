# Codex Switch

<p align="center">
  <a href="README.zh-CN.md">简体中文</a> |
  <a href="CHANGELOG.md">Changelog</a>
</p>

<p align="center">
  <a href="https://github.com/ChArLiEdance/codex-switch/stargazers"><img alt="GitHub stars" src="https://img.shields.io/github/stars/ChArLiEdance/codex-switch?style=social"></a>
  <a href="LICENSE"><img alt="License" src="https://img.shields.io/github/license/ChArLiEdance/codex-switch"></a>
  <img alt="Rust" src="https://img.shields.io/badge/Rust-stable-orange?logo=rust">
  <img alt="Tauri" src="https://img.shields.io/badge/Tauri-2.x-24C8DB?logo=tauri">
  <img alt="TypeScript" src="https://img.shields.io/badge/TypeScript-5.x-3178C6?logo=typescript&logoColor=white">
</p>

**Codex Switch** is a local desktop app for managing OpenAI Codex login profiles. This version has been migrated to the `codex-account-switch` style Tauri implementation, including its UI structure, runtime layout, account cards, login flow, switching flow, quota reading, and build scripts.

The goal is simple: manage multiple Codex accounts on one machine, view the current account, plan, 5-hour quota, weekly quota, and usage context, then safely switch the local `~/.codex` state.

> This is not an official OpenAI project. It only manages local, already-authorized Codex login state. It does not collect passwords, bypass MFA, scrape browser cookies, or provide account sharing.

## Current Status

- Migrated to the `codex-account-switch` frontend and backend structure.
- macOS native Tauri app builds and runs locally.
- Supports account cards, login, switching, deletion, renaming, quota refresh, and Base URL indicators.
- Reads plan and quota metadata and displays 5-hour and weekly remaining quota on account cards.
- Includes an English default README and a Simplified Chinese README.
- GitHub Actions includes build jobs for macOS arm64, macOS x64, Windows x64, and Linux x86_64.

Verified locally:

```bash
npm run build
npm run test:rust
npm run tauri:build:macos-app
```

Local app output:

```text
dist/codex_switch.app
```

## Features

- **Current account view**: show the active Codex profile, plan state, quota windows, and refresh state.
- **Multiple account management**: add, log in, switch, rename, and delete local Profiles.
- **Quota view**: read ChatGPT / Codex account metadata and display 5-hour and weekly quota percentages.
- **Account switching**: restore a target Profile into the active `~/.codex` state while keeping local account directories.
- **Official login flow**: use `codex login` / OAuth; the app does not collect passwords.
- **CLI path detection**: detect the Codex CLI path and allow manual override in Settings.
- **Local cache**: cache account metadata and quota snapshots to reduce repeated requests.
- **Cross-platform structure**: macOS and Windows runtimes are separated, with shared logic under `src-tauri/shared/`.

## Quick Start

```bash
git clone https://github.com/ChArLiEdance/codex-switch.git
cd codex-switch
npm install
npm run tauri:dev
```

Build an unsigned macOS app for local testing:

```bash
npm run tauri:build:macos-app
open -n dist/codex_switch.app
```

## Usage

1. Make sure Codex CLI is installed.
2. Open Codex Switch.
3. Check the Codex CLI path in Settings.
4. Add an account Profile on the Accounts page.
5. Click Login and finish the official browser login flow.
6. Refresh account information to view plan and quota.
7. Add another account and switch between account cards.

Local account state is centered around:

```text
~/.codex/
~/.codex/account_backup/
~/.codex-switch/
```

## Repository Layout

```text
codex_switch/
  src-tauri/
    mac/              macOS frontend shell and runtime
    win/              Windows frontend shell and runtime
    shared/           shared frontend, Tauri commands, and runtime logic
    src/              Tauri entrypoint
    capabilities/     Tauri capability config
    icons/            app icons
  scripts/            version sync, macOS artifact layout, pkg scripts
  macOS-backup/       backup shell workflow
  examples/           example account directory structure
  .github/workflows/  CI build workflow
```

## Development

```bash
npm install                         # install frontend and Tauri CLI dependencies
npm run build                       # TypeScript + Vite production build
npm run test:rust                   # Rust unit tests
npm run tauri:dev                   # desktop development mode
npm run tauri:build:macos-app       # unsigned macOS app for local testing
```

Windows / Linux builds are mainly handled by GitHub Actions:

```bash
npm run tauri:build:windows
npm run tauri:build:linux
```

## Local Test Build

```bash
npm run tauri:build:macos-app
open -n dist/codex_switch.app
```

The local app is unsigned. On first launch, macOS may require allowing it from System Settings -> Privacy & Security.

## Privacy And Safety

- Does not collect OpenAI passwords.
- Does not automate web login.
- Does not bypass MFA.
- Does not scrape browser cookies.
- Does not write tokens, API keys, passwords, or cookies to Git.
- Login state is used only for local Profile switching and quota lookup.

## Stack

- Rust + Tauri 2
- TypeScript + Vite
- Native HTML/CSS frontend structure
- ChatGPT / Codex account metadata lookup
- GitHub Actions multi-platform builds

## License

MIT License. See [LICENSE](LICENSE).
