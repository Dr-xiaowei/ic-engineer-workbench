use crate::attachments::{
    validate_attachment, ChatAttachment, MAX_ATTACHMENTS_PER_MESSAGE, MAX_ATTACHMENTS_TOTAL_BYTES,
};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};
use tauri::Manager;

const MAX_CONVERSATIONS: usize = 200;
const MAX_STORED_MESSAGES: usize = 1_000;
const MAX_TITLE_LENGTH: usize = 160;
const MAX_STORED_MESSAGE_LENGTH: usize = 32_000;
pub(crate) const SCHEMA_VERSION: i64 = 9;
const MAX_DOCUMENT_ATTACHMENTS_PER_MESSAGE: usize = 3;
const MAX_DOCUMENT_ATTACHMENT_LENGTH: usize = 16 * 1024;
const MAX_CONVERSATION_DOCUMENT_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StoredDocumentAttachment {
    pub(crate) id: String,
    pub(crate) markdown: String,
    pub(crate) source_kind: String,
    pub(crate) source_name: String,
    #[serde(default)]
    pub(crate) warnings: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StoredMessage {
    #[serde(default)]
    pub(crate) attachments: Vec<ChatAttachment>,
    #[serde(default)]
    pub(crate) documents: Vec<StoredDocumentAttachment>,
    pub(crate) id: String,
    pub(crate) request_id: Option<String>,
    pub(crate) role: String,
    pub(crate) text: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(crate) struct StoredConversation {
    pub(crate) id: String,
    pub(crate) messages: Vec<StoredMessage>,
    pub(crate) title: String,
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 100
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
}

fn validate_conversation(conversation: &StoredConversation) -> Result<(), String> {
    if !valid_identifier(&conversation.id)
        || conversation.title.trim().is_empty()
        || conversation.title.len() > MAX_TITLE_LENGTH
        || conversation.messages.len() > MAX_STORED_MESSAGES
    {
        return Err("会话标识、标题或消息数量无效。".to_string());
    }
    if conversation.messages.iter().any(|message| {
        !valid_identifier(&message.id)
            || !matches!(message.role.as_str(), "user" | "assistant")
            || message.text.len() > MAX_STORED_MESSAGE_LENGTH
            || message.attachments.len() > MAX_ATTACHMENTS_PER_MESSAGE
            || message.documents.len() > MAX_DOCUMENT_ATTACHMENTS_PER_MESSAGE
            || message.attachments.iter().any(|attachment| {
                message.role != "user" || validate_attachment(attachment).is_err()
            })
            || message
                .request_id
                .as_ref()
                .is_some_and(|request_id| request_id.len() > 240)
            || message.documents.iter().any(|document| {
                !valid_identifier(&document.id)
                    || document.markdown.is_empty()
                    || document.markdown.len() > MAX_DOCUMENT_ATTACHMENT_LENGTH
                    || document.source_name.is_empty()
                    || document.source_name.len() > 180
                    || document.source_kind.is_empty()
                    || document.source_kind.len() > 16
                    || document.warnings.len() > 20
                    || document.warnings.iter().any(|warning| warning.len() > 500)
            })
    }) {
        return Err("会话消息格式或长度无效。".to_string());
    }
    let attachment_bytes = conversation
        .messages
        .iter()
        .flat_map(|message| &message.attachments)
        .map(|attachment| attachment.size_bytes)
        .sum::<usize>();
    if attachment_bytes > MAX_ATTACHMENTS_TOTAL_BYTES.saturating_mul(20) {
        return Err("单个会话的附件数据超过 240 MiB 限制。".to_string());
    }
    let document_bytes = conversation
        .messages
        .iter()
        .flat_map(|message| &message.documents)
        .map(|document| document.markdown.len())
        .sum::<usize>();
    if document_bytes > MAX_CONVERSATION_DOCUMENT_BYTES {
        return Err("单个会话的文档附件文本超过 16 MiB 限制。".to_string());
    }
    Ok(())
}

fn initialize(connection: &Connection) -> Result<(), String> {
    let version = connection
        .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
        .map_err(|_| "无法读取本地会话数据库版本。".to_string())?;
    if version > SCHEMA_VERSION {
        return Err("本地会话数据库来自更新版本，当前应用无法安全打开。".to_string());
    }
    connection
        .execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE IF NOT EXISTS conversations (
               id TEXT PRIMARY KEY,
               title TEXT NOT NULL,
               created_at INTEGER NOT NULL DEFAULT (unixepoch()),
               updated_at INTEGER NOT NULL DEFAULT (unixepoch())
             );
             CREATE TABLE IF NOT EXISTS messages (
               id TEXT PRIMARY KEY,
               conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
               position INTEGER NOT NULL,
               role TEXT NOT NULL CHECK (role IN ('user', 'assistant')),
               text TEXT NOT NULL,
               request_id TEXT
             );
             CREATE INDEX IF NOT EXISTS messages_conversation_position
               ON messages(conversation_id, position);
             CREATE TABLE IF NOT EXISTS attachments (
               id TEXT PRIMARY KEY,
               message_id TEXT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
               position INTEGER NOT NULL,
               name TEXT NOT NULL,
               mime_type TEXT NOT NULL,
               data_url TEXT NOT NULL,
               size_bytes INTEGER NOT NULL
             );
             CREATE INDEX IF NOT EXISTS attachments_message_position
               ON attachments(message_id, position);
             CREATE TABLE IF NOT EXISTS document_attachments (
               id TEXT PRIMARY KEY,
               message_id TEXT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
               position INTEGER NOT NULL,
               source_name TEXT NOT NULL,
               source_kind TEXT NOT NULL,
               markdown TEXT NOT NULL,
               warnings_json TEXT NOT NULL DEFAULT '[]'
             );
             CREATE INDEX IF NOT EXISTS document_attachments_message_position
               ON document_attachments(message_id, position);
             CREATE TABLE IF NOT EXISTS projects (
               id TEXT PRIMARY KEY,
               name TEXT NOT NULL,
               root_path TEXT NOT NULL UNIQUE,
               created_at INTEGER NOT NULL DEFAULT (unixepoch()),
               updated_at INTEGER NOT NULL DEFAULT (unixepoch())
             );
             CREATE TABLE IF NOT EXISTS project_files (
               project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
               relative_path TEXT NOT NULL,
               source_name TEXT NOT NULL,
               source_kind TEXT NOT NULL,
               size_bytes INTEGER NOT NULL,
               modified_at INTEGER NOT NULL,
               document_id TEXT,
               markdown TEXT,
               page_count INTEGER,
               warnings_json TEXT NOT NULL DEFAULT '[]',
               index_error TEXT,
               indexed_at INTEGER NOT NULL DEFAULT (unixepoch()),
               PRIMARY KEY (project_id, relative_path)
             );
             CREATE INDEX IF NOT EXISTS project_files_document_id
               ON project_files(document_id);
             CREATE VIRTUAL TABLE IF NOT EXISTS project_search USING fts5(
               project_id UNINDEXED,
               relative_path UNINDEXED,
               source_name UNINDEXED,
               markdown,
               tokenize = 'unicode61 remove_diacritics 2'
             );
             CREATE TABLE IF NOT EXISTS skills (
               id TEXT PRIMARY KEY,
               name TEXT NOT NULL,
               description TEXT NOT NULL,
               instructions TEXT NOT NULL,
               source_name TEXT NOT NULL,
               enabled INTEGER NOT NULL DEFAULT 0,
               has_scripts INTEGER NOT NULL DEFAULT 0,
               version TEXT NOT NULL DEFAULT 'unspecified',
               source_type TEXT NOT NULL DEFAULT 'local-file',
               permissions_json TEXT NOT NULL DEFAULT '[\"prompt\"]',
               has_restricted_permissions INTEGER NOT NULL DEFAULT 0,
               imported_at INTEGER NOT NULL DEFAULT (unixepoch()),
               updated_at INTEGER NOT NULL DEFAULT (unixepoch())
             );
             CREATE TABLE IF NOT EXISTS mail_accounts (
               id TEXT PRIMARY KEY,
               address TEXT NOT NULL,
               display_name TEXT NOT NULL,
               username TEXT NOT NULL,
               imap_host TEXT NOT NULL,
               imap_port INTEGER NOT NULL,
               smtp_host TEXT NOT NULL,
               smtp_port INTEGER NOT NULL,
               created_at INTEGER NOT NULL DEFAULT (unixepoch()),
               updated_at INTEGER NOT NULL DEFAULT (unixepoch())
             );
             CREATE TABLE IF NOT EXISTS mail_messages (
               account_id TEXT NOT NULL REFERENCES mail_accounts(id) ON DELETE CASCADE,
               folder TEXT NOT NULL,
               uid INTEGER NOT NULL,
               message_id TEXT,
               subject TEXT NOT NULL,
               sender TEXT NOT NULL,
               recipients TEXT NOT NULL,
               sent_at INTEGER,
               preview TEXT NOT NULL,
               body_text TEXT NOT NULL,
               unread INTEGER NOT NULL DEFAULT 0,
               raw_size INTEGER NOT NULL DEFAULT 0,
               synced_at INTEGER NOT NULL DEFAULT (unixepoch()),
               PRIMARY KEY (account_id, folder, uid)
             );
             CREATE VIRTUAL TABLE IF NOT EXISTS mail_search USING fts5(
               account_id UNINDEXED,
               folder UNINDEXED,
               uid UNINDEXED,
               subject,
               sender,
               recipients,
               body_text,
               tokenize = 'unicode61 remove_diacritics 2'
             );
             CREATE TABLE IF NOT EXISTS mail_attachments (
               account_id TEXT NOT NULL,
               folder TEXT NOT NULL,
               uid INTEGER NOT NULL,
               position INTEGER NOT NULL,
               name TEXT NOT NULL,
               mime_type TEXT NOT NULL,
               size_bytes INTEGER NOT NULL,
               data BLOB NOT NULL,
               PRIMARY KEY (account_id, folder, uid, position),
               FOREIGN KEY (account_id, folder, uid)
                 REFERENCES mail_messages(account_id, folder, uid) ON DELETE CASCADE
             );
             CREATE TABLE IF NOT EXISTS app_settings (
               key TEXT PRIMARY KEY,
               value TEXT NOT NULL,
               updated_at INTEGER NOT NULL DEFAULT (unixepoch())
             );
             CREATE TABLE IF NOT EXISTS audit_events (
               id INTEGER PRIMARY KEY AUTOINCREMENT,
               action TEXT NOT NULL,
               outcome TEXT NOT NULL,
               created_at INTEGER NOT NULL DEFAULT (unixepoch())
             );
             CREATE TABLE IF NOT EXISTS project_progress (
               project_id TEXT PRIMARY KEY REFERENCES projects(id) ON DELETE CASCADE,
               start_date TEXT NOT NULL DEFAULT '',
               end_date TEXT NOT NULL DEFAULT '',
               current_phase TEXT NOT NULL DEFAULT '',
               milestone TEXT NOT NULL DEFAULT '',
               weekly_focus TEXT NOT NULL DEFAULT '',
               current_focus TEXT NOT NULL DEFAULT '',
               risks TEXT NOT NULL DEFAULT '',
               progress_percent INTEGER NOT NULL DEFAULT 0 CHECK(progress_percent BETWEEN 0 AND 100),
               updated_at INTEGER NOT NULL DEFAULT (unixepoch())
             );
             CREATE TABLE IF NOT EXISTS calendar_tasks (
               id TEXT PRIMARY KEY,
               title TEXT NOT NULL,
               start_at INTEGER NOT NULL,
               end_at INTEGER,
               status TEXT NOT NULL CHECK(status IN ('todo','doing','done')),
               priority TEXT NOT NULL CHECK(priority IN ('normal','important')),
               remind_at INTEGER,
               project_id TEXT REFERENCES projects(id) ON DELETE SET NULL,
               notes TEXT NOT NULL DEFAULT '',
               created_at INTEGER NOT NULL DEFAULT (unixepoch()),
               updated_at INTEGER NOT NULL DEFAULT (unixepoch())
             );
             CREATE INDEX IF NOT EXISTS calendar_tasks_date ON calendar_tasks(start_at,status);
             CREATE INDEX IF NOT EXISTS calendar_tasks_project ON calendar_tasks(project_id,start_at);
             CREATE TABLE IF NOT EXISTS software_launchers (
               id TEXT PRIMARY KEY,
               name TEXT NOT NULL,
               target_path TEXT NOT NULL UNIQUE,
               target_kind TEXT NOT NULL CHECK(target_kind IN ('application','executable')),
               created_at INTEGER NOT NULL DEFAULT (unixepoch()),
               updated_at INTEGER NOT NULL DEFAULT (unixepoch())
             );
             CREATE TABLE IF NOT EXISTS project_annotations (
               project_id TEXT PRIMARY KEY REFERENCES projects(id) ON DELETE CASCADE,
               starred INTEGER NOT NULL DEFAULT 0 CHECK(starred IN (0,1)),
               note TEXT NOT NULL DEFAULT '',
               updated_at INTEGER NOT NULL DEFAULT (unixepoch())
             );
             CREATE TABLE IF NOT EXISTS project_progress_history (
               id TEXT PRIMARY KEY,
               project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
               snapshot_json TEXT NOT NULL,
               created_at INTEGER NOT NULL DEFAULT (unixepoch())
             );
             CREATE INDEX IF NOT EXISTS project_progress_history_project
               ON project_progress_history(project_id,created_at DESC);
             CREATE TABLE IF NOT EXISTS notes (
               id TEXT PRIMARY KEY,
               title TEXT NOT NULL,
               body TEXT NOT NULL,
               created_at INTEGER NOT NULL DEFAULT (unixepoch()),
               updated_at INTEGER NOT NULL DEFAULT (unixepoch())
             );",
        )
        .map_err(|_| "无法初始化本地会话数据库。".to_string())?;
    if version > 0 && version < 7 {
        connection
            .execute_batch(
                "ALTER TABLE skills ADD COLUMN version TEXT NOT NULL DEFAULT 'unspecified';
                 ALTER TABLE skills ADD COLUMN source_type TEXT NOT NULL DEFAULT 'local-file';
                 ALTER TABLE skills ADD COLUMN permissions_json TEXT NOT NULL DEFAULT '[\"prompt\"]';
                 ALTER TABLE skills ADD COLUMN has_restricted_permissions INTEGER NOT NULL DEFAULT 0;",
            )
            .map_err(|_| "无法升级 Skill 清单数据。".to_string())?;
    }
    connection
        .pragma_update(None, "user_version", SCHEMA_VERSION)
        .map_err(|_| "无法更新本地数据库版本。".to_string())?;
    Ok(())
}

pub(crate) fn network_access_enabled(app: &tauri::AppHandle) -> Result<bool, String> {
    let connection = open_database(app)?;
    Ok(connection
        .query_row(
            "SELECT value FROM app_settings WHERE key='network_access_enabled'",
            [],
            |row| row.get::<_, String>(0),
        )
        .map(|value| value == "true")
        .unwrap_or(false))
}

pub(crate) fn record_audit(app: &tauri::AppHandle, action: &str, outcome: &str) {
    if action.len() > 80 || outcome.len() > 20 {
        return;
    }
    if let Ok(connection) = open_database(app) {
        let _ = connection.execute(
            "INSERT INTO audit_events(action,outcome) VALUES(?1,?2)",
            params![action, outcome],
        );
        let _ = connection.execute(
            "DELETE FROM audit_events WHERE id NOT IN (SELECT id FROM audit_events ORDER BY id DESC LIMIT 2000)",
            [],
        );
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditEvent {
    action: String,
    outcome: String,
    created_at: i64,
}

#[tauri::command]
pub fn list_audit_events(app: tauri::AppHandle) -> Result<Vec<AuditEvent>, String> {
    let connection = open_database(&app)?;
    let mut statement = connection
        .prepare("SELECT action,outcome,created_at FROM audit_events ORDER BY id DESC LIMIT 20")
        .map_err(|_| "无法读取本地审计记录。".to_string())?;
    let records = statement
        .query_map([], |row| {
            Ok(AuditEvent {
                action: row.get(0)?,
                outcome: row.get(1)?,
                created_at: row.get(2)?,
            })
        })
        .map_err(|_| "无法读取本地审计记录。".to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "本地审计记录格式无效。".to_string())?;
    Ok(records)
}

#[tauri::command]
pub fn get_network_access(app: tauri::AppHandle) -> Result<bool, String> {
    network_access_enabled(&app)
}

#[tauri::command]
pub fn set_network_access(app: tauri::AppHandle, enabled: bool) -> Result<bool, String> {
    open_database(&app)?
        .execute(
            "INSERT INTO app_settings(key,value) VALUES('network_access_enabled',?1)
             ON CONFLICT(key) DO UPDATE SET value=excluded.value,updated_at=unixepoch()",
            params![if enabled { "true" } else { "false" }],
        )
        .map_err(|_| "无法更新网络访问总开关。".to_string())?;
    record_audit(
        &app,
        "security.network-toggle",
        if enabled { "enabled" } else { "disabled" },
    );
    Ok(enabled)
}

fn validate_backup_path(path: &str, existing: bool) -> Result<PathBuf, String> {
    let requested = Path::new(path);
    if requested.extension().and_then(|value| value.to_str()) != Some("icwb") {
        return Err("工作台备份必须使用 .icwb 扩展名。".to_string());
    }
    if existing {
        let metadata =
            fs::symlink_metadata(requested).map_err(|_| "无法读取所选备份文件。".to_string())?;
        if metadata.file_type().is_symlink()
            || !metadata.is_file()
            || metadata.len() > 1024 * 1024 * 1024
        {
            return Err("备份必须是普通文件、不能是符号链接，且不能超过 1 GiB。".to_string());
        }
        return requested
            .canonicalize()
            .map_err(|_| "无法验证所选备份文件。".to_string());
    }
    let parent = requested
        .parent()
        .filter(|value| !value.as_os_str().is_empty())
        .ok_or_else(|| "备份目录无效。".to_string())?
        .canonicalize()
        .map_err(|_| "备份目录不存在或不可访问。".to_string())?;
    let name = requested
        .file_name()
        .ok_or_else(|| "备份文件名无效。".to_string())?;
    let destination = parent.join(name);
    if destination.exists()
        && fs::symlink_metadata(&destination)
            .map(|metadata| metadata.file_type().is_symlink() || !metadata.is_file())
            .unwrap_or(true)
    {
        return Err("备份目标不是普通文件或指向符号链接。".to_string());
    }
    Ok(destination)
}

#[tauri::command]
pub fn backup_local_data(
    app: tauri::AppHandle,
    path: String,
    ui_state_json: String,
) -> Result<(), String> {
    if ui_state_json.len() > 3 * 1024 * 1024
        || !serde_json::from_str::<serde_json::Value>(&ui_state_json)
            .is_ok_and(|value| value.is_object())
    {
        return Err("界面配置快照无效或超过 3 MiB。".to_string());
    }
    let source = open_database(&app)?;
    source
        .execute(
            "INSERT INTO app_settings(key,value) VALUES('ui_backup_state',?1)
             ON CONFLICT(key) DO UPDATE SET value=excluded.value,updated_at=unixepoch()",
            params![ui_state_json],
        )
        .map_err(|_| "无法保存界面配置快照。".to_string())?;
    let destination_path = validate_backup_path(&path, false)?;
    let mut destination =
        Connection::open(destination_path).map_err(|_| "无法创建工作台备份。".to_string())?;
    let result = {
        let backup = rusqlite::backup::Backup::new(&source, &mut destination)
            .map_err(|_| "无法开始工作台备份。".to_string())?;
        backup
            .run_to_completion(64, Duration::from_millis(20), None)
            .map_err(|_| "工作台备份未能完成。".to_string())
    };
    record_audit(
        &app,
        "data.backup",
        if result.is_ok() { "success" } else { "failed" },
    );
    result
}

#[tauri::command]
pub fn restore_local_data(
    app: tauri::AppHandle,
    path: String,
    confirmed: bool,
) -> Result<String, String> {
    if !confirmed {
        return Err("恢复本地数据前必须完成人工二次确认。".to_string());
    }
    let source_path = validate_backup_path(&path, true)?;
    let source = Connection::open_with_flags(
        source_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|_| "所选文件不是可读取的 SQLite 工作台备份。".to_string())?;
    let version = source
        .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
        .map_err(|_| "无法读取备份版本。".to_string())?;
    if version <= 0 || version > SCHEMA_VERSION {
        return Err("备份版本无效或来自更高版本应用。".to_string());
    }
    let integrity = source
        .query_row("PRAGMA integrity_check", [], |row| row.get::<_, String>(0))
        .map_err(|_| "无法完成备份完整性检查。".to_string())?;
    if integrity != "ok" {
        return Err("备份未通过 SQLite 完整性检查。".to_string());
    }
    let required_tables = ["conversations", "projects", "skills", "mail_accounts"];
    for table in required_tables {
        let exists = source
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
                params![table],
                |row| row.get::<_, bool>(0),
            )
            .unwrap_or(false);
        if !exists {
            return Err("备份缺少必要的工作台数据表。".to_string());
        }
    }
    let mut destination = open_database(&app)?;
    {
        let backup = rusqlite::backup::Backup::new(&source, &mut destination)
            .map_err(|_| "无法开始恢复本地数据。".to_string())?;
        backup
            .run_to_completion(64, Duration::from_millis(20), None)
            .map_err(|_| "本地数据恢复未能完成。".to_string())?;
    }
    destination
        .execute(
            "INSERT INTO app_settings(key,value) VALUES('network_access_enabled','false')
             ON CONFLICT(key) DO UPDATE SET value='false',updated_at=unixepoch()",
            [],
        )
        .map_err(|_| "数据已恢复，但无法重置网络总开关。".to_string())?;
    initialize(&destination)?;
    let ui_state = destination
        .query_row(
            "SELECT value FROM app_settings WHERE key='ui_backup_state'",
            [],
            |row| row.get::<_, String>(0),
        )
        .unwrap_or_else(|_| "{}".to_string());
    record_audit(&app, "data.restore", "success");
    Ok(ui_state)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardSummary {
    project_count: i64,
    conversation_count: i64,
    skill_count: i64,
    mail_account_count: i64,
    cached_mail_count: i64,
    recent_projects: Vec<String>,
    recent_documents: Vec<String>,
    recent_conversations: Vec<String>,
}

fn string_list(connection: &Connection, sql: &str) -> Vec<String> {
    let Ok(mut statement) = connection.prepare(sql) else {
        return Vec::new();
    };
    statement
        .query_map([], |row| row.get::<_, String>(0))
        .ok()
        .map(|rows| rows.filter_map(Result::ok).collect())
        .unwrap_or_default()
}

#[tauri::command]
pub fn get_dashboard_summary(app: tauri::AppHandle) -> Result<DashboardSummary, String> {
    let connection = open_database(&app)?;
    let count = |table: &str| {
        connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap_or(0)
    };
    Ok(DashboardSummary {
        project_count: count("projects"),
        conversation_count: count("conversations"),
        skill_count: count("skills"),
        mail_account_count: count("mail_accounts"),
        cached_mail_count: count("mail_messages"),
        recent_projects: string_list(&connection, "SELECT name FROM projects ORDER BY updated_at DESC LIMIT 3"),
        recent_documents: string_list(&connection, "SELECT source_name FROM project_files WHERE document_id IS NOT NULL ORDER BY indexed_at DESC LIMIT 3"),
        recent_conversations: string_list(&connection, "SELECT title FROM conversations ORDER BY updated_at DESC LIMIT 3"),
    })
}

fn database_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|_| "无法确定本地数据目录。".to_string())?;
    fs::create_dir_all(&directory).map_err(|_| "无法创建本地数据目录。".to_string())?;
    Ok(directory.join("workbench.sqlite3"))
}

pub(crate) fn open_database(app: &tauri::AppHandle) -> Result<Connection, String> {
    let connection = Connection::open(database_path(app)?)
        .map_err(|_| "无法打开本地会话数据库。".to_string())?;
    initialize(&connection)?;
    Ok(connection)
}

fn save_with_connection(
    connection: &mut Connection,
    conversation: &StoredConversation,
) -> Result<(), String> {
    validate_conversation(conversation)?;
    let transaction = connection
        .transaction()
        .map_err(|_| "无法开始本地会话事务。".to_string())?;
    transaction
        .execute(
            "INSERT INTO conversations (id, title) VALUES (?1, ?2)
             ON CONFLICT(id) DO UPDATE SET title = excluded.title, updated_at = unixepoch()",
            params![conversation.id, conversation.title],
        )
        .map_err(|_| "无法保存本地会话。".to_string())?;
    transaction
        .execute(
            "DELETE FROM messages WHERE conversation_id = ?1",
            params![conversation.id],
        )
        .map_err(|_| "无法更新本地会话消息。".to_string())?;
    {
        let mut insert = transaction
            .prepare(
                "INSERT INTO messages (id, conversation_id, position, role, text, request_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )
            .map_err(|_| "无法准备本地会话写入。".to_string())?;
        let mut insert_attachment = transaction
            .prepare(
                "INSERT INTO attachments
                 (id, message_id, position, name, mime_type, data_url, size_bytes)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            )
            .map_err(|_| "无法准备本地附件写入。".to_string())?;
        let mut insert_document = transaction
            .prepare(
                "INSERT INTO document_attachments
                 (id, message_id, position, source_name, source_kind, markdown, warnings_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            )
            .map_err(|_| "无法准备本地文档附件写入。".to_string())?;
        for (position, message) in conversation.messages.iter().enumerate() {
            insert
                .execute(params![
                    message.id,
                    conversation.id,
                    position as i64,
                    message.role,
                    message.text,
                    message.request_id
                ])
                .map_err(|_| "无法保存本地会话消息。".to_string())?;
            for (attachment_position, attachment) in message.attachments.iter().enumerate() {
                insert_attachment
                    .execute(params![
                        attachment.id,
                        message.id,
                        attachment_position as i64,
                        attachment.name,
                        attachment.mime_type,
                        attachment.data_url,
                        attachment.size_bytes as i64
                    ])
                    .map_err(|_| "无法保存本地附件。".to_string())?;
            }
            for (document_position, document) in message.documents.iter().enumerate() {
                insert_document
                    .execute(params![
                        document.id,
                        message.id,
                        document_position as i64,
                        document.source_name,
                        document.source_kind,
                        document.markdown,
                        serde_json::to_string(&document.warnings)
                            .map_err(|_| "无法编码文档附件警告。".to_string())?
                    ])
                    .map_err(|_| "无法保存本地文档附件。".to_string())?;
            }
        }
    }
    transaction
        .commit()
        .map_err(|_| "无法提交本地会话。".to_string())
}

fn load_with_connection(connection: &Connection) -> Result<Vec<StoredConversation>, String> {
    let mut conversations_query = connection
        .prepare(
            "SELECT id, title FROM conversations ORDER BY updated_at DESC, rowid DESC LIMIT ?1",
        )
        .map_err(|_| "无法读取本地会话。".to_string())?;
    let rows = conversations_query
        .query_map(params![MAX_CONVERSATIONS as i64], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|_| "无法读取本地会话。".to_string())?;
    let summaries = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "本地会话数据格式无效。".to_string())?;

    let mut message_query = connection
        .prepare(
            "SELECT id, role, text, request_id FROM messages
             WHERE conversation_id = ?1 ORDER BY position ASC LIMIT ?2",
        )
        .map_err(|_| "无法读取本地会话消息。".to_string())?;
    summaries
        .into_iter()
        .map(|(id, title)| {
            let rows = message_query
                .query_map(params![id, MAX_STORED_MESSAGES as i64], |row| {
                    Ok(StoredMessage {
                        attachments: Vec::new(),
                        documents: Vec::new(),
                        id: row.get(0)?,
                        role: row.get(1)?,
                        text: row.get(2)?,
                        request_id: row.get(3)?,
                    })
                })
                .map_err(|_| "无法读取本地会话消息。".to_string())?;
            let mut messages = rows
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| "本地会话消息格式无效。".to_string())?;
            for message in &mut messages {
                let mut attachment_query = connection
                    .prepare(
                        "SELECT id, name, mime_type, data_url, size_bytes FROM attachments
                         WHERE message_id = ?1 ORDER BY position ASC LIMIT ?2",
                    )
                    .map_err(|_| "无法读取本地附件。".to_string())?;
                let rows = attachment_query
                    .query_map(
                        params![
                            message.id,
                            crate::attachments::MAX_ATTACHMENTS_PER_MESSAGE as i64
                        ],
                        |row| {
                            Ok(ChatAttachment {
                                id: row.get(0)?,
                                name: row.get(1)?,
                                mime_type: row.get(2)?,
                                data_url: row.get(3)?,
                                size_bytes: row.get::<_, i64>(4)? as usize,
                            })
                        },
                    )
                    .map_err(|_| "无法读取本地附件。".to_string())?;
                message.attachments = rows
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|_| "本地附件格式无效。".to_string())?;
                let mut document_query = connection
                    .prepare(
                        "SELECT id, source_name, source_kind, markdown, warnings_json
                         FROM document_attachments WHERE message_id = ?1
                         ORDER BY position ASC LIMIT ?2",
                    )
                    .map_err(|_| "无法读取本地文档附件。".to_string())?;
                let rows = document_query
                    .query_map(
                        params![message.id, MAX_DOCUMENT_ATTACHMENTS_PER_MESSAGE as i64],
                        |row| {
                            let warnings_json: String = row.get(4)?;
                            Ok(StoredDocumentAttachment {
                                id: row.get(0)?,
                                source_name: row.get(1)?,
                                source_kind: row.get(2)?,
                                markdown: row.get(3)?,
                                warnings: serde_json::from_str(&warnings_json).unwrap_or_default(),
                            })
                        },
                    )
                    .map_err(|_| "无法读取本地文档附件。".to_string())?;
                message.documents = rows
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|_| "本地文档附件格式无效。".to_string())?;
            }
            let conversation = StoredConversation {
                id,
                messages,
                title,
            };
            validate_conversation(&conversation)?;
            Ok(conversation)
        })
        .collect()
}

#[tauri::command]
pub(crate) fn load_conversations(app: tauri::AppHandle) -> Result<Vec<StoredConversation>, String> {
    load_with_connection(&open_database(&app)?)
}

#[tauri::command]
pub(crate) fn save_conversation(
    app: tauri::AppHandle,
    conversation: StoredConversation,
) -> Result<(), String> {
    save_with_connection(&mut open_database(&app)?, &conversation)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn conversation(text: &str) -> StoredConversation {
        StoredConversation {
            id: "conversation-test".into(),
            title: "测试会话".into(),
            messages: vec![StoredMessage {
                attachments: Vec::new(),
                documents: Vec::new(),
                id: "message-test".into(),
                request_id: Some("req-test".into()),
                role: "assistant".into(),
                text: text.into(),
            }],
        }
    }

    fn conversation_with_attachment() -> StoredConversation {
        StoredConversation {
            id: "conversation-image".into(),
            title: "图片会话".into(),
            messages: vec![StoredMessage {
                attachments: vec![ChatAttachment {
                    data_url: "data:image/png;base64,iVBORw0KGgpzeW50aGV0aWM=".into(),
                    id: "attachment-image".into(),
                    mime_type: "image/png".into(),
                    name: "synthetic.png".into(),
                    size_bytes: 17,
                }],
                documents: vec![StoredDocumentAttachment {
                    id: "document-notes".into(),
                    markdown: "# 合成说明\n\n偏置电流 10 uA。".into(),
                    source_kind: "DOCX".into(),
                    source_name: "synthetic.docx".into(),
                    warnings: vec!["合成样本".into()],
                }],
                id: "message-image".into(),
                request_id: None,
                role: "user".into(),
                text: "分析图片".into(),
            }],
        }
    }

    #[test]
    fn migrates_skill_manifest_columns_and_keeps_network_off_by_default() {
        let connection = Connection::open_in_memory().expect("memory database should open");
        connection
            .execute_batch(
                "CREATE TABLE skills (
                   id TEXT PRIMARY KEY,name TEXT NOT NULL,description TEXT NOT NULL,
                   instructions TEXT NOT NULL,source_name TEXT NOT NULL,enabled INTEGER NOT NULL DEFAULT 0,
                   has_scripts INTEGER NOT NULL DEFAULT 0,imported_at INTEGER NOT NULL DEFAULT 0,
                   updated_at INTEGER NOT NULL DEFAULT 0
                 );
                 PRAGMA user_version=6;",
            )
            .expect("legacy schema should initialize");
        initialize(&connection).expect("legacy schema should migrate");
        let version = connection
            .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
            .expect("version should read");
        assert_eq!(version, SCHEMA_VERSION);
        let columns = string_list(&connection, "SELECT name FROM pragma_table_info('skills')");
        assert!(columns.contains(&"permissions_json".to_string()));
        assert!(columns.contains(&"has_restricted_permissions".to_string()));
        assert_eq!(
            connection
                .query_row("SELECT COUNT(*) FROM app_settings", [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }

    #[test]
    fn sqlite_round_trip_and_transactional_update_work() {
        let mut connection = Connection::open_in_memory().expect("memory database should open");
        initialize(&connection).expect("schema should initialize");
        let version = connection
            .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
            .expect("schema version should be readable");
        assert_eq!(version, SCHEMA_VERSION);
        save_with_connection(&mut connection, &conversation("第一版"))
            .expect("conversation should save");
        save_with_connection(&mut connection, &conversation("流式完成"))
            .expect("conversation should update");

        let loaded = load_with_connection(&connection).expect("conversation should load");
        assert_eq!(loaded, vec![conversation("流式完成")]);
    }

    #[test]
    fn invalid_or_oversized_conversations_are_rejected() {
        let mut invalid = conversation("内容");
        invalid.id = "../outside".into();
        assert!(validate_conversation(&invalid).is_err());

        let mut oversized = conversation("内容");
        oversized.messages[0].text = "x".repeat(MAX_STORED_MESSAGE_LENGTH + 1);
        assert!(validate_conversation(&oversized).is_err());
    }

    #[test]
    fn image_attachments_survive_sqlite_round_trip() {
        let mut connection = Connection::open_in_memory().expect("memory database should open");
        initialize(&connection).expect("schema should initialize");
        let conversation = conversation_with_attachment();
        save_with_connection(&mut connection, &conversation)
            .expect("conversation attachment should save");

        assert_eq!(
            load_with_connection(&connection).expect("conversation attachment should load"),
            vec![conversation]
        );
    }
}
