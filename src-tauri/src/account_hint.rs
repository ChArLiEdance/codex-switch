use base64::{engine::general_purpose, Engine as _};
use serde_json::Value;
use std::{fs, path::Path};

pub fn redacted_account_hint_from_path(path: &Path) -> Option<String> {
    let mut budget = AccountHintBudget {
        files_remaining: 20,
        bytes_remaining: 512 * 1024,
        max_depth: 3,
    };
    redacted_account_hint_from_path_with_budget(path, 0, &mut budget)
}

fn redacted_account_hint_from_path_with_budget(
    path: &Path,
    depth: usize,
    budget: &mut AccountHintBudget,
) -> Option<String> {
    if budget.files_remaining == 0 || budget.bytes_remaining == 0 || depth > budget.max_depth {
        return None;
    }
    let metadata = fs::symlink_metadata(path).ok()?;
    if metadata.file_type().is_symlink() {
        return None;
    }
    if metadata.is_dir() {
        let entries = fs::read_dir(path).ok()?;
        for entry in entries.filter_map(Result::ok) {
            if let Some(hint) =
                redacted_account_hint_from_path_with_budget(&entry.path(), depth + 1, budget)
            {
                return Some(hint);
            }
        }
        return None;
    }
    if !metadata.is_file() {
        return None;
    }
    let size = metadata.len() as usize;
    if size == 0 || size > budget.bytes_remaining || size > 128 * 1024 {
        return None;
    }
    budget.files_remaining = budget.files_remaining.saturating_sub(1);
    budget.bytes_remaining = budget.bytes_remaining.saturating_sub(size);
    let content = fs::read_to_string(path).ok()?;
    redacted_account_hint_from_content(&content)
}

pub fn redacted_account_hint_from_content(content: &str) -> Option<String> {
    if let Ok(value) = serde_json::from_str::<Value>(content) {
        if let Some(email) = email_from_oauth_tokens(&value) {
            return Some(redact_email(&email));
        }
        if let Some(email) = email_from_json_value(&value) {
            return Some(redact_email(&email));
        }
    }
    first_user_email_like(content).map(|email| redact_email(&email))
}

pub fn redact_email_like_text(content: &str) -> String {
    let mut redacted = String::with_capacity(content.len());
    let mut candidate = String::new();
    for character in content.chars() {
        if character.is_ascii_alphanumeric() || matches!(character, '@' | '.' | '_' | '-' | '+') {
            candidate.push(character);
        } else {
            push_redacted_candidate(&mut redacted, &mut candidate);
            redacted.push(character);
        }
    }
    push_redacted_candidate(&mut redacted, &mut candidate);
    redacted
}

fn push_redacted_candidate(output: &mut String, candidate: &mut String) {
    if candidate.is_empty() {
        return;
    }
    if is_email_like(candidate) {
        output.push_str(&redact_email(candidate));
    } else {
        output.push_str(candidate);
    }
    candidate.clear();
}

fn email_from_json_value(value: &Value) -> Option<String> {
    match value {
        Value::Object(map) => {
            for (key, value) in map {
                let key = key.to_ascii_lowercase();
                if matches!(
                    key.as_str(),
                    "email"
                        | "account"
                        | "account_email"
                        | "user"
                        | "username"
                        | "login"
                        | "profile"
                ) {
                    if let Value::String(text) = value {
                        if let Some(email) = first_user_email_like(text) {
                            return Some(email);
                        }
                    }
                }
            }
            for value in map.values() {
                if let Some(email) = email_from_json_value(value) {
                    return Some(email);
                }
            }
            None
        }
        Value::Array(values) => values.iter().find_map(email_from_json_value),
        Value::String(text) => first_user_email_like(text),
        _ => None,
    }
}

fn email_from_oauth_tokens(value: &Value) -> Option<String> {
    let tokens = value.get("tokens").unwrap_or(value);
    ["id_token", "access_token"].iter().find_map(|field| {
        tokens
            .get(*field)
            .and_then(Value::as_str)
            .and_then(email_from_jwt)
    })
}

fn email_from_jwt(token: &str) -> Option<String> {
    let payload = token.split('.').nth(1)?;
    let decoded = general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .or_else(|_| general_purpose::URL_SAFE.decode(payload))
        .ok()?;
    let value: Value = serde_json::from_slice(&decoded).ok()?;
    value
        .get("email")
        .and_then(Value::as_str)
        .and_then(first_user_email_like)
}

fn first_user_email_like(content: &str) -> Option<String> {
    for token in content.split(|character: char| {
        character.is_whitespace()
            || matches!(
                character,
                '"' | '\'' | '<' | '>' | '(' | ')' | '[' | ']' | '{' | '}' | ',' | ';'
            )
    }) {
        let candidate = token.trim_matches(|character: char| {
            !character.is_ascii_alphanumeric() && !matches!(character, '@' | '.' | '_' | '-' | '+')
        });
        if is_email_like(candidate) && !is_service_email(candidate) {
            return Some(candidate.to_string());
        }
    }
    None
}

fn is_service_email(email: &str) -> bool {
    let Some((local, domain)) = email.split_once('@') else {
        return false;
    };
    let local = local.to_ascii_lowercase();
    let domain = domain.to_ascii_lowercase();
    domain == "openai.com"
        && matches!(
            local.as_str(),
            "support" | "security" | "noreply" | "no-reply" | "help" | "privacy"
        )
}

fn is_email_like(value: &str) -> bool {
    let Some((local, domain)) = value.split_once('@') else {
        return false;
    };
    !local.is_empty()
        && local.len() <= 128
        && domain.contains('.')
        && domain.len() <= 255
        && local.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-' | '+')
        })
        && domain
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '.' | '-'))
}

fn redact_email(email: &str) -> String {
    let Some((local, domain)) = email.split_once('@') else {
        return "Unknown".to_string();
    };
    let first = local.chars().next().unwrap_or('*');
    format!("{first}***@{domain}")
}

struct AccountHintBudget {
    files_remaining: usize,
    bytes_remaining: usize,
    max_depth: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        env, fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn redacts_email_from_json_account_hint() {
        let content = r#"{"auth":{"email":"charlie@example.com","access_token":"secret"}}"#;

        assert_eq!(
            redacted_account_hint_from_content(content),
            Some("c***@example.com".to_string())
        );
    }

    #[test]
    fn redacts_email_from_text_fallback() {
        let content = "signed in as user.name+codex@example.org";

        assert_eq!(
            redacted_account_hint_from_content(content),
            Some("u***@example.org".to_string())
        );
    }

    #[test]
    fn redacts_email_like_tokens_in_diagnostic_text() {
        let content = "path=/Users/person@example.com/.codex owner <other.user@example.org>";

        assert_eq!(
            redact_email_like_text(content),
            "path=/Users/p***@example.com/.codex owner <o***@example.org>"
        );
    }

    #[test]
    fn reads_bounded_redacted_hint_from_path() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let root = env::temp_dir().join(format!(
            "codex-switch-account-hint-{}-{unique}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create hint dir");
        let auth_path = root.join("auth.json");
        fs::write(&auth_path, r#"{"email":"person@example.com"}"#).expect("write auth");

        assert_eq!(
            redacted_account_hint_from_path(&auth_path),
            Some("p***@example.com".to_string())
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn redacts_email_from_oauth_id_token_claim() {
        let payload =
            general_purpose::URL_SAFE_NO_PAD.encode(r#"{"email":"jwt.user@example.net"}"#);
        let content = format!(r#"{{"tokens":{{"id_token":"header.{payload}.signature"}}}}"#);

        assert_eq!(
            redacted_account_hint_from_content(&content),
            Some("j***@example.net".to_string())
        );
    }

    #[test]
    fn prefers_oauth_claim_over_openai_service_email() {
        let payload = general_purpose::URL_SAFE_NO_PAD.encode(r#"{"email":"real.user@example.net"}"#);
        let content = format!(
            r#"{{
                "support":"support@openai.com",
                "tokens":{{"id_token":"header.{payload}.signature"}}
            }}"#
        );

        assert_eq!(
            redacted_account_hint_from_content(&content),
            Some("r***@example.net".to_string())
        );
    }

    #[test]
    fn ignores_openai_service_email_when_no_user_identity_exists() {
        let content = r#"{"support":"support@openai.com","help":"security@openai.com"}"#;

        assert_eq!(redacted_account_hint_from_content(content), None);
    }

    #[test]
    #[ignore]
    fn redacts_real_account_hint_fixtures_from_env() {
        let fixture_paths = env::var_os("CODEX_SWITCH_ACCOUNT_HINT_FIXTURES")
            .expect("set CODEX_SWITCH_ACCOUNT_HINT_FIXTURES to auth fixture directories");
        let hints: Vec<String> = env::split_paths(&fixture_paths)
            .map(|path| {
                let hint = redacted_account_hint_from_path(&path)
                    .unwrap_or_else(|| panic!("no account hint extracted from {}", path.display()));
                assert_ne!(hint, "Unknown");
                if hint.contains('@') {
                    assert!(hint.contains("***"));
                }
                hint
            })
            .collect();

        assert!(hints.len() >= 2);
        assert_ne!(hints[0], hints[1]);
    }
}
