//! Local drafts and immutable source snapshots. No network or executable templates.
use crate::{
    document::{convert_document_bytes, source_file, ConvertedDocument},
    storage::open_database,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Component, Path};

fn state_key(key: &str) -> Result<(), String> {
    matches!(
        key,
        "note-draft"
            | "chat-draft"
            | "mail-draft"
            | "reader"
            | "project-view"
            | "templates"
            | "navigation"
            | "task-draft"
    )
    .then_some(())
    .ok_or_else(|| "不支持的工作状态。".into())
}
#[tauri::command]
pub fn load_workspace_state(
    app: tauri::AppHandle,
    key: String,
) -> Result<Option<serde_json::Value>, String> {
    state_key(&key)?;
    let raw: Option<String> = open_database(&app)?
        .query_row(
            "SELECT value FROM workspace_state WHERE key=?1",
            [&key],
            |r| r.get(0),
        )
        .optional()
        .map_err(|_| "无法读取工作草稿。")?;
    raw.map(|s| serde_json::from_str(&s).map_err(|_| "工作草稿格式无效，请从备份恢复。".into()))
        .transpose()
}
#[tauri::command]
pub fn save_workspace_state(
    app: tauri::AppHandle,
    key: String,
    value: serde_json::Value,
) -> Result<(), String> {
    state_key(&key)?;
    let raw = serde_json::to_string(&value).map_err(|_| "草稿格式无效。")?;
    if raw.len() > 24 * 1024 * 1024 {
        return Err("草稿超过 24 MiB，请减少附件。".into());
    }
    open_database(&app)?.execute("INSERT INTO workspace_state(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value,updated_at=unixepoch()", params![key,raw]).map_err(|_| "草稿保存失败，请保留当前页面并重试。")?;
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewSource {
    pub id: String,
    pub document: ConvertedDocument,
    pub pdf_url: Option<String>,
}

fn source_from_bytes(
    name: &str,
    bytes: &[u8],
) -> Result<(ConvertedDocument, Option<String>), String> {
    let extension = Path::new(name)
        .extension()
        .and_then(|v| v.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let document = convert_document_bytes(name, &extension, bytes)?;
    let pdf_url = (extension == "pdf")
        .then(|| format!("data:application/pdf;base64,{}", STANDARD.encode(bytes)));
    Ok((document, pdf_url))
}
fn read_source(
    path: Option<&str>,
) -> Result<(ConvertedDocument, Option<String>, Option<String>), String> {
    match path {
        Some(path) => {
            let (canonical, bytes) = source_file(path)?;
            // Reject a replaced parent directory as well as a replaced leaf.
            if !crate::file_safety::unchanged_location(Path::new(path), &canonical) {
                return Err("原件路径发生变化或包含符号链接，请重新选择普通文件。".into());
            }
            let name = canonical
                .file_name()
                .and_then(|v| v.to_str())
                .ok_or("文件名无效。")?;
            let (document, url) = source_from_bytes(name, &bytes)?;
            Ok((
                document,
                url,
                Some(canonical.to_string_lossy().into_owned()),
            ))
        }
        None if crate::is_demo_mode() => {
            let (document, url) = source_from_bytes(
                "synthetic_analog_ic_datasheet.pdf",
                include_bytes!("../../../../demo/samples/synthetic_analog_ic_datasheet.pdf"),
            )?;
            Ok((document, url, None))
        }
        None => Err("请选择本地资料。".into()),
    }
}
#[tauri::command]
pub fn register_review_source(
    app: tauri::AppHandle,
    path: Option<String>,
) -> Result<ReviewSource, String> {
    let (document, pdf_url, path) = read_source(path.as_deref())?;
    let id = format!(
        "source-{:x}",
        Sha256::digest(format!("{}|{}", path.as_deref().unwrap_or("demo"), document.id).as_bytes())
    );
    let json = serde_json::to_string(&document).map_err(|_| "无法保存来源。")?;
    open_database(&app)?
        .execute(
            "INSERT OR IGNORE INTO review_sources(id,path,document_json) VALUES(?1,?2,?3)",
            params![id, path, json],
        )
        .map_err(|_| "无法记录来源版本。")?;
    Ok(ReviewSource {
        id,
        document,
        pdf_url,
    })
}
#[tauri::command]
pub fn register_project_review_source(
    app: tauri::AppHandle,
    project_id: String,
    relative_path: String,
) -> Result<ReviewSource, String> {
    if relative_path.is_empty()
        || Path::new(&relative_path)
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err("项目相对路径无效。".into());
    }
    let connection = open_database(&app)?;
    let root: String = connection.query_row("SELECT p.root_path FROM projects p JOIN project_files f ON f.project_id=p.id WHERE p.id=?1 AND f.relative_path=?2",params![project_id,relative_path],|r|r.get(0)).map_err(|_| "未授权的项目资料。")?;
    let path = Path::new(&root).join(relative_path);
    register_review_source(app, Some(path.to_string_lossy().into_owned()))
}
#[tauri::command]
pub fn load_review_source(
    app: tauri::AppHandle,
    source_id: String,
) -> Result<ReviewSource, String> {
    let connection = open_database(&app)?;
    let (path, json): (Option<String>, String) = connection
        .query_row(
            "SELECT path,document_json FROM review_sources WHERE id=?1",
            [&source_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|_| "来源记录不存在，请重新导入。")?;
    let saved: ConvertedDocument = serde_json::from_str(&json).map_err(|_| "来源记录无效。")?;
    resolve_source(source_id, path.as_deref(), &saved)
}

fn resolve_source(
    source_id: String,
    path: Option<&str>,
    saved: &ConvertedDocument,
) -> Result<ReviewSource, String> {
    let (document, pdf_url, _) =
        read_source(path).map_err(|e| format!("原件不可用：{e} 笔记中的历史摘录仍保留。"))?;
    if document.id != saved.id {
        return Err(
            "原件内容已变化，不能按旧版本定位。历史摘录仍保留；请重新导入并复核引用。".into(),
        );
    }
    Ok(ReviewSource {
        id: source_id,
        document,
        pdf_url,
    })
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Citation {
    pub source_id: String,
    pub source_name: String,
    pub version: String,
    pub page: usize,
    pub quote: String,
    #[serde(default)]
    pub source_kind: Option<String>,
}
fn compact(value: &str) -> String {
    value.chars().filter(|c| !c.is_whitespace()).collect()
}
pub(crate) fn page_text(document: &ConvertedDocument, page: usize) -> Option<String> {
    if document.source_kind != "PDF" {
        return (page == 1).then(|| document.markdown.clone());
    }
    if page == 0 || page > document.page_count.unwrap_or(0) {
        return None;
    }
    let heading = format!("## 第 {page} 页\n");
    let text = document.markdown.split_once(&heading)?.1;
    Some(
        text.split(&format!("## 第 {} 页\n", page + 1))
            .next()?
            .to_string(),
    )
}
pub(crate) fn validate_citations(
    connection: &Connection,
    citations: &[Citation],
) -> Result<(), String> {
    if citations.len() > 50 {
        return Err("单篇笔记最多保留 50 条原文引用。".into());
    }
    for citation in citations {
        if citation.quote.trim().is_empty() || citation.quote.len() > 16000 {
            return Err("原文摘录不能为空且不能超过 16 KiB。".into());
        }
        let json: String = connection
            .query_row(
                "SELECT document_json FROM review_sources WHERE id=?1",
                [&citation.source_id],
                |r| r.get(0),
            )
            .map_err(|_| "引用来源不存在，请重新选择原文。")?;
        let document: ConvertedDocument =
            serde_json::from_str(&json).map_err(|_| "引用来源无效。")?;
        let page = page_text(&document, citation.page).ok_or("引用页码无效。")?;
        if citation
            .source_kind
            .as_ref()
            .is_some_and(|kind| kind != &document.source_kind)
            || document.id != citation.version
            || document.source_name != citation.source_name
            || !compact(&page).contains(&compact(&citation.quote))
        {
            return Err("摘录与记录的原文页不一致，请从按页文本中重新选取。".into());
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(windows)]
    #[test]
    fn windows_file_picker_path_registers_evidence_without_verbatim_prefix() {
        let root = std::env::temp_dir().join(format!("icwb-picker-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("中文 evidence.md");
        std::fs::write(&path, "Iq=10 µA，TT，27°C").unwrap();
        let normal = path.to_str().unwrap().trim_start_matches(r"\\?\");
        let (document, _, location) = read_source(Some(normal)).unwrap();
        assert!(document.markdown.contains("10 µA"));
        assert_eq!(
            location.unwrap(),
            path.canonicalize().unwrap().to_string_lossy()
        );
        assert!(crate::file_safety::unchanged_location(Path::new(normal), &path.canonicalize().unwrap()));
        assert!(!crate::file_safety::unchanged_location(&root.join("other.md"), &path.canonicalize().unwrap()));
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn original_version_and_missing_file_never_silently_rebind_citations() {
        let root = std::env::temp_dir().join(format!("icwb-source-test-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("source.md");
        std::fs::write(&path, "版本 A：PSRR 60 dB").unwrap();
        let path = path.canonicalize().unwrap();
        let text = path.to_str().unwrap();
        let (saved, _, _) = read_source(Some(text)).unwrap();
        assert!(resolve_source("id".into(), Some(text), &saved).is_ok());
        std::fs::write(&path, "版本 B：PSRR 40 dB").unwrap();
        assert!(resolve_source("id".into(), Some(text), &saved)
            .err()
            .unwrap()
            .contains("内容已变化"));
        std::fs::remove_file(&path).unwrap();
        assert!(resolve_source("id".into(), Some(text), &saved)
            .err()
            .unwrap()
            .contains("原件不可用"));
        assert!(saved.markdown.contains("60 dB"));
        std::fs::remove_dir(&root).unwrap();
    }

    #[test]
    fn migrated_drafts_and_citations_survive_reopen() {
        let connection = Connection::open_in_memory().unwrap();
        crate::storage::initialize(&connection).unwrap();
        connection.execute_batch("INSERT INTO notes(id,title,body) VALUES('note-old','原有评审','必须保留'); DROP TABLE note_reviews; DROP TABLE review_sources; DROP TABLE workspace_state; PRAGMA user_version=9;").unwrap();
        crate::storage::initialize(&connection).unwrap();
        connection
            .execute(
                "INSERT INTO notes(id,title,body) VALUES('note-legacy','旧版笔记','保留原有正文')",
                [],
            )
            .unwrap();
        connection.execute_batch("DROP TABLE note_reviews; DROP TABLE review_sources; DROP TABLE workspace_state; PRAGMA user_version=9;").unwrap();
        crate::storage::initialize(&connection).unwrap();
        assert_eq!(
            connection
                .query_row("SELECT body FROM notes WHERE id='note-legacy'", [], |r| {
                    r.get::<_, String>(0)
                })
                .unwrap(),
            "保留原有正文"
        );
        connection
            .execute(
                "INSERT INTO workspace_state(key,value) VALUES('note-draft',?1)",
                [r#"{"body":"中文未完成草稿"}"#],
            )
            .unwrap();
        crate::storage::initialize(&connection).unwrap();
        assert_eq!(
            connection
                .query_row("SELECT value FROM workspace_state", [], |r| r
                    .get::<_, String>(0))
                .unwrap(),
            r#"{"body":"中文未完成草稿"}"#
        );
        let (document, _) =
            source_from_bytes("part.md", "启动电路 ABC-123\n条件：25 °C".as_bytes()).unwrap();
        connection
            .execute(
                "INSERT INTO review_sources(id,document_json) VALUES('source-test',?1)",
                [serde_json::to_string(&document).unwrap()],
            )
            .unwrap();
        let citation = Citation {
            source_id: "source-test".into(),
            source_name: document.source_name.clone(),
            version: document.id.clone(),
            page: 1,
            quote: "条件：25 °C".into(),
            source_kind: Some("MD".into()),
        };
        assert!(validate_citations(&connection, &[citation.clone()]).is_ok());
        assert!(validate_citations(
            &connection,
            &[Citation {
                page: 2,
                ..citation.clone()
            }]
        )
        .is_err());
        assert!(validate_citations(
            &connection,
            &[Citation {
                quote: "5 mA".into(),
                ..citation
            }]
        )
        .is_err());
        assert!(state_key("model-secret").is_err());
    }
}

#[tauri::command]
pub fn validate_review_citation(app: tauri::AppHandle, citation: Citation) -> Result<(), String> {
    validate_citations(&open_database(&app)?, &[citation])
}

#[tauri::command]
pub fn export_work_template(path: String, content: String) -> Result<(), String> {
    crate::file_safety::validate_path(&path)?;
    use std::io::Write;
    if content.len() > 100000 || !path.to_lowercase().ends_with(".json") {
        return Err("模板必须为不超过 100 KB 的 JSON 文件。".into());
    }
    let value: serde_json::Value =
        serde_json::from_str(&content).map_err(|_| "模板 JSON 无效。")?;
    if value.get("schemaVersion").and_then(|v| v.as_i64()) != Some(1)
        || value.get("permissions") != Some(&serde_json::json!(["prompt"]))
    {
        return Err("仅允许提示词模板。".into());
    }
    let target = Path::new(&path);
    if std::fs::symlink_metadata(target).is_ok()
        && std::fs::symlink_metadata(target)
            .map(|m| crate::file_safety::is_link(&m) || !m.is_file())
            .unwrap_or(true)
    {
        return Err("请选择普通 JSON 文件。".into());
    }
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(target)
        .map_err(|_| "无法写入模板。")?;
    file.write_all(content.as_bytes())
        .and_then(|_| file.sync_all())
        .map_err(|_| "模板导出失败。".into())
}
