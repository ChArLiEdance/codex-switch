use crate::errors::CommandError;
use crate::models::{
    ActionResponse, CodexPromptEntry, CodexPromptIdPayload, CodexSkillEntry, CodexSkillIdPayload,
    SaveCodexPromptPayload, SaveCodexSkillPayload,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri_plugin_opener::OpenerExt;

#[cfg(target_os = "macos")]
use crate::macos as platform_runtime;

#[cfg(not(target_os = "macos"))]
use crate::windows as platform_runtime;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
struct PromptIndex {
    schema_version: u32,
    prompts: BTreeMap<String, PromptMeta>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
struct PromptMeta {
    id: String,
    name: String,
    description: Option<String>,
    enabled: bool,
    created_at: Option<u64>,
    updated_at: Option<u64>,
}

#[tauri::command]
pub fn list_codex_skills() -> Result<Vec<CodexSkillEntry>, CommandError> {
    let codex_home = platform_runtime::paths::get_codex_home();
    let dir = codex_home.join("skills");
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut entries = Vec::new();
    for entry in fs::read_dir(&dir).map_err(|error| io_error("SKILL_LIST_FAILED", &dir, error))? {
        let entry = entry.map_err(|error| io_error("SKILL_LIST_FAILED", &dir, error))?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(id) = path.file_name().and_then(|name| name.to_str()).map(str::to_string) else {
            continue;
        };
        let skill_path = path.join("SKILL.md");
        if !skill_path.exists() {
            continue;
        }
        let content = fs::read_to_string(&skill_path)
            .map_err(|error| io_error("SKILL_READ_FAILED", &skill_path, error))?;
        let (name, description) = parse_skill_doc(&id, &content);
        entries.push(CodexSkillEntry {
            id,
            name,
            description,
            content,
            path: skill_path.to_string_lossy().into_owned(),
            updated_at: file_mtime_ms(&skill_path),
        });
    }

    entries.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(entries)
}

#[tauri::command]
pub fn save_codex_skill(payload: SaveCodexSkillPayload) -> Result<CodexSkillEntry, CommandError> {
    let name = payload.name.trim();
    if name.is_empty() {
        return Err(CommandError::new("SKILL_NAME_REQUIRED", "Skill name is required."));
    }

    let codex_home = platform_runtime::paths::get_codex_home();
    let skills_dir = codex_home.join("skills");
    fs::create_dir_all(&skills_dir)
        .map_err(|error| io_error("SKILL_CREATE_FAILED", &skills_dir, error))?;

    let id = payload
        .id
        .as_deref()
        .map(sanitize_id)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| unique_id(&skills_dir, name));
    let dir = skills_dir.join(&id);
    assert_inside(&skills_dir, &dir)?;
    fs::create_dir_all(&dir).map_err(|error| io_error("SKILL_CREATE_FAILED", &dir, error))?;
    let skill_path = dir.join("SKILL.md");
    let content = if payload.content.trim().is_empty() {
        default_skill_content(name, payload.description.as_deref())
    } else {
        payload.content
    };
    fs::write(&skill_path, content.as_bytes())
        .map_err(|error| io_error("SKILL_SAVE_FAILED", &skill_path, error))?;

    let (parsed_name, parsed_description) = parse_skill_doc(&id, &content);
    Ok(CodexSkillEntry {
        id,
        name: if parsed_name.trim().is_empty() {
            name.to_string()
        } else {
            parsed_name
        },
        description: parsed_description.or(payload.description),
        content,
        path: skill_path.to_string_lossy().into_owned(),
        updated_at: file_mtime_ms(&skill_path),
    })
}

#[tauri::command]
pub fn delete_codex_skill(payload: CodexSkillIdPayload) -> Result<ActionResponse, CommandError> {
    let codex_home = platform_runtime::paths::get_codex_home();
    let skills_dir = codex_home.join("skills");
    let id = sanitize_id(&payload.id);
    let dir = skills_dir.join(&id);
    assert_inside(&skills_dir, &dir)?;
    if dir.exists() {
        fs::remove_dir_all(&dir).map_err(|error| io_error("SKILL_DELETE_FAILED", &dir, error))?;
    }
    Ok(ActionResponse {
        ok: true,
        message: "Deleted Codex skill.".to_string(),
        path: Some(dir.to_string_lossy().into_owned()),
    })
}

#[tauri::command]
pub fn open_codex_skills_folder(app: tauri::AppHandle) -> Result<ActionResponse, CommandError> {
    let codex_home = platform_runtime::paths::get_codex_home();
    let dir = codex_home.join("skills");
    fs::create_dir_all(&dir).map_err(|error| io_error("SKILL_FOLDER_FAILED", &dir, error))?;
    let path = dir.to_string_lossy().into_owned();
    app.opener()
        .open_path(path.clone(), None::<&str>)
        .map_err(|error| CommandError::new("SKILL_FOLDER_OPEN_FAILED", error.to_string()))?;
    Ok(ActionResponse {
        ok: true,
        message: "Opened Codex skills folder.".to_string(),
        path: Some(path),
    })
}

#[tauri::command]
pub fn list_codex_prompts() -> Result<Vec<CodexPromptEntry>, CommandError> {
    let codex_home = platform_runtime::paths::get_codex_home();
    let prompts_dir = codex_home.join("prompts");
    fs::create_dir_all(&prompts_dir)
        .map_err(|error| io_error("PROMPT_FOLDER_FAILED", &prompts_dir, error))?;

    let mut index = load_prompt_index(&prompts_dir)?;
    import_agents_on_first_launch(&codex_home, &prompts_dir, &mut index)?;
    let mut changed = false;
    let mut seen = BTreeSet::new();

    for entry in fs::read_dir(&prompts_dir)
        .map_err(|error| io_error("PROMPT_LIST_FAILED", &prompts_dir, error))?
    {
        let entry = entry.map_err(|error| io_error("PROMPT_LIST_FAILED", &prompts_dir, error))?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }
        let Some(id) = path.file_stem().and_then(|name| name.to_str()).map(str::to_string) else {
            continue;
        };
        seen.insert(id.clone());
        if !index.prompts.contains_key(&id) {
            let content = fs::read_to_string(&path).unwrap_or_default();
            let (name, description) = parse_prompt_doc(&id, &content);
            let now = now_ms();
            index.prompts.insert(
                id.clone(),
                PromptMeta {
                    id,
                    name,
                    description,
                    enabled: false,
                    created_at: Some(now),
                    updated_at: file_mtime_ms(&path).or(Some(now)),
                },
            );
            changed = true;
        }
    }

    let missing: Vec<String> = index
        .prompts
        .keys()
        .filter(|id| !seen.contains(*id))
        .cloned()
        .collect();
    for id in missing {
        index.prompts.remove(&id);
        changed = true;
    }
    if changed {
        save_prompt_index(&prompts_dir, &index)?;
    }

    entries_from_prompt_index(&prompts_dir, &index)
}

#[tauri::command]
pub fn save_codex_prompt(
    payload: SaveCodexPromptPayload,
) -> Result<CodexPromptEntry, CommandError> {
    let name = payload.name.trim();
    if name.is_empty() {
        return Err(CommandError::new(
            "PROMPT_NAME_REQUIRED",
            "Prompt name is required.",
        ));
    }

    let codex_home = platform_runtime::paths::get_codex_home();
    let prompts_dir = codex_home.join("prompts");
    fs::create_dir_all(&prompts_dir)
        .map_err(|error| io_error("PROMPT_FOLDER_FAILED", &prompts_dir, error))?;
    let mut index = load_prompt_index(&prompts_dir)?;
    let id = payload
        .id
        .as_deref()
        .map(sanitize_id)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| unique_file_id(&prompts_dir, name, "md"));
    let path = prompts_dir.join(format!("{id}.md"));
    assert_inside(&prompts_dir, &path)?;
    let content = payload.content;
    fs::write(&path, content.as_bytes())
        .map_err(|error| io_error("PROMPT_SAVE_FAILED", &path, error))?;

    let now = now_ms();
    if payload.enabled {
        for prompt in index.prompts.values_mut() {
            prompt.enabled = false;
        }
        write_agents_prompt(&codex_home, &content)?;
    }

    let created_at = index.prompts.get(&id).and_then(|meta| meta.created_at).or(Some(now));
    index.prompts.insert(
        id.clone(),
        PromptMeta {
            id: id.clone(),
            name: name.to_string(),
            description: payload.description.clone().filter(|value| !value.trim().is_empty()),
            enabled: payload.enabled,
            created_at,
            updated_at: Some(now),
        },
    );
    save_prompt_index(&prompts_dir, &index)?;
    entries_from_prompt_index(&prompts_dir, &index)?
        .into_iter()
        .find(|entry| entry.id == id)
        .ok_or_else(|| CommandError::new("PROMPT_SAVE_FAILED", "Saved prompt was not found."))
}

#[tauri::command]
pub fn enable_codex_prompt(payload: CodexPromptIdPayload) -> Result<ActionResponse, CommandError> {
    let codex_home = platform_runtime::paths::get_codex_home();
    let prompts_dir = codex_home.join("prompts");
    let id = sanitize_id(&payload.id);
    let mut index = load_prompt_index(&prompts_dir)?;
    let path = prompts_dir.join(format!("{id}.md"));
    assert_inside(&prompts_dir, &path)?;
    if !path.exists() {
        return Err(CommandError::new("PROMPT_NOT_FOUND", "Prompt not found."));
    }
    let content =
        fs::read_to_string(&path).map_err(|error| io_error("PROMPT_READ_FAILED", &path, error))?;
    for prompt in index.prompts.values_mut() {
        prompt.enabled = false;
    }
    let meta = index
        .prompts
        .get_mut(&id)
        .ok_or_else(|| CommandError::new("PROMPT_NOT_FOUND", "Prompt not found."))?;
    meta.enabled = true;
    meta.updated_at = Some(now_ms());
    write_agents_prompt(&codex_home, &content)?;
    save_prompt_index(&prompts_dir, &index)?;
    Ok(ActionResponse {
        ok: true,
        message: "Enabled Codex prompt.".to_string(),
        path: Some(codex_home.join("AGENTS.md").to_string_lossy().into_owned()),
    })
}

#[tauri::command]
pub fn delete_codex_prompt(payload: CodexPromptIdPayload) -> Result<ActionResponse, CommandError> {
    let codex_home = platform_runtime::paths::get_codex_home();
    let prompts_dir = codex_home.join("prompts");
    let id = sanitize_id(&payload.id);
    let mut index = load_prompt_index(&prompts_dir)?;
    if index.prompts.get(&id).is_some_and(|meta| meta.enabled) {
        return Err(CommandError::new(
            "PROMPT_DELETE_ENABLED",
            "Cannot delete the enabled prompt.",
        ));
    }
    let path = prompts_dir.join(format!("{id}.md"));
    assert_inside(&prompts_dir, &path)?;
    if path.exists() {
        fs::remove_file(&path).map_err(|error| io_error("PROMPT_DELETE_FAILED", &path, error))?;
    }
    index.prompts.remove(&id);
    save_prompt_index(&prompts_dir, &index)?;
    Ok(ActionResponse {
        ok: true,
        message: "Deleted Codex prompt.".to_string(),
        path: Some(path.to_string_lossy().into_owned()),
    })
}

#[tauri::command]
pub fn import_codex_prompt_from_agents() -> Result<CodexPromptEntry, CommandError> {
    let codex_home = platform_runtime::paths::get_codex_home();
    let agents_path = codex_home.join("AGENTS.md");
    if !agents_path.exists() {
        return Err(CommandError::new(
            "PROMPT_AGENTS_MISSING",
            "AGENTS.md does not exist.",
        ));
    }
    let content = fs::read_to_string(&agents_path)
        .map_err(|error| io_error("PROMPT_IMPORT_FAILED", &agents_path, error))?;
    if content.trim().is_empty() {
        return Err(CommandError::new(
            "PROMPT_AGENTS_EMPTY",
            "AGENTS.md is empty.",
        ));
    }
    let now = now_ms();
    let payload = SaveCodexPromptPayload {
        id: Some(format!("imported-{now}")),
        name: format!("Imported AGENTS {}", now / 1000),
        description: Some("Imported from ~/.codex/AGENTS.md".to_string()),
        content,
        enabled: false,
    };
    save_codex_prompt(payload)
}

fn parse_skill_doc(id: &str, content: &str) -> (String, Option<String>) {
    let name = content
        .lines()
        .find_map(|line| line.strip_prefix("# ").map(str::trim))
        .filter(|value| !value.is_empty())
        .unwrap_or(id)
        .to_string();
    let description = parse_description(content);
    (name, description)
}

fn parse_prompt_doc(id: &str, content: &str) -> (String, Option<String>) {
    let name = content
        .lines()
        .find_map(|line| line.strip_prefix("# ").map(str::trim))
        .filter(|value| !value.is_empty())
        .unwrap_or(id)
        .to_string();
    let description = parse_description(content);
    (name, description)
}

fn parse_description(content: &str) -> Option<String> {
    let mut in_frontmatter = false;
    for (index, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if index == 0 && trimmed == "---" {
            in_frontmatter = true;
            continue;
        }
        if in_frontmatter {
            if trimmed == "---" {
                in_frontmatter = false;
            } else if let Some(value) = trimmed.strip_prefix("description:") {
                let cleaned = value.trim().trim_matches('"').trim_matches('\'');
                if !cleaned.is_empty() {
                    return Some(cleaned.to_string());
                }
            }
            continue;
        }
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("```") {
            continue;
        }
        return Some(trimmed.chars().take(180).collect());
    }
    None
}

fn default_skill_content(name: &str, description: Option<&str>) -> String {
    let mut content = format!("# {name}\n\n");
    if let Some(description) = description.filter(|value| !value.trim().is_empty()) {
        content.push_str(description.trim());
        content.push_str("\n\n");
    }
    content.push_str("Use this skill when the task matches the description above.\n");
    content
}

fn load_prompt_index(prompts_dir: &Path) -> Result<PromptIndex, CommandError> {
    let path = prompts_dir.join("index.json");
    if !path.exists() {
        return Ok(PromptIndex {
            schema_version: 1,
            prompts: BTreeMap::new(),
        });
    }
    let content =
        fs::read_to_string(&path).map_err(|error| io_error("PROMPT_INDEX_READ_FAILED", &path, error))?;
    serde_json::from_str::<PromptIndex>(&content).map_err(|error| {
        CommandError::new(
            "PROMPT_INDEX_PARSE_FAILED",
            format!("Failed to parse prompt index: {error}"),
        )
    })
}

fn save_prompt_index(prompts_dir: &Path, index: &PromptIndex) -> Result<(), CommandError> {
    fs::create_dir_all(prompts_dir)
        .map_err(|error| io_error("PROMPT_INDEX_SAVE_FAILED", prompts_dir, error))?;
    let path = prompts_dir.join("index.json");
    let content = serde_json::to_string_pretty(index).map_err(|error| {
        CommandError::new(
            "PROMPT_INDEX_SAVE_FAILED",
            format!("Failed to serialize prompt index: {error}"),
        )
    })?;
    fs::write(&path, content.as_bytes())
        .map_err(|error| io_error("PROMPT_INDEX_SAVE_FAILED", &path, error))
}

fn entries_from_prompt_index(
    prompts_dir: &Path,
    index: &PromptIndex,
) -> Result<Vec<CodexPromptEntry>, CommandError> {
    let mut entries = Vec::new();
    for (id, meta) in &index.prompts {
        let path = prompts_dir.join(format!("{id}.md"));
        if !path.exists() {
            continue;
        }
        let content = fs::read_to_string(&path)
            .map_err(|error| io_error("PROMPT_READ_FAILED", &path, error))?;
        entries.push(CodexPromptEntry {
            id: id.clone(),
            name: meta.name.clone(),
            description: meta.description.clone(),
            content,
            enabled: meta.enabled,
            path: path.to_string_lossy().into_owned(),
            created_at: meta.created_at,
            updated_at: meta.updated_at.or_else(|| file_mtime_ms(&path)),
        });
    }
    entries.sort_by(|a, b| match (b.enabled, a.enabled) {
        (true, false) => std::cmp::Ordering::Greater,
        (false, true) => std::cmp::Ordering::Less,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });
    Ok(entries)
}

fn import_agents_on_first_launch(
    codex_home: &Path,
    prompts_dir: &Path,
    index: &mut PromptIndex,
) -> Result<(), CommandError> {
    if !index.prompts.is_empty() {
        return Ok(());
    }
    let agents_path = codex_home.join("AGENTS.md");
    if !agents_path.exists() {
        return Ok(());
    }
    let content = fs::read_to_string(&agents_path)
        .map_err(|error| io_error("PROMPT_IMPORT_FAILED", &agents_path, error))?;
    if content.trim().is_empty() {
        return Ok(());
    }
    let now = now_ms();
    let id = "current-agents".to_string();
    let path = prompts_dir.join(format!("{id}.md"));
    fs::write(&path, content.as_bytes())
        .map_err(|error| io_error("PROMPT_IMPORT_FAILED", &path, error))?;
    index.prompts.insert(
        id.clone(),
        PromptMeta {
            id,
            name: "Current AGENTS.md".to_string(),
            description: Some("Automatically imported from ~/.codex/AGENTS.md".to_string()),
            enabled: true,
            created_at: Some(now),
            updated_at: Some(now),
        },
    );
    save_prompt_index(prompts_dir, index)
}

fn write_agents_prompt(codex_home: &Path, content: &str) -> Result<(), CommandError> {
    fs::create_dir_all(codex_home)
        .map_err(|error| io_error("PROMPT_AGENTS_SAVE_FAILED", codex_home, error))?;
    let agents_path = codex_home.join("AGENTS.md");
    fs::write(&agents_path, content.as_bytes())
        .map_err(|error| io_error("PROMPT_AGENTS_SAVE_FAILED", &agents_path, error))
}

fn sanitize_id(value: &str) -> String {
    let mut out = String::new();
    for ch in value.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if matches!(ch, '-' | '_' | ' ' | '.') {
            if !out.ends_with('-') {
                out.push('-');
            }
        }
    }
    out.trim_matches('-').to_string()
}

fn unique_id(base: &Path, name: &str) -> String {
    let mut id = sanitize_id(name);
    if id.is_empty() {
        id = format!("skill-{}", now_ms());
    }
    let mut candidate = id.clone();
    let mut suffix = 2;
    while base.join(&candidate).exists() {
        candidate = format!("{id}-{suffix}");
        suffix += 1;
    }
    candidate
}

fn unique_file_id(base: &Path, name: &str, extension: &str) -> String {
    let mut id = sanitize_id(name);
    if id.is_empty() {
        id = format!("prompt-{}", now_ms());
    }
    let mut candidate = id.clone();
    let mut suffix = 2;
    while base.join(format!("{candidate}.{extension}")).exists() {
        candidate = format!("{id}-{suffix}");
        suffix += 1;
    }
    candidate
}

fn assert_inside(base: &Path, path: &Path) -> Result<(), CommandError> {
    let base = normalize_for_compare(base);
    let path = normalize_for_compare(path);
    if path.starts_with(&base) {
        Ok(())
    } else {
        Err(CommandError::new(
            "INVALID_PATH",
            "Resolved path escaped the Codex home directory.",
        ))
    }
}

fn normalize_for_compare(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

fn file_mtime_ms(path: &Path) -> Option<u64> {
    fs::metadata(path)
        .ok()
        .and_then(|metadata| metadata.modified().ok())
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis() as u64)
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

fn io_error(code: &'static str, path: &Path, error: std::io::Error) -> CommandError {
    CommandError::new(code, format!("{}: {error}", path.to_string_lossy()))
}
