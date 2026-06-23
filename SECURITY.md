# Security

## Security Goals

Codex Switch manages only local authentication state that the user has already created through official Codex login flows. The app must preserve account boundaries, avoid credential disclosure, and make failed switches recoverable.

## Non-Goals

- No password collection
- No browser cookie scraping
- No web login automation
- No MFA bypass
- No shared-account workaround
- No prompt, source-code, or repository-content logging

## Credential Storage

Sensitive values such as access tokens, refresh tokens, API keys, cookies, and complete auth payloads must not be stored in ordinary JSON files, frontend state, localStorage, logs, crash reports, or Git.

The backend will use a credential-store abstraction with these platform targets:

- macOS: Keychain
- Windows: Credential Manager
- Linux: Secret Service / keyring

Profile metadata can be stored locally if it contains only redacted account hints, labels, notes, support state, and timestamps. Secret profile payloads must be encrypted or stored through the system secure credential store.

Current implementation status:

- `ProfileMetadata` rejects unredacted email-style account hints.
- Available environment states must reference an opaque secret key, not inline auth content.
- `SecretStore` defines the secret persistence boundary.
- `KeychainSecretStore` uses the Rust `keyring` crate for the production OS credential backend.
- The import command returns counts and warnings only; raw captured file contents stay inside backend secret storage.
- Multi-environment import fails unless the caller confirms the selected local states belong to the same account.
- Unit tests use `MemorySecretStore` and do not touch real credentials.

## Threat Model

Primary risks:

- Accidental credential commit
- Token leakage through logs or UI state
- Partial switch leaving mixed account state
- Wrong-account restore after detector ambiguity
- Destructive process shutdown while user work is unsaved
- Local malware or hostile user with filesystem access

Controls:

- Privacy-focused `.gitignore`
- Redacted account identifiers only
- Transaction log with non-secret state only
- Timestamped backups before every switch
- Rollback on write, permission, validation, or restart failure
- Read-only real-environment detection mode
- Explicit user confirmation before process shutdown

The transaction runner stores backup manifests and transaction events without raw file contents. Restored contents are written only to the target filesystem paths supplied by a backend restore plan.

Desktop process handling prefers graceful application quit before restore. If the app cannot be confirmed stopped, restore does not proceed and the error includes the still-running process names.

CLI switching refuses to restore while matching Codex CLI processes are active. CLI validation reports availability separately from account identity so the app does not claim a verified account without evidence.

VS Code switching defaults to a manual Reload Window instruction to avoid closing unsaved editor work. Automatic restart is available through the adapter but remains an explicit configuration choice.

## Logging Policy

Allowed:

- Switch timestamp
- Source and target profile IDs or display names
- Environment list
- Success, failure, or rollback status
- Error category and non-secret diagnostic text

Forbidden:

- Tokens
- API keys
- Cookies
- Passwords
- Complete email addresses
- Prompt content
- Code file content
- Full auth payloads
