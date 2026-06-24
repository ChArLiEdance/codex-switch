# Codex Switch

<p align="center">
  <a href="CHANGELOG.md">Changelog</a>
</p>

<p align="center">
  <a href="https://github.com/ChArLiEdance/codex-switch/stargazers"><img alt="GitHub stars" src="https://img.shields.io/github/stars/ChArLiEdance/codex-switch?style=social"></a>
  <a href="LICENSE"><img alt="License" src="https://img.shields.io/github/license/ChArLiEdance/codex-switch"></a>
  <img alt="Rust" src="https://img.shields.io/badge/Rust-stable-orange?logo=rust">
  <img alt="Tauri" src="https://img.shields.io/badge/Tauri-2.x-24C8DB?logo=tauri">
  <img alt="TypeScript" src="https://img.shields.io/badge/TypeScript-5.x-3178C6?logo=typescript&logoColor=white">
</p>

**Codex Switch** 是一个面向 OpenAI Codex 本地登录状态的桌面账号切换工具。当前版本已经整体迁移到 `codex-account-switch` 风格的 Tauri 实现：界面、前端结构、后端 runtime、账号卡片、登录、切换、额度读取和发布脚本都以该实现为基础，后续会在这个版本上继续做个性化修改。

项目目标很直接：在一台电脑上管理多个已经通过官方流程登录的 Codex 账号，查看当前账号、套餐、5 小时额度、周额度和历史用量，并在本地安全切换 `~/.codex` 状态。

> 本项目不是 OpenAI 官方项目。它只管理本机已授权的 Codex 登录状态，不收集密码，不绕过 MFA，不抓取浏览器 Cookie，不提供共享账号能力。

## 当前状态

- 已迁移到 `codex-account-switch` 的前后端结构。
- macOS 原生 Tauri 桌面端可构建、可运行。
- 支持账号卡片、登录、切换、删除、重命名、刷新额度、Base URL 标记等核心交互。
- 支持读取 plan / quota 信息，并在账号卡片上展示 5 小时额度和周额度。
- 默认 README 为英文，并提供简体中文说明页。
- GitHub Actions 已包含 macOS arm64、macOS x64、Windows x64、Linux x86_64 构建矩阵。

当前本机验证过：

```bash
npm run build
npm run test:rust
npm run tauri:build:macos-app
```

本机 `.app` 输出路径：

```text
dist/codex_switch.app
```

## 功能

- **当前账号展示**：显示当前 Codex 账号对应的 Profile、套餐状态、额度窗口和刷新状态。
- **多账号管理**：每个账号对应一个本地 Profile，可以添加、登录、切换、重命名、删除。
- **额度查看**：读取 ChatGPT / Codex 账号元数据，展示 5 小时额度和周额度剩余比例。
- **账号切换**：切换时将目标 Profile 的登录状态恢复到当前 `~/.codex`，并保留本地账号目录。
- **登录流程**：通过官方 `codex login` / OAuth 流程完成授权，不在应用内收集密码。
- **路径检测**：检测 Codex CLI 路径，必要时可在设置中手动指定。
- **本地缓存**：缓存账号元数据和额度快照，减少重复请求。
- **跨平台结构**：macOS、Windows runtime 分离，共享业务逻辑放在 `src-tauri/shared/`。

## 快速开始

```bash
git clone https://github.com/ChArLiEdance/codex-switch.git
cd codex-switch
npm install
npm run tauri:dev
```

本机打包一个未签名的 macOS `.app`：

```bash
npm run tauri:build:macos-app
open -n dist/codex_switch.app
```

## 使用方式

1. 先确认本机已经安装 Codex CLI。
2. 打开 Codex Switch。
3. 在设置页确认 Codex CLI 路径可用。
4. 在账号页添加一个账号 Profile。
5. 点击该账号的登录按钮，按官方浏览器流程完成登录。
6. 登录成功后刷新账号信息，查看套餐和额度。
7. 添加第二个账号后，即可在账号卡片之间切换。

账号数据默认围绕本机 Codex 目录工作：

```text
~/.codex/
~/.codex/account_backup/
~/.codex-switch/
```

## 仓库结构

```text
codex_switch/
  src-tauri/
    mac/              macOS 前端壳与 runtime
    win/              Windows 前端壳与 runtime
    shared/           共享前端、Tauri command、业务 runtime
    src/              Tauri 入口
    capabilities/     Tauri 权限配置
    icons/            应用图标
  scripts/            版本同步、macOS 产物整理、pkg 生成脚本
  macOS-backup/       旧 shell 切换流程的备份脚本
  examples/           示例账号目录结构
  .github/workflows/  CI 与多平台构建
```

## 开发命令

```bash
npm install                         # 安装前端/Tauri CLI 依赖
npm run build                       # TypeScript + Vite production build
npm run test:rust                   # Rust 单元测试
npm run tauri:dev                   # 本地桌面开发模式
npm run tauri:build:macos-app       # 本机生成未签名 macOS app
```

Windows / Linux 构建主要交给 GitHub Actions：

```bash
npm run tauri:build:windows
npm run tauri:build:linux
```

## 本机测试版

如果只是自己测试：

```bash
npm run tauri:build:macos-app
open -n dist/codex_switch.app
```

该产物没有正式签名，macOS 首次打开可能需要在「系统设置 -> 隐私与安全」中允许。

## 隐私与安全

- 不收集 OpenAI 密码。
- 不自动化网页登录。
- 不绕过 MFA。
- 不抓取浏览器 Cookie。
- 不把 token、API key、密码、Cookie 写入 Git。
- 账号登录状态只用于本机 Profile 切换和额度查询。

## 技术栈

- Rust + Tauri 2
- TypeScript + Vite
- 原生 HTML/CSS 前端结构
- ChatGPT / Codex account metadata 读取
- GitHub Actions 多平台构建

## License

MIT License。完整文本见 [LICENSE](LICENSE)。
