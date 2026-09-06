use crate::{
    model_http_client, parse_intranet_url,
    storage::{network_access_enabled, open_database, record_audit},
};
use rusqlite::{params, Connection};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{fs, path::Path};

const MAX_SKILL_BYTES: u64 = 256 * 1024;
const MAX_ENABLED_SKILL_BYTES: usize = 16 * 1024;
const MAX_SKILLS: usize = 200;
const MAX_SKILL_URL_LENGTH: usize = 2048;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillRecord {
    description: String,
    enabled: bool,
    has_scripts: bool,
    id: String,
    instructions: String,
    name: String,
    permissions: Vec<String>,
    restricted_permissions: bool,
    source_name: String,
    source_type: String,
    version: String,
}

fn validate_id(id: &str) -> Result<(), String> {
    let valid = id.starts_with("skill-")
        && id.len() <= 80
        && id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-');
    valid
        .then_some(())
        .ok_or_else(|| "Skill 标识无效。".to_string())
}

fn clean_metadata_value(value: &str, limit: usize) -> String {
    value
        .trim()
        .trim_matches(['"', '\''])
        .replace(['\r', '\n'], " ")
        .chars()
        .take(limit)
        .collect()
}

fn parse_permissions(value: &str) -> Result<Vec<String>, String> {
    let cleaned = value.trim().trim_matches(['[', ']']);
    let mut permissions = cleaned
        .split(',')
        .map(|item| clean_metadata_value(item, 60).to_ascii_lowercase())
        .filter(|item| !item.is_empty())
        .collect::<Vec<_>>();
    permissions.sort();
    permissions.dedup();
    if permissions.len() > 20
        || permissions.iter().any(|item| {
            !item.chars().all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | ':')
            })
        })
    {
        return Err("Skill 权限清单格式无效或超过 20 项。".to_string());
    }
    Ok(permissions)
}

fn parse_skill(
    markdown: &str,
    fallback_name: &str,
) -> Result<(String, String, String, String, Vec<String>), String> {
    let normalized = markdown.trim_start_matches('\u{feff}');
    let mut name = fallback_name.to_string();
    let mut description = "本地导入的说明型 Skill".to_string();
    let mut version = "unspecified".to_string();
    let mut permissions = vec!["prompt".to_string()];
    let instructions = if let Some(frontmatter) = normalized.strip_prefix("---\n") {
        if let Some((metadata, body)) = frontmatter.split_once("\n---\n") {
            for line in metadata.lines() {
                if let Some(value) = line.strip_prefix("name:") {
                    name = clean_metadata_value(value, 80);
                } else if let Some(value) = line.strip_prefix("description:") {
                    description = clean_metadata_value(value, 240);
                } else if let Some(value) = line.strip_prefix("version:") {
                    version = clean_metadata_value(value, 60);
                } else if let Some(value) = line.strip_prefix("permissions:") {
                    permissions = parse_permissions(value)?;
                }
            }
            body.trim().to_string()
        } else {
            return Err("SKILL.md 的 YAML 前置信息没有正确结束。".to_string());
        }
    } else {
        normalized.trim().to_string()
    };
    if name.is_empty() || instructions.is_empty() || instructions.len() > MAX_SKILL_BYTES as usize {
        return Err("Skill 名称或说明内容为空、过长。".to_string());
    }
    if version.is_empty() || permissions.is_empty() {
        return Err("Skill 版本和权限清单不能为空。".to_string());
    }
    Ok((name, description, instructions, version, permissions))
}

fn has_executable_content(directory: &Path) -> bool {
    fs::read_dir(directory)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .any(|entry| {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
            name == "scripts"
                || name == "bin"
                || path
                    .extension()
                    .and_then(|value| value.to_str())
                    .is_some_and(|extension| {
                        matches!(
                            extension.to_ascii_lowercase().as_str(),
                            "sh" | "bash"
                                | "zsh"
                                | "py"
                                | "js"
                                | "mjs"
                                | "cjs"
                                | "ps1"
                                | "exe"
                                | "bat"
                                | "cmd"
                        )
                    })
        })
}

fn selected_skill(path: &str) -> Result<(String, String, String, bool), String> {
    if path.is_empty() || path.len() > 4096 {
        return Err("Skill 路径无效。".to_string());
    }
    let selected = Path::new(path);
    let metadata =
        fs::symlink_metadata(selected).map_err(|_| "无法读取所选 Skill。".to_string())?;
    if metadata.file_type().is_symlink() {
        return Err("Skill 导入不接受符号链接。".to_string());
    }
    let (skill_file, source_type, has_scripts) = if metadata.is_dir() {
        (
            selected.join("SKILL.md"),
            "local-directory".to_string(),
            has_executable_content(selected),
        )
    } else if metadata.is_file()
        && selected.file_name().and_then(|value| value.to_str()) == Some("SKILL.md")
    {
        (selected.to_path_buf(), "local-file".to_string(), false)
    } else {
        return Err("请选择 SKILL.md 文件或包含该文件的目录。".to_string());
    };
    let skill_metadata =
        fs::symlink_metadata(&skill_file).map_err(|_| "所选目录缺少 SKILL.md。".to_string())?;
    if skill_metadata.file_type().is_symlink()
        || !skill_metadata.is_file()
        || skill_metadata.len() == 0
        || skill_metadata.len() > MAX_SKILL_BYTES
    {
        return Err("SKILL.md 必须是普通文件，且大小不超过 256 KiB。".to_string());
    }
    let bytes = fs::read(&skill_file).map_err(|_| "无法读取 SKILL.md。".to_string())?;
    let markdown =
        String::from_utf8(bytes).map_err(|_| "SKILL.md 必须使用 UTF-8 编码。".to_string())?;
    let source_name = selected
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("SKILL.md")
        .chars()
        .take(180)
        .collect();
    Ok((markdown, source_name, source_type, has_scripts))
}

fn load_one(connection: &Connection, id: &str) -> Result<SkillRecord, String> {
    connection
        .query_row(
            "SELECT id, name, description, instructions, source_name, enabled, has_scripts,
                    version, source_type, permissions_json, has_restricted_permissions
             FROM skills WHERE id = ?1",
            params![id],
            |row| {
                Ok(SkillRecord {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    instructions: row.get(3)?,
                    source_name: row.get(4)?,
                    enabled: row.get::<_, i64>(5)? != 0,
                    has_scripts: row.get::<_, i64>(6)? != 0,
                    version: row.get(7)?,
                    source_type: row.get(8)?,
                    permissions: serde_json::from_str(&row.get::<_, String>(9)?)
                        .unwrap_or_default(),
                    restricted_permissions: row.get::<_, i64>(10)? != 0,
                })
            },
        )
        .map_err(|_| "Skill 不存在或记录无效。".to_string())
}

fn save_imported_skill(
    app: &tauri::AppHandle,
    markdown: String,
    source_name: String,
    source_type: String,
    has_scripts: bool,
) -> Result<SkillRecord, String> {
    let fallback_name = Path::new(&source_name)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("本地 Skill");
    let (name, description, instructions, version, permissions) =
        parse_skill(&markdown, fallback_name)?;
    let restricted_permissions = permissions.iter().any(|permission| permission != "prompt");
    let permissions_json =
        serde_json::to_string(&permissions).map_err(|_| "无法编码 Skill 权限清单。".to_string())?;
    let digest = Sha256::digest(markdown.as_bytes());
    let id = format!(
        "skill-{}",
        digest[..12]
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    );
    let connection = open_database(&app)?;
    let count = connection
        .query_row("SELECT COUNT(*) FROM skills", [], |row| {
            row.get::<_, i64>(0)
        })
        .map_err(|_| "无法读取 Skill 数量。".to_string())? as usize;
    if count >= MAX_SKILLS
        && !connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM skills WHERE id = ?1)",
                params![id],
                |row| row.get::<_, bool>(0),
            )
            .unwrap_or(false)
    {
        return Err("本地 Skill 数量已达到 200 个上限。".to_string());
    }
    connection
        .execute(
            "INSERT INTO skills
             (id, name, description, instructions, source_name, enabled, has_scripts,
              version, source_type, permissions_json, has_restricted_permissions)
             VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(id) DO UPDATE SET
               name = excluded.name,
               description = excluded.description,
               instructions = excluded.instructions,
               source_name = excluded.source_name,
               enabled = 0,
               has_scripts = excluded.has_scripts,
               version = excluded.version,
               source_type = excluded.source_type,
               permissions_json = excluded.permissions_json,
               has_restricted_permissions = excluded.has_restricted_permissions,
               updated_at = unixepoch()",
            params![
                id,
                name,
                description,
                instructions,
                source_name,
                has_scripts as i64,
                version,
                source_type,
                permissions_json,
                restricted_permissions as i64
            ],
        )
        .map_err(|_| "无法保存本地 Skill。".to_string())?;
    load_one(&connection, &id)
}

#[tauri::command]
pub fn import_skill(app: tauri::AppHandle, path: String) -> Result<SkillRecord, String> {
    let (markdown, source_name, source_type, has_scripts) = selected_skill(&path)?;
    save_imported_skill(&app, markdown, source_name, source_type, has_scripts)
}

fn validate_skill_url(source_url: &str) -> Result<url::Url, String> {
    if source_url.is_empty() || source_url.len() > MAX_SKILL_URL_LENGTH {
        return Err("Skill 地址为空或超过 2048 字符。".to_string());
    }
    let url = parse_intranet_url(source_url).map_err(str::to_string)?;
    if url.query().is_some() || url.fragment().is_some() {
        return Err("Skill 地址不能包含查询参数或片段。".to_string());
    }
    if !url
        .path_segments()
        .and_then(Iterator::last)
        .is_some_and(|name| name.eq_ignore_ascii_case("SKILL.md"))
    {
        return Err("内网 Skill 地址必须明确指向 SKILL.md。".to_string());
    }
    Ok(url)
}

async fn download_skill(source_url: &str) -> Result<(String, String), String> {
    let url = validate_skill_url(source_url)?;
    let source_name = format!(
        "{}/SKILL.md",
        url.host_str()
            .unwrap_or("intranet")
            .chars()
            .take(253)
            .collect::<String>()
    );
    let mut response = model_http_client(&url)?
        .get(url)
        .header("accept", "text/markdown, text/plain;q=0.9")
        .send()
        .await
        .map_err(|_| "无法下载内网 SKILL.md，请检查地址、证书和服务状态。".to_string())?;
    if !response.status().is_success() {
        return Err(format!(
            "内网 Skill 服务返回 HTTP {}。",
            response.status().as_u16()
        ));
    }
    if response
        .content_length()
        .is_some_and(|length| length == 0 || length > MAX_SKILL_BYTES)
    {
        return Err("内网 SKILL.md 为空或超过 256 KiB。".to_string());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "读取内网 SKILL.md 时连接中断。".to_string())?
    {
        if bytes.len().saturating_add(chunk.len()) > MAX_SKILL_BYTES as usize {
            return Err("内网 SKILL.md 超过 256 KiB。".to_string());
        }
        bytes.extend_from_slice(&chunk);
    }
    if bytes.is_empty() {
        return Err("内网 SKILL.md 为空。".to_string());
    }
    let markdown =
        String::from_utf8(bytes).map_err(|_| "内网 SKILL.md 必须使用 UTF-8 编码。".to_string())?;
    Ok((markdown, source_name))
}

#[tauri::command]
pub async fn import_skill_url(
    app: tauri::AppHandle,
    source_url: String,
) -> Result<SkillRecord, String> {
    if !network_access_enabled(&app)? {
        record_audit(&app, "skill.intranet-import", "blocked");
        return Err("网络访问总开关已关闭；未连接内网 Skill 来源。".to_string());
    }
    let result = match download_skill(&source_url).await {
        Ok((markdown, source_name)) => save_imported_skill(
            &app,
            markdown,
            source_name,
            "intranet-url".to_string(),
            false,
        ),
        Err(error) => Err(error),
    };
    record_audit(
        &app,
        "skill.intranet-import",
        if result.is_ok() { "success" } else { "failed" },
    );
    result
}

#[tauri::command]
pub fn list_skills(app: tauri::AppHandle) -> Result<Vec<SkillRecord>, String> {
    let connection = open_database(&app)?;
    let mut query = connection
        .prepare(
            "SELECT id, name, description, instructions, source_name, enabled, has_scripts,
                    version, source_type, permissions_json, has_restricted_permissions
             FROM skills ORDER BY updated_at DESC LIMIT ?1",
        )
        .map_err(|_| "无法读取 Skills。".to_string())?;
    let skills = query
        .query_map(params![MAX_SKILLS as i64], |row| {
            Ok(SkillRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                instructions: row.get(3)?,
                source_name: row.get(4)?,
                enabled: row.get::<_, i64>(5)? != 0,
                has_scripts: row.get::<_, i64>(6)? != 0,
                version: row.get(7)?,
                source_type: row.get(8)?,
                permissions: serde_json::from_str(&row.get::<_, String>(9)?).unwrap_or_default(),
                restricted_permissions: row.get::<_, i64>(10)? != 0,
            })
        })
        .map_err(|_| "无法读取 Skills。".to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "Skill 记录格式无效。".to_string())?;
    Ok(skills)
}

#[tauri::command]
pub fn set_skill_enabled(
    app: tauri::AppHandle,
    skill_id: String,
    enabled: bool,
) -> Result<SkillRecord, String> {
    validate_id(&skill_id)?;
    let connection = open_database(&app)?;
    let skill = load_one(&connection, &skill_id)?;
    if enabled && (skill.has_scripts || skill.restricted_permissions) {
        return Err("包含脚本或受限权限声明的 Skill 在首版中不能启用。".to_string());
    }
    if enabled && skill.instructions.len() > MAX_ENABLED_SKILL_BYTES {
        return Err("Skill 说明超过 16 KiB，首版不能启用；可先精简后重新导入。".to_string());
    }
    connection
        .execute(
            "UPDATE skills SET enabled = ?1, updated_at = unixepoch() WHERE id = ?2",
            params![enabled as i64, skill_id],
        )
        .map_err(|_| "无法更新 Skill 状态。".to_string())?;
    load_one(&connection, &skill_id)
}

#[tauri::command]
pub fn remove_skill(app: tauri::AppHandle, skill_id: String) -> Result<(), String> {
    validate_id(&skill_id)?;
    open_database(&app)?
        .execute("DELETE FROM skills WHERE id = ?1", params![skill_id])
        .map(|_| ())
        .map_err(|_| "无法移除 Skill 记录。".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_prompt_only_skill_metadata() {
        let (name, description, instructions, version, permissions) = parse_skill(
            "---\nname: analog-review\ndescription: 检查模拟电路说明\nversion: 1.2.0\npermissions: [prompt]\n---\n只根据输入内容检查单位。",
            "fallback",
        )
        .expect("skill should parse");
        assert_eq!(name, "analog-review");
        assert_eq!(description, "检查模拟电路说明");
        assert_eq!(instructions, "只根据输入内容检查单位。");
        assert_eq!(version, "1.2.0");
        assert_eq!(permissions, vec!["prompt"]);
    }

    #[test]
    fn rejects_invalid_frontmatter_and_identifiers() {
        assert!(parse_skill("---\nname: incomplete", "fallback").is_err());
        assert!(parse_permissions("[prompt, ../../shell]").is_err());
        assert!(validate_id("../skill").is_err());
    }

    #[test]
    fn intranet_skill_urls_are_explicit_and_bounded() {
        assert!(validate_skill_url("https://skills.internal/analog/SKILL.md").is_ok());
        assert!(validate_skill_url("http://127.0.0.1:18081/SKILL.md").is_ok());
        assert!(validate_skill_url("https://example.com/SKILL.md").is_err());
        assert!(validate_skill_url("https://skills.internal/analog/readme.md").is_err());
        assert!(validate_skill_url("https://skills.internal/SKILL.md?token=secret").is_err());
    }
}
