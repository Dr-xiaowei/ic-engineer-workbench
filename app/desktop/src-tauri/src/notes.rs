use crate::{
    pdf_export,
    storage::{open_database, record_audit},
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    fs::OpenOptions,
    io::{Cursor, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

const MAX_TITLE: usize = 180;
const MAX_BODY: usize = 8 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteInput {
    id: Option<String>,
    title: String,
    body: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    id: String,
    title: String,
    body: String,
    created_at: i64,
    updated_at: i64,
}

fn valid_id(value: &str) -> bool {
    value.starts_with("note-")
        && value.len() <= 80
        && value.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

fn new_id(title: &str) -> String {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|v| v.as_nanos())
        .unwrap_or_default();
    let digest = Sha256::digest(format!("{title}|{stamp}").as_bytes());
    format!(
        "note-{}",
        digest[..12]
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    )
}

fn validate(title: &str, body: &str) -> Result<(), String> {
    if title.trim().is_empty()
        || title.len() > MAX_TITLE
        || title.contains('\0')
        || body.len() > MAX_BODY
        || body.contains('\0')
    {
        return Err("笔记标题或内容无效，正文不能超过 8 MiB。".to_string());
    }
    Ok(())
}

fn notes_root(app: &tauri::AppHandle) -> Result<Option<PathBuf>, String> {
    let connection = open_database(app)?;
    let value = connection
        .query_row(
            "SELECT value FROM app_settings WHERE key='notes_root'",
            [],
            |row| row.get::<_, String>(0),
        )
        .ok();
    Ok(value.map(PathBuf::from))
}

fn verified_notes_root(app: &tauri::AppHandle) -> Result<Option<PathBuf>, String> {
    let Some(root) = notes_root(app)? else {
        return Ok(None);
    };
    let metadata = fs::symlink_metadata(&root)
        .map_err(|_| "已映射的笔记目录不可用，请重新选择。".to_string())?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err("已映射的笔记目录不再是安全目录，请重新选择。".to_string());
    }
    let canonical = root
        .canonicalize()
        .map_err(|_| "无法复核笔记映射目录。".to_string())?;
    if canonical != root {
        return Err("笔记映射目录已发生变化，请重新选择。".to_string());
    }
    Ok(Some(root))
}

#[tauri::command]
pub fn get_notes_root(app: tauri::AppHandle) -> Result<Option<String>, String> {
    Ok(notes_root(&app)?.and_then(|p| p.to_str().map(str::to_string)))
}

#[tauri::command]
pub fn set_notes_root(app: tauri::AppHandle, path: String) -> Result<String, String> {
    let requested = Path::new(&path);
    let metadata =
        fs::symlink_metadata(requested).map_err(|_| "无法读取所选笔记目录。".to_string())?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err("笔记存储位置必须是非符号链接目录。".to_string());
    }
    let mapped = requested
        .canonicalize()
        .map_err(|_| "无法验证笔记目录。".to_string())?
        .join("芯智笔记");
    if mapped.exists()
        && fs::symlink_metadata(&mapped)
            .map(|m| m.file_type().is_symlink() || !m.is_dir())
            .unwrap_or(true)
    {
        return Err("映射目标不是安全目录。".to_string());
    }
    fs::create_dir_all(&mapped).map_err(|_| "无法创建芯智笔记映射目录。".to_string())?;
    let text = mapped
        .to_str()
        .ok_or_else(|| "笔记目录编码不受支持。".to_string())?;
    // A new mapping must immediately include existing notes, not just future edits.
    for note in load(&app)? {
        sync_markdown_to(&mapped, &note.id, &note.title, &note.body)?;
    }
    open_database(&app)?.execute("INSERT INTO app_settings(key,value) VALUES('notes_root',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value,updated_at=unixepoch()", params![text]).map_err(|_| "无法保存笔记目录。".to_string())?;
    record_audit(&app, "notes.root-set", "success");
    Ok(text.to_string())
}

fn load(app: &tauri::AppHandle) -> Result<Vec<Note>, String> {
    let connection = open_database(app)?;
    let mut statement = connection.prepare("SELECT id,title,body,created_at,updated_at FROM notes ORDER BY updated_at DESC,id DESC").map_err(|_| "无法读取笔记。".to_string())?;
    let notes = statement
        .query_map([], |row| {
            Ok(Note {
                id: row.get(0)?,
                title: row.get(1)?,
                body: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })
        .map_err(|_| "无法读取笔记。".to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "笔记记录无效。".to_string())?;
    Ok(notes)
}

#[tauri::command]
pub fn list_notes(app: tauri::AppHandle) -> Result<Vec<Note>, String> {
    load(&app)
}

fn sync_markdown(app: &tauri::AppHandle, id: &str, title: &str, body: &str) -> Result<(), String> {
    let Some(root) = verified_notes_root(app)? else {
        return Ok(());
    };
    sync_markdown_to(&root, id, title, body)
}

fn sync_markdown_to(root: &Path, id: &str, title: &str, body: &str) -> Result<(), String> {
    let target = root.join(format!("{id}.md"));
    if fs::symlink_metadata(&target).is_ok()
        && fs::symlink_metadata(&target)
            .map(|m| m.file_type().is_symlink() || !m.is_file())
            .unwrap_or(true)
    {
        return Err("笔记映射文件不是普通文件。".to_string());
    }
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(target)
        .map_err(|_| "无法写入笔记映射文件。".to_string())?;
    file.write_all(format!("# {}\n\n{}", title.trim(), body).as_bytes())
        .and_then(|_| file.sync_all())
        .map_err(|_| "无法完成笔记文件写入。".to_string())
}

#[tauri::command]
pub fn save_note(app: tauri::AppHandle, input: NoteInput) -> Result<Note, String> {
    validate(&input.title, &input.body)?;
    let id = input.id.clone().unwrap_or_else(|| new_id(&input.title));
    if !valid_id(&id) {
        return Err("笔记标识无效。".to_string());
    }
    sync_markdown(&app, &id, &input.title, &input.body)?;
    open_database(&app)?.execute("INSERT INTO notes(id,title,body) VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET title=excluded.title,body=excluded.body,updated_at=unixepoch()", params![id,input.title.trim(),input.body]).map_err(|_| "无法保存笔记。".to_string())?;
    record_audit(&app, "notes.save", "success");
    load(&app)?
        .into_iter()
        .find(|n| n.id == id)
        .ok_or_else(|| "无法读取已保存笔记。".to_string())
}

#[tauri::command]
pub fn delete_note(app: tauri::AppHandle, note_id: String) -> Result<(), String> {
    if !valid_id(&note_id) {
        return Err("笔记标识无效。".to_string());
    }
    if let Some(root) = verified_notes_root(&app)? {
        let target = root.join(format!("{note_id}.md"));
        if target.exists() {
            let metadata =
                fs::symlink_metadata(&target).map_err(|_| "无法检查笔记映射文件。".to_string())?;
            if metadata.is_file() && !metadata.file_type().is_symlink() {
                fs::remove_file(target).map_err(|_| "无法移除笔记映射文件。".to_string())?;
            }
        }
    }
    open_database(&app)?
        .execute("DELETE FROM notes WHERE id=?1", params![note_id])
        .map_err(|_| "无法删除笔记。".to_string())?;
    record_audit(&app, "notes.delete", "success");
    Ok(())
}

fn safe_destination(path: &str, extension: &str) -> Result<PathBuf, String> {
    let target = Path::new(path);
    if target
        .extension()
        .and_then(|v| v.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
        != Some(extension)
    {
        return Err(format!("导出文件必须使用 .{extension} 扩展名。"));
    }
    let parent = target
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .ok_or_else(|| "导出目录无效。".to_string())?
        .canonicalize()
        .map_err(|_| "导出目录不存在。".to_string())?;
    let result = parent.join(
        target
            .file_name()
            .ok_or_else(|| "导出文件名无效。".to_string())?,
    );
    if fs::symlink_metadata(&result).is_ok()
        && fs::symlink_metadata(&result)
            .map(|m| m.file_type().is_symlink() || !m.is_file())
            .unwrap_or(true)
    {
        return Err("导出目标不是普通文件。".to_string());
    }
    Ok(result)
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
fn docx_bytes(title: &str, body: &str) -> Result<Vec<u8>, String> {
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut cursor);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        zip.start_file("[Content_Types].xml", options)
            .map_err(|_| "无法创建 DOCX。".to_string())?;
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#).map_err(|_|"无法创建 DOCX。".to_string())?;
        zip.start_file("_rels/.rels", options)
            .map_err(|_| "无法创建 DOCX。".to_string())?;
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#).map_err(|_|"无法创建 DOCX。".to_string())?;
        let paragraphs = std::iter::once(format!("# {title}"))
            .chain(body.lines().map(str::to_string))
            .map(|line| {
                format!(
                    "<w:p><w:r><w:t xml:space=\"preserve\">{}</w:t></w:r></w:p>",
                    xml_escape(&line)
                )
            })
            .collect::<String>();
        let xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>{paragraphs}<w:sectPr/></w:body></w:document>"#
        );
        zip.start_file("word/document.xml", options)
            .map_err(|_| "无法创建 DOCX。".to_string())?;
        zip.write_all(xml.as_bytes())
            .map_err(|_| "无法创建 DOCX。".to_string())?;
        zip.finish().map_err(|_| "无法完成 DOCX。".to_string())?;
    }
    Ok(cursor.into_inner())
}

#[tauri::command]
pub fn export_note(
    path: String,
    format: String,
    title: String,
    body: String,
    formatted: Option<String>,
) -> Result<(), String> {
    validate(&title, &body)?;
    if !matches!(format.as_str(), "md" | "pdf" | "doc" | "docx") {
        return Err("不支持的笔记导出格式。".to_string());
    }
    if format == "pdf" {
        let markdown = format!("# {title}\n\n{body}");
        return pdf_export::export_pdf(path, title, markdown);
    }
    let destination = safe_destination(&path, &format)?;
    let bytes = if format == "md" {
        format!("# {title}\n\n{body}").into_bytes()
    } else if let Some(content) = formatted {
        if content.len() > 16 * 1024 * 1024 {
            return Err("排版导出超过 16 MiB 上限。".to_string());
        }
        if format == "docx" {
            let decoded = STANDARD
                .decode(&content)
                .map_err(|_| "DOCX 数据编码无效。".to_string())?;
            let mut archive = zip::ZipArchive::new(Cursor::new(&decoded))
                .map_err(|_| "DOCX 数据结构无效。".to_string())?;
            archive
                .by_name("word/document.xml")
                .map_err(|_| "DOCX 缺少正文。".to_string())?;
            drop(archive);
            decoded
        } else {
            content.into_bytes()
        }
    } else if format == "doc" {
        format!("<!doctype html><meta charset=\"utf-8\"><title>{}</title><h1>{}</h1><pre style=\"white-space:pre-wrap;font:11pt sans-serif\">{}</pre>",xml_escape(&title),xml_escape(&title),xml_escape(&body)).into_bytes()
    } else {
        docx_bytes(&title, &body)?
    };
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(destination)
        .map_err(|_| "无法创建笔记导出文件。".to_string())?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| "无法写入笔记导出文件。".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn docx_is_valid_and_note_ids_are_bounded() {
        let bytes = docx_bytes("测试笔记", "## 参数\n偏置电流 10 uA").unwrap();
        let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
        assert!(zip.by_name("word/document.xml").is_ok());
        assert!(valid_id("note-0123abcd"));
        assert!(!valid_id("note-../../bad"));
    }
}
