# Read-Only Environment Detection

The detector is intentionally conservative. It reports evidence found on the local machine, but it does not read token contents, parse full auth payloads, create probe files, mutate settings, or assume that a single file represents a complete profile.

## Reported Evidence

For each target environment, the backend returns:

- Confirmed executable or application path when found
- Candidate auth, config, and cache paths
- Whether each candidate path exists
- Permission confidence based on read-only filesystem metadata
- Matching running process names
- Redacted account hint, currently `Unknown`
- Support status: `detected`, `partial`, or `not-detected`

## Diagnostics Export

The Environment page can generate a copyable `environment-diagnostics/v1` JSON report from the same read-only scan. The report includes:

- OS and scan timestamp
- Installed/running/support status for CLI, VS Code, and Desktop
- Candidate app/auth/config/cache paths with home-directory shortening and email-like segment redaction
- Redacted account hints only
- Local permission summaries
- Notes that the report is read-only and does not include tokens, cookies, API keys, file contents, or unredacted emails

The diagnostics export is intended to help validate real-world Codex Desktop and VS Code candidate paths without adding hard-coded assumptions.

## Codex CLI

Signals:

- `codex` executable in `PATH`
- `CODEX_HOME` if set
- Existing `$HOME/.codex`
- Existing `$HOME/.codex/auth.json`
- Existing `$HOME/.codex/cache`
- Running process names containing `codex`

The detector does not read `auth.json` contents.

## VS Code

Signals:

- `code` executable in `PATH`
- Existing Visual Studio Code macOS app bundle
- Candidate global storage directories under Code and Code Insiders
- Existing child directories whose names contain `codex` or `openai`
- Running process names containing `code` or `visual studio code`

The detector does not claim that any extension storage folder is a complete profile until a later adapter validates it.

## Codex Desktop App

Signals:

- Existing app bundle candidates named Codex, Codex Desktop, or OpenAI Codex
- Candidate application support directories under common platform config roots
- Existing child directories whose names contain `codex` or `openai`
- Running process names containing `codex` or `codex desktop`

Known limitation: real Codex Desktop authentication paths have not been verified on this machine. The detector records candidates and unknowns without hard-coding a single auth-file assumption.
