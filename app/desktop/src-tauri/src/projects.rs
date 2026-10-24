use crate::{document::convert_document_path, storage::open_database};
use rusqlite::{params, Connection};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};
use walkdir::{DirEntry, WalkDir};

const MAX_PROJECT_FILES: usize = 2000;
const MAX_RELATIVE_PATH_LENGTH: usize = 1000;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSummary {
    id: String,
    name: String,
    root_name: String,
    file_count: usize,
    indexed_count: usize,
    updated_at: i64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectFile {
    document_id: Option<String>,
    index_error: Option<String>,
    modified_at: i64,
    page_count: Option<usize>,
    relative_path: String,
    size_bytes: u64,
    source_kind: String,
    source_name: String,
    warnings: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDetail {
    files: Vec<ProjectFile>,
    summary: ProjectSummary,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexedDocument {
    markdown: String,
    page_count: Option<usize>,
    relative_path: String,
    source_kind: String,
    source_name: String,
    warnings: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    document_id: String,
    relative_path: String,
    score: f64,
    snippet: String,
    source_name: String,
}

fn project_id(path: &Path) -> String {
    let digest = Sha256::digest(path.to_string_lossy().as_bytes());
    format!(
        "project-{}",
        digest[..12]
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    )
}

fn validate_project_id(value: &str) -> Result<(), String> {
    let valid = value.starts_with("project-")
        && value.len() <= 80
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-');
    valid
        .then_some(())
        .ok_or_else(|| "项目标识无效。".to_string())
}

fn project_root(path: &str) -> Result<PathBuf, String> {
    crate::file_safety::validate_path(path)?;
    if path.is_empty() || path.len() > 4096 {
        return Err("项目目录路径无效。".to_string());
    }
    let requested = Path::new(path);
    let metadata =
        fs::symlink_metadata(requested).map_err(|_| "无法读取所选项目目录。".to_string())?;
    if crate::file_safety::is_link(&metadata) || !metadata.is_dir() {
        return Err("只允许用户明确选择的普通目录，不接受符号链接。".to_string());
    }
    requested
        .canonicalize()
        .map_err(|_| "无法解析项目目录。".to_string())
}

fn root_name(root: &Path) -> String {
    root.file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("本地项目")
        .chars()
        .take(120)
        .collect()
}

fn allowed_entry(entry: &DirEntry) -> bool {
    if entry.depth() == 0 {
        return true;
    }
    let name = entry.file_name().to_string_lossy();
    if name.starts_with('.') || matches!(name.as_ref(), "node_modules" | "target" | "dist") {
        return false;
    }
    entry
        .metadata()
        .is_ok_and(|metadata| !crate::file_safety::is_link(&metadata))
}

fn supported_file(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .is_some_and(|extension| matches!(extension.as_str(), "txt" | "md" | "docx" | "pdf"))
}

fn modified_at(metadata: &fs::Metadata) -> i64 {
    metadata
        .modified()
        .ok()
        .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
        .map(|value| value.as_secs() as i64)
        .unwrap_or(0)
}

fn load_summary(connection: &Connection, id: &str) -> Result<ProjectSummary, String> {
    connection
        .query_row(
            "SELECT p.id, p.name, p.root_path, p.updated_at,
                    COUNT(f.relative_path), COUNT(f.document_id)
             FROM projects p
             LEFT JOIN project_files f ON f.project_id = p.id
             WHERE p.id = ?1
             GROUP BY p.id",
            params![id],
            |row| {
                let path: String = row.get(2)?;
                Ok(ProjectSummary {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    root_name: root_name(Path::new(&path)),
                    updated_at: row.get(3)?,
                    file_count: row.get::<_, i64>(4)? as usize,
                    indexed_count: row.get::<_, i64>(5)? as usize,
                })
            },
        )
        .map_err(|_| "项目不存在或本地记录无效。".to_string())
}

#[tauri::command]
pub fn add_project(app: tauri::AppHandle, path: String) -> Result<ProjectSummary, String> {
    let root = project_root(&path)?;
    let root_path = root
        .to_str()
        .ok_or_else(|| "项目目录不是受支持的文本编码。".to_string())?;
    let id = project_id(&root);
    let name = root_name(&root);
    let connection = open_database(&app)?;
    connection
        .execute(
            "INSERT INTO projects (id, name, root_path) VALUES (?1, ?2, ?3)
             ON CONFLICT(root_path) DO UPDATE SET name = excluded.name, updated_at = unixepoch()",
            params![id, name, root_path],
        )
        .map_err(|_| "无法保存项目授权。".to_string())?;
    load_summary(&connection, &id)
}

#[tauri::command]
pub fn list_projects(app: tauri::AppHandle) -> Result<Vec<ProjectSummary>, String> {
    let connection = open_database(&app)?;
    let mut query = connection
        .prepare("SELECT id FROM projects ORDER BY updated_at DESC LIMIT 100")
        .map_err(|_| "无法读取项目列表。".to_string())?;
    let ids = query
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|_| "无法读取项目列表。".to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "项目列表格式无效。".to_string())?;
    ids.iter().map(|id| load_summary(&connection, id)).collect()
}

#[tauri::command]
pub fn remove_project(app: tauri::AppHandle, project_id: String) -> Result<(), String> {
    validate_project_id(&project_id)?;
    let mut connection = open_database(&app)?;
    let transaction = connection
        .transaction()
        .map_err(|_| "无法开始移除项目事务。".to_string())?;
    transaction
        .execute(
            "DELETE FROM project_search WHERE project_id = ?1",
            params![project_id],
        )
        .map_err(|_| "无法清理项目索引。".to_string())?;
    transaction
        .execute("DELETE FROM projects WHERE id = ?1", params![project_id])
        .map_err(|_| "无法移除项目记录。".to_string())?;
    transaction
        .commit()
        .map_err(|_| "无法提交项目移除事务。".to_string())
}

#[tauri::command]
pub fn index_project(app: tauri::AppHandle, project_id: String) -> Result<ProjectDetail, String> {
    validate_project_id(&project_id)?;
    let mut connection = open_database(&app)?;
    let root_path: String = connection
        .query_row(
            "SELECT root_path FROM projects WHERE id = ?1",
            params![project_id],
            |row| row.get(0),
        )
        .map_err(|_| "项目不存在。".to_string())?;
    let root = project_root(&root_path)?;
    let entries = WalkDir::new(&root)
        .max_depth(16)
        .follow_links(false)
        .into_iter()
        .filter_entry(allowed_entry)
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file() && supported_file(entry.path()))
        .take(MAX_PROJECT_FILES + 1)
        .collect::<Vec<_>>();
    if entries.len() > MAX_PROJECT_FILES {
        return Err("项目中受支持的文档超过 2000 个，请缩小授权目录。".to_string());
    }

    let existing = {
        let mut statement = connection
            .prepare(
                "SELECT relative_path, size_bytes, modified_at, document_id, markdown, page_count,
                        source_kind, source_name, warnings_json
                 FROM project_files WHERE project_id = ?1",
            )
            .map_err(|_| "无法读取已有项目索引。".to_string())?;
        let rows = statement
            .query_map(params![project_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    (
                        row.get::<_, i64>(1)? as u64,
                        row.get::<_, i64>(2)?,
                        row.get::<_, Option<String>>(3)?,
                        row.get::<_, Option<String>>(4)?,
                        row.get::<_, Option<i64>>(5)?.map(|value| value as usize),
                        row.get::<_, String>(6)?,
                        row.get::<_, String>(7)?,
                        row.get::<_, String>(8)?,
                    ),
                ))
            })
            .map_err(|_| "无法读取已有项目索引。".to_string())?;
        rows.collect::<Result<HashMap<_, _>, _>>()
            .map_err(|_| "已有项目索引格式无效。".to_string())?
    };

    let mut files = Vec::with_capacity(entries.len());
    for entry in entries {
        let relative_path = entry
            .path()
            .strip_prefix(&root)
            .ok()
            .and_then(Path::to_str)
            .filter(|value| value.len() <= MAX_RELATIVE_PATH_LENGTH)
            .ok_or_else(|| "项目内存在过长或无法编码的相对路径。".to_string())?
            .to_string();
        let metadata = entry
            .metadata()
            .map_err(|_| "无法读取项目文件元数据。".to_string())?;
        let fallback_name = entry
            .file_name()
            .to_string_lossy()
            .chars()
            .take(180)
            .collect();
        let fallback_kind = entry
            .path()
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("")
            .to_ascii_uppercase();
        let current_modified_at = modified_at(&metadata);
        if let Some((
            size_bytes,
            indexed_modified_at,
            document_id,
            markdown,
            page_count,
            source_kind,
            source_name,
            warnings_json,
        )) = existing.get(&relative_path)
        {
            if *size_bytes == metadata.len() && *indexed_modified_at == current_modified_at {
                if let (Some(document_id), Some(markdown)) = (document_id.clone(), markdown.clone())
                {
                    files.push((
                        ProjectFile {
                            document_id: Some(document_id),
                            index_error: None,
                            modified_at: current_modified_at,
                            page_count: *page_count,
                            relative_path,
                            size_bytes: *size_bytes,
                            source_kind: source_kind.clone(),
                            source_name: source_name.clone(),
                            warnings: serde_json::from_str(warnings_json).unwrap_or_default(),
                        },
                        Some(markdown),
                    ));
                    continue;
                }
            }
        }
        match convert_document_path(entry.path()) {
            Ok(document) => files.push((
                ProjectFile {
                    document_id: Some(document.id),
                    index_error: None,
                    modified_at: current_modified_at,
                    page_count: document.page_count,
                    relative_path,
                    size_bytes: document.size_bytes,
                    source_kind: document.source_kind,
                    source_name: document.source_name,
                    warnings: document.warnings,
                },
                Some(document.markdown),
            )),
            Err(error) => files.push((
                ProjectFile {
                    document_id: None,
                    index_error: Some(error),
                    modified_at: current_modified_at,
                    page_count: None,
                    relative_path,
                    size_bytes: metadata.len(),
                    source_kind: fallback_kind,
                    source_name: fallback_name,
                    warnings: Vec::new(),
                },
                None,
            )),
        }
    }

    let transaction = connection
        .transaction()
        .map_err(|_| "无法开始项目索引事务。".to_string())?;
    transaction
        .execute(
            "DELETE FROM project_search WHERE project_id = ?1",
            params![project_id],
        )
        .and_then(|_| {
            transaction.execute(
                "DELETE FROM project_files WHERE project_id = ?1",
                params![project_id],
            )
        })
        .map_err(|_| "无法更新项目索引。".to_string())?;
    for (file, markdown) in &files {
        let markdown = markdown.as_deref();
        let warnings_json = serde_json::to_string(&file.warnings)
            .map_err(|_| "无法序列化文档警告。".to_string())?;
        transaction
            .execute(
                "INSERT INTO project_files
                 (project_id, relative_path, source_name, source_kind, size_bytes, modified_at,
                  document_id, markdown, page_count, warnings_json, index_error)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    project_id,
                    file.relative_path,
                    file.source_name,
                    file.source_kind,
                    file.size_bytes as i64,
                    file.modified_at,
                    file.document_id,
                    markdown,
                    file.page_count.map(|value| value as i64),
                    warnings_json,
                    file.index_error,
                ],
            )
            .map_err(|_| "无法保存项目文件索引。".to_string())?;
        if let (Some(document_id), Some(markdown)) = (&file.document_id, markdown) {
            transaction
                .execute(
                    "INSERT INTO project_search (project_id, relative_path, source_name, markdown)
                     VALUES (?1, ?2, ?3, ?4)",
                    params![project_id, file.relative_path, file.source_name, markdown],
                )
                .map_err(|_| format!("无法写入文档全文索引：{document_id}"))?;
        }
    }
    transaction
        .execute(
            "UPDATE projects SET updated_at = unixepoch() WHERE id = ?1",
            params![project_id],
        )
        .map_err(|_| "无法更新项目索引时间。".to_string())?;
    transaction
        .commit()
        .map_err(|_| "无法提交项目索引事务。".to_string())?;
    load_project_detail(&connection, &project_id)
}

fn load_project_detail(connection: &Connection, project_id: &str) -> Result<ProjectDetail, String> {
    let summary = load_summary(connection, project_id)?;
    let mut query = connection
        .prepare(
            "SELECT document_id, index_error, modified_at, page_count, relative_path,
                    size_bytes, source_kind, source_name, warnings_json
             FROM project_files WHERE project_id = ?1 ORDER BY relative_path LIMIT 2000",
        )
        .map_err(|_| "无法读取项目文件。".to_string())?;
    let files = query
        .query_map(params![project_id], |row| {
            let warnings_json: String = row.get(8)?;
            Ok(ProjectFile {
                document_id: row.get(0)?,
                index_error: row.get(1)?,
                modified_at: row.get(2)?,
                page_count: row.get::<_, Option<i64>>(3)?.map(|value| value as usize),
                relative_path: row.get(4)?,
                size_bytes: row.get::<_, i64>(5)? as u64,
                source_kind: row.get(6)?,
                source_name: row.get(7)?,
                warnings: serde_json::from_str(&warnings_json).unwrap_or_default(),
            })
        })
        .map_err(|_| "无法读取项目文件。".to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "项目文件索引格式无效。".to_string())?;
    Ok(ProjectDetail { files, summary })
}

#[tauri::command]
pub fn get_project(app: tauri::AppHandle, project_id: String) -> Result<ProjectDetail, String> {
    validate_project_id(&project_id)?;
    load_project_detail(&open_database(&app)?, &project_id)
}

#[tauri::command]
pub fn get_indexed_document(
    app: tauri::AppHandle,
    project_id: String,
    relative_path: String,
) -> Result<IndexedDocument, String> {
    validate_project_id(&project_id)?;
    if relative_path.is_empty() || relative_path.len() > MAX_RELATIVE_PATH_LENGTH {
        return Err("项目文件相对路径无效。".to_string());
    }
    open_database(&app)?
        .query_row(
            "SELECT markdown, page_count, relative_path, source_kind, source_name, warnings_json
             FROM project_files
             WHERE project_id = ?1 AND relative_path = ?2 AND markdown IS NOT NULL",
            params![project_id, relative_path],
            |row| {
                let warnings_json: String = row.get(5)?;
                Ok(IndexedDocument {
                    markdown: row.get(0)?,
                    page_count: row.get::<_, Option<i64>>(1)?.map(|value| value as usize),
                    relative_path: row.get(2)?,
                    source_kind: row.get(3)?,
                    source_name: row.get(4)?,
                    warnings: serde_json::from_str(&warnings_json).unwrap_or_default(),
                })
            },
        )
        .map_err(|_| "该文件尚未成功建立本地索引。".to_string())
}

fn search_terms(query: &str) -> Result<Vec<String>, String> {
    if query.trim().is_empty() || query.len() > 500 {
        return Err("请输入不超过 500 字节的中文、型号或关键词。".into());
    }
    Ok(query
        .split_whitespace()
        .take(12)
        .map(str::to_lowercase)
        .collect())
}

// Literal substring matching complements unicode61: CJK short words and hyphenated
// part numbers must work without rebuilding existing user indexes. SQL returns only
// ranked excerpts; complete documents are never sent to a model during retrieval.
fn search_index(
    connection: &Connection,
    project_id: &str,
    query: &str,
) -> Result<Vec<SearchHit>, String> {
    let terms = serde_json::to_string(&search_terms(query)?).map_err(|_| "搜索词无效。")?;
    let mut statement = connection
        .prepare(
            "WITH matches AS (
           SELECT f.document_id,f.relative_path,f.source_name,f.markdown,
             instr(lower(f.markdown),t.value) AS pos,
             instr(lower(f.source_name),t.value)>0 AS title_match
           FROM project_files f,json_each(?2) t
           WHERE f.project_id=?1 AND f.document_id IS NOT NULL AND
             (instr(lower(f.markdown),t.value)>0 OR instr(lower(f.source_name),t.value)>0)
         ), ranked AS (
           SELECT document_id,relative_path,source_name,markdown,
             COALESCE(min(NULLIF(pos,0)),1) AS position,count(*)+sum(title_match)*3 AS relevance
           FROM matches GROUP BY relative_path
         )
         SELECT document_id,relative_path,source_name,
           substr(markdown,max(1,position-180),1100),relevance
         FROM ranked ORDER BY relevance DESC,relative_path LIMIT 30",
        )
        .map_err(|_| "无法准备项目搜索。")?;
    let rows = statement
        .query_map(params![project_id, terms], |row| {
            Ok(SearchHit {
                document_id: row.get(0)?,
                relative_path: row.get(1)?,
                source_name: row.get(2)?,
                snippet: row.get(3)?,
                score: row.get(4)?,
            })
        })
        .map_err(|_| "项目搜索失败。")?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|_| "项目搜索结果无效。".into())
}
#[tauri::command]
pub fn search_project(
    app: tauri::AppHandle,
    project_id: String,
    query: String,
) -> Result<Vec<SearchHit>, String> {
    validate_project_id(&project_id)?;
    search_index(&open_database(&app)?, &project_id, &query)
}
#[tauri::command]
pub fn project_semantic_candidates(
    app: tauri::AppHandle,
    project_id: String,
    query: String,
) -> Result<Vec<SearchHit>, String> {
    // The caller supplies explicit keywords, not the 40 most recently modified files.
    search_project(app, project_id, query)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifiers_and_fts_queries_are_bounded() {
        assert!(validate_project_id("project-0123abcdef").is_ok());
        assert!(validate_project_id("../outside").is_err());
        assert_eq!(search_terms("偏置 电流").unwrap(), vec!["偏置", "电流"]);
        assert!(search_terms("").is_err());
    }
}

#[cfg(test)]
mod retrieval_tests {
    use super::*;
    #[test]
    fn chinese_models_and_old_document_tail_are_retrieved_without_fts_token_boundaries() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE project_files(project_id TEXT,document_id TEXT,relative_path TEXT,source_name TEXT,markdown TEXT);").unwrap();
        for i in 0..60 {
            let body = format!(
                "{}\n第 {} 项：带隙基准启动电路 ZXW-{:03} PSRR 60 dB 25 °C",
                "背景资料。".repeat(3000),
                i,
                i
            );
            db.execute(
                "INSERT INTO project_files VALUES('p',?1,?2,?2,?3)",
                params![format!("id{i}"), format!("part-{i}.md"), body],
            )
            .unwrap();
        }
        db.execute("INSERT INTO project_files VALUES('other','foreign','private.md','private.md','私有独占')",[]).unwrap();
        let started = std::time::Instant::now();
        for i in 0..30 {
            let results = search_index(&db, "p", &format!("ZXW-{i:03}")).unwrap();
            assert_eq!(results.len(), 1);
            assert!(results[0].snippet.contains("启动电路"));
        }
        assert_eq!(search_index(&db, "p", "启动").unwrap().len(), 30);
        assert_eq!(
            search_index(&db, "p", "zxw-000").unwrap()[0].document_id,
            "id0"
        );
        assert!(search_index(&db, "p", "私有独占").unwrap().is_empty());
        assert!(search_index(&db, "p", "%_'").unwrap().is_empty());
        eprintln!("60 个合成文档、30 次型号定位耗时 {:?}", started.elapsed());
    }
}
