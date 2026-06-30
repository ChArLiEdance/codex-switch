# Codex Switch

<p align="center">
  <a href="README.md">English</a> |
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

**Codex Switch** 是一个用于管理多个 OpenAI Codex 本地登录 Profile 的桌面应用。它可以查看当前账号、切换已保存的本地账号、刷新额度信息，并让每个账号的 Codex 本地状态相互隔离。

`1.1.3` 版本关闭切换账号前的健康检查弹窗，让账号切换流程更直接，并继续保留优化后的 macOS 和 Windows 状态栏体验、可用的 **技能** 和 **提示词** 管理流程、账号切换、登录、额度、设置、使用统计和会话记录等基础能力。

> 本项目不是 OpenAI 官方项目。它只管理本机已经通过官方流程授权的 Codex 登录状态，不收集密码，不绕过 MFA，不抓取浏览器 Cookie，也不提供共享账号能力。

## 软件特色

- **账号卡片**：展示昵称、账号详细名称、套餐标记、当前登录状态、5 小时额度和周额度。
- **账号管理**：添加、登录、切换、重命名、删除和拖拽排序本地 Codex 账号 Profile。
- **额度查询**：刷新并展开账号额度详情，支持按账号配置用量查询设置。
- **使用统计**：读取本地 Codex 会话用量，汇总 token 消耗，并展示用量趋势。
- **会话记录**：浏览本地 Codex 会话，查看历史对话并恢复会话。
- **技能和提示词**：管理本地 Codex 技能和可复用提示词，并支持中英文界面标签。
- **设置页面**：切换语言、选择白色/黑色/跟随系统主题，配置更新地址和 Codex CLI 路径。
- **本地隐私优先**：使用本机 Codex/OAuth 状态，不把密码或浏览器 Cookie 写入仓库。

## 安装方式

请从 [GitHub Releases](https://github.com/ChArLiEdance/codex-switch/releases) 下载安装包。当前 `1.1.3` 版本提供 macOS Apple Silicon 和 Windows x64 安装包。

### macOS

1. 下载 [`codex_switch_1.1.3_aarch64.dmg`](https://github.com/ChArLiEdance/codex-switch/releases/download/v1.1.3/codex_switch_1.1.3_aarch64.dmg)。
2. 打开 `.dmg`。
3. 将 `codex_switch.app` 拖入 `Applications`。
4. 启动 Codex Switch。

也可以在终端直接下载：

```bash
curl -L -o ~/Downloads/codex_switch_1.1.3_aarch64.dmg \
  https://github.com/ChArLiEdance/codex-switch/releases/download/v1.1.3/codex_switch_1.1.3_aarch64.dmg
```

如果你更想使用安装器，可以下载 [`codex_switch_1.1.3_aarch64.pkg`](https://github.com/ChArLiEdance/codex-switch/releases/download/v1.1.3/codex_switch_1.1.3_aarch64.pkg)，然后双击打开安装。

当前版本不提供 macOS x64 安装包。

当前安装包会根据构建环境使用未签名或 ad-hoc 签名。首次打开如果被 macOS 拦截，可以到「系统设置 -> 隐私与安全」中允许打开。

### Windows

1. 下载 [`codex_switch_1.1.3_x64-setup.exe`](https://github.com/ChArLiEdance/codex-switch/releases/download/v1.1.3/codex_switch_1.1.3_x64-setup.exe)。
2. 运行安装程序。
3. 从开始菜单或桌面快捷方式打开 Codex Switch。

也可以在 PowerShell 直接下载：

```powershell
Invoke-WebRequest `
  -Uri "https://github.com/ChArLiEdance/codex-switch/releases/download/v1.1.3/codex_switch_1.1.3_x64-setup.exe" `
  -OutFile "$env:USERPROFILE\Downloads\codex_switch_1.1.3_x64-setup.exe"
```

Windows 版本使用 Tauri NSIS 安装器。

## 使用方式

1. 先安装并确认本机 Codex CLI 可用。
2. 打开 Codex Switch。
3. 进入设置页面，确认 Codex CLI 路径正确。
4. 点击右上角加号添加账号 Profile。
5. 点击登录，按官方浏览器 OAuth 流程完成授权。
6. 刷新账号卡片，加载套餐和额度信息。
7. 添加第二个账号后，可以点击切换，在不同本地账号状态之间切换。

Codex Switch 主要围绕这些本地目录工作：

```text
~/.codex/
~/.codex/account_backup/
~/.codex-switch/
```

## GitHub 仓库目录

当前仓库结构：

```text
codex-switch/
  .github/workflows/      GitHub Actions 构建和发布流程
  macOS-backup/           旧版 shell 账号切换脚本备份
  scripts/                版本同步、macOS 产物整理和安装包辅助脚本
  src-tauri/
    capabilities/         Tauri 权限能力配置
    icons/                macOS 和 Windows 应用图标
    mac/                  macOS 前端壳和平台 runtime
    shared/               共享前端、Tauri command、元数据、额度、历史记录和切换逻辑
    src/                  Tauri Rust 入口
    win/                  Windows 前端壳和平台 runtime
    Cargo.toml            Rust crate 配置
    tauri.conf.json       基础 Tauri 配置
    tauri.macos.conf.json macOS 打包目标
    tauri.windows.conf.json Windows NSIS 安装包目标
  CHANGELOG.md
  LICENSE
  README.md
  README.zh-CN.md
  package.json
  package-lock.json
  tsconfig.json
  vite.config.ts
```

`dist/`、`node_modules/`、`src-tauri/target/` 等生成目录不会提交到 Git。

## 开发命令

```bash
npm install
npm run build
npm run test:rust
npm run tauri:dev
```

在浏览器里预览 Windows UI，并使用 mock Tauri 命令：

```bash
npm run dev:windows-preview
```

然后打开 `http://127.0.0.1:1421`。这个预览模式使用模拟账号、额度、用量统计、会话记录、设置和账号操作，不会读取或写入真实 Codex 凭证，也不会切换本地账号。

不启动 dev server，只构建静态 Windows 预览产物：

```bash
npm run build:windows-preview
```

静态输出目录是 `dist/windows-preview`，和正式 Tauri 前端输出 `dist/web` 分开。

本机构建 macOS 安装包：

```bash
npm run tauri:build:macos-release
```

Windows 安装包建议在 Windows runner 上构建：

```bash
npm run tauri:build:windows
```

## 发布打包

安装包应该作为 GitHub Release assets 上传，不应该提交到源码仓库，也不应该放进 `package.json`。

`1.1.3` 版本当前发布产物：

```text
codex_switch_1.1.3_aarch64.dmg
codex_switch_1.1.3_aarch64.pkg
codex_switch_1.1.3_x64-setup.exe
```

仓库已经配置 GitHub Actions，可以根据版本 tag 构建发布产物。`1.1.3` 的公开 Release 不包含 macOS x64。

## 隐私与安全

- 不收集 OpenAI 密码。
- 不自动化网页登录。
- 不绕过 MFA。
- 不抓取浏览器 Cookie。
- 不把 token、API key、密码、Cookie 写入 Git。
- 登录状态只用于本机 Profile 切换和额度查询。

## 技术栈

- Rust + Tauri 2
- TypeScript + Vite
- 原生 HTML/CSS 前端
- 本地 Codex 账号元数据和额度读取
- GitHub Actions 多平台构建

## License

MIT License。完整文本见 [LICENSE](LICENSE)。
