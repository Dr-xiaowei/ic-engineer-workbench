use crate::{
    document::convert_document_bytes,
    host_is_intranet, ip_is_intranet, is_demo_mode,
    storage::{network_access_enabled, open_database, record_audit},
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use futures_util::TryStreamExt;
use keyring::{Entry, Error as KeyringError};
use mail_parser::{MessageParser, MimeHeaders};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    net::{SocketAddr, ToSocketAddrs},
    path::Path,
    time::Duration,
};
use tokio::{net::TcpStream, time::timeout};

const MAIL_CREDENTIAL_SERVICE: &str = "com.icengineer.workbench.mail";
const DEMO_MAIL_ID: &str = "mail-embedded-demo";
const MAX_ACCOUNTS: usize = 20;
const MAX_SYNC_MESSAGES: usize = 100;
const MAX_MAIL_BODY_BYTES: usize = 256 * 1024;
const MAX_MAIL_RAW_BYTES: usize = 8 * 1024 * 1024;
const MAX_ATTACHMENT_BYTES: usize = 10 * 1024 * 1024;
const MAX_ATTACHMENTS_PER_MAIL: usize = 20;
const MAX_SEND_RECIPIENTS: usize = 50;
const MAX_SEND_TOTAL_BYTES: usize = 20 * 1024 * 1024;

type ImapSession = async_imap::Session<tokio_native_tls::TlsStream<TcpStream>>;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MailAccountInput {
    id: Option<String>,
    address: String,
    display_name: String,
    username: String,
    imap_host: String,
    imap_port: u16,
    smtp_host: String,
    smtp_port: u16,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MailAccount {
    id: String,
    address: String,
    display_name: String,
    username: String,
    imap_host: String,
    imap_port: u16,
    smtp_host: String,
    smtp_port: u16,
    updated_at: i64,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct MailSecrets {
    imap_password: String,
    smtp_password: String,
}

fn validate_account_id(id: &str) -> Result<(), String> {
    let valid = id.starts_with("mail-")
        && id.len() <= 80
        && id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-');
    valid
        .then_some(())
        .ok_or_else(|| "邮箱账户标识无效。".to_string())
}

fn clean_host(host: &str) -> Result<String, String> {
    let host = host.trim().to_ascii_lowercase();
    if host.is_empty()
        || host.len() > 253
        || !host_is_intranet(&host)
        || !host.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | ':' | '[' | ']')
        })
    {
        return Err("邮箱服务器只允许本机、私有网段或内网主机名。".to_string());
    }
    Ok(host)
}

fn validated_addresses(host: &str, port: u16) -> Result<Vec<SocketAddr>, String> {
    let addresses = (host.trim_matches(['[', ']']), port)
        .to_socket_addrs()
        .map_err(|_| "无法解析内网邮箱服务器。".to_string())?
        .collect::<Vec<_>>();
    if addresses.is_empty()
        || addresses
            .iter()
            .any(|address| !ip_is_intranet(address.ip()))
    {
        return Err("邮箱服务器解析结果包含非内网地址，已拒绝连接。".to_string());
    }
    Ok(addresses)
}

fn normalize_input(input: MailAccountInput) -> Result<MailAccountInput, String> {
    let address = input.address.trim().to_string();
    if address.len() > 254 || address.parse::<lettre::Address>().is_err() {
        return Err("邮箱地址格式无效。".to_string());
    }
    let display_name = input
        .display_name
        .trim()
        .chars()
        .take(120)
        .collect::<String>();
    let username = input.username.trim().to_string();
    if display_name.is_empty() || username.is_empty() || username.len() > 254 {
        return Err("显示名称或登录用户名无效。".to_string());
    }
    if let Some(id) = &input.id {
        validate_account_id(id)?;
    }
    Ok(MailAccountInput {
        id: input.id,
        address,
        display_name,
        username,
        imap_host: clean_host(&input.imap_host)?,
        imap_port: input.imap_port,
        smtp_host: clean_host(&input.smtp_host)?,
        smtp_port: input.smtp_port,
    })
}

fn generated_id(input: &MailAccountInput) -> String {
    let digest = Sha256::digest(
        format!("{}|{}|{}", input.address, input.imap_host, input.smtp_host).as_bytes(),
    );
    format!(
        "mail-{}",
        digest[..12]
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    )
}

fn credential_entry(account_id: &str) -> Result<Entry, String> {
    validate_account_id(account_id)?;
    Entry::new(MAIL_CREDENTIAL_SERVICE, account_id)
        .map_err(|_| "无法访问系统邮箱凭据库。".to_string())
}

fn read_secrets(account_id: &str) -> Result<Option<MailSecrets>, String> {
    match credential_entry(account_id)?.get_password() {
        Ok(value) => serde_json::from_str(&value)
            .map(Some)
            .map_err(|_| "系统邮箱凭据格式无效。".to_string()),
        Err(KeyringError::NoEntry) => Ok(None),
        Err(_) => Err("无法读取系统邮箱凭据库。".to_string()),
    }
}

#[tauri::command]
pub fn save_mail_account(
    app: tauri::AppHandle,
    account: MailAccountInput,
) -> Result<MailAccount, String> {
    let account = normalize_input(account)?;
    let id = account.id.clone().unwrap_or_else(|| generated_id(&account));
    let connection = open_database(&app)?;
    let count = connection
        .query_row("SELECT COUNT(*) FROM mail_accounts", [], |row| {
            row.get::<_, i64>(0)
        })
        .map_err(|_| "无法读取邮箱账户数量。".to_string())? as usize;
    if count >= MAX_ACCOUNTS
        && !connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM mail_accounts WHERE id = ?1)",
                params![id],
                |row| row.get::<_, bool>(0),
            )
            .unwrap_or(false)
    {
        return Err("邮箱账户已达到 20 个上限。".to_string());
    }
    connection
        .execute(
            "INSERT INTO mail_accounts
             (id, address, display_name, username, imap_host, imap_port, smtp_host, smtp_port)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(id) DO UPDATE SET
               address=excluded.address, display_name=excluded.display_name,
               username=excluded.username, imap_host=excluded.imap_host,
               imap_port=excluded.imap_port, smtp_host=excluded.smtp_host,
               smtp_port=excluded.smtp_port, updated_at=unixepoch()",
            params![
                id,
                account.address,
                account.display_name,
                account.username,
                account.imap_host,
                account.imap_port,
                account.smtp_host,
                account.smtp_port
            ],
        )
        .map_err(|_| "无法保存邮箱账户元数据。".to_string())?;
    let result = load_account(&connection, &id);
    record_audit(
        &app,
        "mail.account-save",
        if result.is_ok() { "success" } else { "failed" },
    );
    result
}

fn load_account(connection: &rusqlite::Connection, id: &str) -> Result<MailAccount, String> {
    connection
        .query_row(
            "SELECT id, address, display_name, username, imap_host, imap_port,
                    smtp_host, smtp_port, updated_at FROM mail_accounts WHERE id=?1",
            params![id],
            |row| {
                Ok(MailAccount {
                    id: row.get(0)?,
                    address: row.get(1)?,
                    display_name: row.get(2)?,
                    username: row.get(3)?,
                    imap_host: row.get(4)?,
                    imap_port: row.get::<_, i64>(5)? as u16,
                    smtp_host: row.get(6)?,
                    smtp_port: row.get::<_, i64>(7)? as u16,
                    updated_at: row.get(8)?,
                })
            },
        )
        .map_err(|_| "邮箱账户不存在或记录无效。".to_string())
}

async fn connect_tcp(host: &str, port: u16) -> Result<TcpStream, String> {
    let addresses = validated_addresses(host, port)?;
    let mut last_error = None;
    for address in addresses {
        match timeout(Duration::from_secs(6), TcpStream::connect(address)).await {
            Ok(Ok(stream)) => return Ok(stream),
            Ok(Err(error)) => last_error = Some(error.to_string()),
            Err(_) => last_error = Some("连接超时".to_string()),
        }
    }
    Err(format!(
        "无法连接内网邮箱服务器：{}",
        last_error.unwrap_or_default()
    ))
}

async fn open_imap_session(account: &MailAccount, password: &str) -> Result<ImapSession, String> {
    let tcp = connect_tcp(&account.imap_host, account.imap_port).await?;
    let connector = native_tls::TlsConnector::builder()
        .build()
        .map_err(|_| "无法初始化系统 TLS。".to_string())?;
    let tls = timeout(
        Duration::from_secs(8),
        tokio_native_tls::TlsConnector::from(connector)
            .connect(account.imap_host.trim_matches(['[', ']']), tcp),
    )
    .await
    .map_err(|_| "IMAP TLS 握手超时。".to_string())?
    .map_err(|_| "IMAP TLS 证书或握手验证失败。".to_string())?;
    let mut client = async_imap::Client::new(tls);
    timeout(Duration::from_secs(8), client.read_response())
        .await
        .map_err(|_| "IMAP 服务问候超时。".to_string())?
        .map_err(|_| "无法读取 IMAP 服务问候。".to_string())?;
    timeout(
        Duration::from_secs(8),
        client.login(&account.username, password),
    )
    .await
    .map_err(|_| "IMAP 登录超时。".to_string())?
    .map_err(|_| "IMAP 用户名或密码未通过验证。".to_string())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MailConnectionResult {
    imap_message: String,
    smtp_message: String,
    success: bool,
}

#[tauri::command]
pub async fn test_mail_account(
    app: tauri::AppHandle,
    account_id: String,
) -> Result<MailConnectionResult, String> {
    if !network_access_enabled(&app)? {
        record_audit(&app, "mail.connection-test", "blocked");
        return Err("网络访问总开关已关闭；未连接邮箱服务器。".to_string());
    }
    validate_account_id(&account_id)?;
    if is_demo_mode() && account_id == DEMO_MAIL_ID {
        record_audit(&app, "mail.connection-test", "success");
        return Ok(MailConnectionResult {
            success: true,
            imap_message: "Demo 合成收件箱已就绪；未建立网络连接。".to_string(),
            smtp_message: "Demo 发送模拟已就绪；不会投递真实邮件。".to_string(),
        });
    }
    let account = load_account(&open_database(&app)?, &account_id)?;
    let secrets = read_secrets(&account_id)?
        .ok_or_else(|| "请先把 IMAP/SMTP 密码保存到系统凭据库。".to_string())?;

    let mut session = open_imap_session(&account, &secrets.imap_password).await?;
    timeout(Duration::from_secs(8), session.select("INBOX"))
        .await
        .map_err(|_| "IMAP 收件箱检查超时。".to_string())?
        .map_err(|_| "IMAP 收件箱不可用。".to_string())?;
    let _ = session.logout().await;

    validated_addresses(&account.smtp_host, account.smtp_port)?;
    let smtp_host = account.smtp_host.clone();
    let smtp_port = account.smtp_port;
    let smtp_username = account.username.clone();
    let smtp_password = secrets.smtp_password;
    let smtp_ok = tauri::async_runtime::spawn_blocking(move || {
        use lettre::{transport::smtp::authentication::Credentials, SmtpTransport};
        let builder = if smtp_port == 465 {
            SmtpTransport::relay(&smtp_host)
        } else {
            SmtpTransport::starttls_relay(&smtp_host)
        }
        .map_err(|_| "无法初始化 SMTP TLS。".to_string())?;
        builder
            .port(smtp_port)
            .timeout(Some(Duration::from_secs(8)))
            .credentials(Credentials::new(smtp_username, smtp_password))
            .build()
            .test_connection()
            .map_err(|_| "SMTP 连接、TLS 或认证预检失败。".to_string())
    })
    .await
    .map_err(|_| "SMTP 检测任务失败。".to_string())??;

    record_audit(&app, "mail.connection-test", "success");
    Ok(MailConnectionResult {
        success: smtp_ok,
        imap_message: "IMAP TLS、登录与 INBOX 检查通过。".to_string(),
        smtp_message: if smtp_ok {
            "SMTP TLS 连接预检通过。"
        } else {
            "SMTP 服务未确认连接。"
        }
        .to_string(),
    })
}

fn validate_folder(folder: &str) -> Result<(), String> {
    if folder.is_empty() || folder.len() > 200 || folder.contains(['\r', '\n', '\0']) {
        return Err("邮件文件夹名称无效。".to_string());
    }
    Ok(())
}

fn address_text(address: Option<&mail_parser::Address<'_>>) -> String {
    address
        .map(|value| {
            value
                .iter()
                .take(30)
                .filter_map(|item| {
                    item.address().map(|email| match item.name() {
                        Some(name) => format!("{name} <{email}>"),
                        None => email.to_string(),
                    })
                })
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default()
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MailAttachmentSummary {
    position: usize,
    name: String,
    mime_type: String,
    size_bytes: usize,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MailMessage {
    account_id: String,
    folder: String,
    uid: u32,
    message_id: Option<String>,
    subject: String,
    sender: String,
    recipients: String,
    sent_at: Option<i64>,
    preview: String,
    body_text: String,
    unread: bool,
    attachments: Vec<MailAttachmentSummary>,
}

struct ParsedMail {
    record: MailMessage,
    attachment_data: Vec<Vec<u8>>,
}

fn parse_mail(
    account_id: &str,
    folder: &str,
    uid: u32,
    unread: bool,
    raw: &[u8],
) -> Result<ParsedMail, String> {
    if raw.len() > MAX_MAIL_RAW_BYTES {
        return Err("单封邮件超过 8 MiB 同步上限。".to_string());
    }
    let message = MessageParser::default()
        .parse(raw)
        .ok_or_else(|| "邮件 MIME 格式无法解析。".to_string())?;
    let body = message
        .body_text(0)
        .unwrap_or_default()
        .chars()
        .take(MAX_MAIL_BODY_BYTES)
        .collect::<String>();
    let mut summaries = Vec::new();
    let mut attachment_data = Vec::new();
    for (position, attachment) in message
        .attachments()
        .take(MAX_ATTACHMENTS_PER_MAIL)
        .enumerate()
    {
        if attachment.len() > MAX_ATTACHMENT_BYTES {
            continue;
        }
        let content_type = attachment.content_type();
        let mime_type = content_type
            .map(|value| {
                format!(
                    "{}/{}",
                    value.ctype(),
                    value.subtype().unwrap_or("octet-stream")
                )
            })
            .unwrap_or_else(|| "application/octet-stream".to_string());
        let name = attachment
            .attachment_name()
            .unwrap_or("attachment.bin")
            .chars()
            .take(180)
            .collect::<String>();
        summaries.push(MailAttachmentSummary {
            position,
            name,
            mime_type,
            size_bytes: attachment.len(),
        });
        attachment_data.push(attachment.contents().to_vec());
    }
    Ok(ParsedMail {
        record: MailMessage {
            account_id: account_id.to_string(),
            folder: folder.to_string(),
            uid,
            message_id: message
                .message_id()
                .map(|value| value.chars().take(500).collect()),
            subject: message
                .subject()
                .unwrap_or("（无主题）")
                .chars()
                .take(500)
                .collect(),
            sender: address_text(message.from()),
            recipients: address_text(message.to()),
            sent_at: message.date().map(|value| value.to_timestamp()),
            preview: body.chars().take(240).collect(),
            body_text: body,
            unread,
            attachments: summaries,
        },
        attachment_data,
    })
}

fn save_synced(connection: &mut Connection, parsed: &[ParsedMail]) -> Result<(), String> {
    let transaction = connection
        .transaction()
        .map_err(|_| "无法开始邮件同步事务。".to_string())?;
    for mail in parsed {
        let record = &mail.record;
        transaction
            .execute(
                "DELETE FROM mail_search WHERE account_id=?1 AND folder=?2 AND uid=?3",
                params![record.account_id, record.folder, record.uid],
            )
            .map_err(|_| "无法更新邮件搜索索引。".to_string())?;
        transaction.execute(
            "INSERT INTO mail_messages (account_id,folder,uid,message_id,subject,sender,recipients,sent_at,preview,body_text,unread,raw_size)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)
             ON CONFLICT(account_id,folder,uid) DO UPDATE SET message_id=excluded.message_id,subject=excluded.subject,
             sender=excluded.sender,recipients=excluded.recipients,sent_at=excluded.sent_at,preview=excluded.preview,
             body_text=excluded.body_text,unread=excluded.unread,raw_size=excluded.raw_size,synced_at=unixepoch()",
            params![record.account_id, record.folder, record.uid, record.message_id, record.subject, record.sender,
                record.recipients, record.sent_at, record.preview, record.body_text, record.unread as i64,
                record.body_text.len() as i64]).map_err(|_| "无法保存本地邮件。".to_string())?;
        transaction
            .execute(
                "DELETE FROM mail_attachments WHERE account_id=?1 AND folder=?2 AND uid=?3",
                params![record.account_id, record.folder, record.uid],
            )
            .map_err(|_| "无法更新邮件附件。".to_string())?;
        for (summary, data) in record.attachments.iter().zip(&mail.attachment_data) {
            transaction.execute("INSERT INTO mail_attachments (account_id,folder,uid,position,name,mime_type,size_bytes,data) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
                params![record.account_id, record.folder, record.uid, summary.position as i64, summary.name,
                    summary.mime_type, summary.size_bytes as i64, data]).map_err(|_| "无法保存邮件附件。".to_string())?;
        }
        transaction.execute("INSERT INTO mail_search (account_id,folder,uid,subject,sender,recipients,body_text) VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![record.account_id, record.folder, record.uid, record.subject, record.sender, record.recipients, record.body_text])
            .map_err(|_| "无法写入邮件全文索引。".to_string())?;
    }
    transaction
        .commit()
        .map_err(|_| "无法提交邮件同步事务。".to_string())
}

#[tauri::command]
pub async fn list_mail_folders(
    app: tauri::AppHandle,
    account_id: String,
) -> Result<Vec<String>, String> {
    if !network_access_enabled(&app)? {
        record_audit(&app, "mail.folder-list", "blocked");
        return Err("网络访问总开关已关闭；未连接邮箱服务器。".to_string());
    }
    validate_account_id(&account_id)?;
    if is_demo_mode() && account_id == DEMO_MAIL_ID {
        return Ok(vec!["INBOX".to_string(), "Sent".to_string()]);
    }
    let account = load_account(&open_database(&app)?, &account_id)?;
    let secrets = read_secrets(&account_id)?.ok_or_else(|| "邮箱凭据尚未保存。".to_string())?;
    let mut session = open_imap_session(&account, &secrets.imap_password).await?;
    let folders = session
        .list(None, Some("*"))
        .await
        .map_err(|_| "无法列出 IMAP 文件夹。".to_string())?
        .try_collect::<Vec<_>>()
        .await
        .map_err(|_| "无法读取 IMAP 文件夹。".to_string())?
        .into_iter()
        .map(|item| item.name().to_string())
        .take(100)
        .collect();
    let _ = session.logout().await;
    Ok(folders)
}

#[tauri::command]
pub async fn sync_mail_folder(
    app: tauri::AppHandle,
    account_id: String,
    folder: String,
) -> Result<Vec<MailMessage>, String> {
    if !network_access_enabled(&app)? {
        record_audit(&app, "mail.folder-sync", "blocked");
        return Err("网络访问总开关已关闭；未同步邮件。".to_string());
    }
    validate_account_id(&account_id)?;
    validate_folder(&folder)?;
    if is_demo_mode() && account_id == DEMO_MAIL_ID {
        record_audit(&app, "mail.folder-sync", "success");
        return local_messages(&open_database(&app)?, &account_id, &folder, None);
    }
    let account = load_account(&open_database(&app)?, &account_id)?;
    let secrets = read_secrets(&account_id)?.ok_or_else(|| "邮箱凭据尚未保存。".to_string())?;
    let mut session = open_imap_session(&account, &secrets.imap_password).await?;
    session
        .examine(&folder)
        .await
        .map_err(|_| "无法只读打开 IMAP 文件夹。".to_string())?;
    let mut uids = session
        .uid_search("ALL")
        .await
        .map_err(|_| "无法搜索 IMAP 邮件 UID。".to_string())?
        .into_iter()
        .collect::<Vec<_>>();
    uids.sort_unstable();
    let uid_set = uids
        .into_iter()
        .rev()
        .take(MAX_SYNC_MESSAGES)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|uid| uid.to_string())
        .collect::<Vec<_>>()
        .join(",");
    if uid_set.is_empty() {
        let _ = session.logout().await;
        return Ok(Vec::new());
    }
    let fetches = session
        .uid_fetch(uid_set, "(UID FLAGS RFC822.SIZE BODY.PEEK[])")
        .await
        .map_err(|_| "无法请求 IMAP 邮件。".to_string())?
        .try_collect::<Vec<_>>()
        .await
        .map_err(|_| "IMAP 邮件接收中断。".to_string())?;
    let mut parsed = Vec::new();
    for fetch in fetches {
        let Some(uid) = fetch.uid else { continue };
        let Some(raw) = fetch.body() else { continue };
        let unread = !fetch
            .flags()
            .any(|flag| matches!(flag, async_imap::types::Flag::Seen));
        if let Ok(mail) = parse_mail(&account_id, &folder, uid, unread, raw) {
            parsed.push(mail);
        }
    }
    let _ = session.logout().await;
    save_synced(&mut open_database(&app)?, &parsed)?;
    record_audit(&app, "mail.folder-sync", "success");
    Ok(parsed.into_iter().map(|mail| mail.record).collect())
}

fn load_attachment_summaries(
    connection: &Connection,
    account_id: &str,
    folder: &str,
    uid: u32,
) -> Vec<MailAttachmentSummary> {
    let Ok(mut query) = connection.prepare("SELECT position,name,mime_type,size_bytes FROM mail_attachments WHERE account_id=?1 AND folder=?2 AND uid=?3 ORDER BY position") else { return Vec::new() };
    query
        .query_map(params![account_id, folder, uid], |row| {
            Ok(MailAttachmentSummary {
                position: row.get::<_, i64>(0)? as usize,
                name: row.get(1)?,
                mime_type: row.get(2)?,
                size_bytes: row.get::<_, i64>(3)? as usize,
            })
        })
        .ok()
        .map(|rows| rows.filter_map(Result::ok).collect())
        .unwrap_or_default()
}

fn local_messages(
    connection: &Connection,
    account_id: &str,
    folder: &str,
    query_text: Option<&str>,
) -> Result<Vec<MailMessage>, String> {
    let (sql, query_value) = if let Some(value) = query_text {
        if value.trim().is_empty() || value.len() > 500 {
            return Err("邮件搜索词无效。".to_string());
        }
        ("SELECT m.account_id,m.folder,m.uid,m.message_id,m.subject,m.sender,m.recipients,m.sent_at,m.preview,m.body_text,m.unread FROM mail_search s JOIN mail_messages m ON m.account_id=s.account_id AND m.folder=s.folder AND m.uid=s.uid WHERE mail_search MATCH ?3 AND s.account_id=?1 AND s.folder=?2 ORDER BY m.sent_at DESC LIMIT 100", format!("\"{}\"", value.replace('"', "\"\"")))
    } else {
        ("SELECT account_id,folder,uid,message_id,subject,sender,recipients,sent_at,preview,body_text,unread FROM mail_messages WHERE account_id=?1 AND folder=?2 ORDER BY sent_at DESC,uid DESC LIMIT 100", String::new())
    };
    let mut statement = connection
        .prepare(sql)
        .map_err(|_| "无法读取本地邮件。".to_string())?;
    let map = |row: &rusqlite::Row<'_>| {
        Ok(MailMessage {
            account_id: row.get(0)?,
            folder: row.get(1)?,
            uid: row.get::<_, i64>(2)? as u32,
            message_id: row.get(3)?,
            subject: row.get(4)?,
            sender: row.get(5)?,
            recipients: row.get(6)?,
            sent_at: row.get(7)?,
            preview: row.get(8)?,
            body_text: row.get(9)?,
            unread: row.get::<_, i64>(10)? != 0,
            attachments: Vec::new(),
        })
    };
    let mut records = if query_text.is_some() {
        statement.query_map(params![account_id, folder, query_value], map)
    } else {
        statement.query_map(params![account_id, folder], map)
    }
    .map_err(|_| "无法查询本地邮件。".to_string())?
    .collect::<Result<Vec<_>, _>>()
    .map_err(|_| "本地邮件格式无效。".to_string())?;
    for record in &mut records {
        record.attachments = load_attachment_summaries(connection, account_id, folder, record.uid);
    }
    Ok(records)
}

#[tauri::command]
pub fn list_local_mail(
    app: tauri::AppHandle,
    account_id: String,
    folder: String,
) -> Result<Vec<MailMessage>, String> {
    validate_account_id(&account_id)?;
    validate_folder(&folder)?;
    local_messages(&open_database(&app)?, &account_id, &folder, None)
}

#[tauri::command]
pub fn search_local_mail(
    app: tauri::AppHandle,
    account_id: String,
    folder: String,
    query: String,
) -> Result<Vec<MailMessage>, String> {
    validate_account_id(&account_id)?;
    validate_folder(&folder)?;
    local_messages(&open_database(&app)?, &account_id, &folder, Some(&query))
}

fn safe_export_destination(path: &str) -> Result<std::path::PathBuf, String> {
    let destination = Path::new(path);
    let parent = destination
        .parent()
        .filter(|value| !value.as_os_str().is_empty())
        .ok_or_else(|| "附件导出目录无效。".to_string())?;
    let canonical_parent = parent
        .canonicalize()
        .map_err(|_| "附件导出目录不存在或不可访问。".to_string())?;
    let file_name = destination
        .file_name()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "附件导出文件名无效。".to_string())?;
    let result = canonical_parent.join(file_name);
    if result.exists()
        && fs::symlink_metadata(&result)
            .map(|metadata| metadata.file_type().is_symlink() || !metadata.is_file())
            .unwrap_or(true)
    {
        return Err("附件导出目标不是普通文件或指向符号链接。".to_string());
    }
    Ok(result)
}

#[tauri::command]
pub fn export_mail_attachment(
    app: tauri::AppHandle,
    account_id: String,
    folder: String,
    uid: u32,
    position: usize,
    path: String,
) -> Result<(), String> {
    validate_account_id(&account_id)?;
    validate_folder(&folder)?;
    if position >= MAX_ATTACHMENTS_PER_MAIL {
        return Err("邮件附件位置无效。".to_string());
    }
    let connection = open_database(&app)?;
    let data = connection
        .query_row(
            "SELECT data FROM mail_attachments WHERE account_id=?1 AND folder=?2 AND uid=?3 AND position=?4",
            params![account_id, folder, uid, position as i64],
            |row| row.get::<_, Vec<u8>>(0),
        )
        .map_err(|_| "本地邮件附件不存在。".to_string())?;
    if data.len() > MAX_ATTACHMENT_BYTES {
        return Err("邮件附件超过 10 MiB 导出上限。".to_string());
    }
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(safe_export_destination(&path)?)
        .map_err(|_| "无法创建附件文件。".to_string())?;
    file.write_all(&data)
        .and_then(|_| file.sync_all())
        .map_err(|_| "写入邮件附件失败。".to_string())
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MailAttachmentPreview {
    kind: String,
    name: String,
    content: String,
}

#[tauri::command]
pub fn preview_mail_attachment(
    app: tauri::AppHandle,
    account_id: String,
    folder: String,
    uid: u32,
    position: usize,
) -> Result<MailAttachmentPreview, String> {
    validate_account_id(&account_id)?;
    validate_folder(&folder)?;
    if position >= MAX_ATTACHMENTS_PER_MAIL {
        return Err("邮件附件位置无效。".to_string());
    }
    let connection = open_database(&app)?;
    let (name, data):(String,Vec<u8>) = connection.query_row(
        "SELECT name,data FROM mail_attachments WHERE account_id=?1 AND folder=?2 AND uid=?3 AND position=?4",
        params![account_id,folder,uid,position as i64], |row| Ok((row.get(0)?,row.get(1)?))
    ).map_err(|_|"本地邮件附件不存在。".to_string())?;
    if data.len() > MAX_ATTACHMENT_BYTES {
        return Err("邮件附件超过 10 MiB 预览上限。".to_string());
    }
    let extension = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    if extension == "pdf" {
        if !data.starts_with(b"%PDF-") {
            return Err("附件扩展名与 PDF 内容不匹配。".to_string());
        }
        return Ok(MailAttachmentPreview {
            kind: "pdf".into(),
            name,
            content: format!("data:application/pdf;base64,{}", STANDARD.encode(data)),
        });
    }
    if !matches!(extension.as_str(), "md" | "txt" | "docx") {
        return Err("此附件类型不支持内嵌预览，请导出后使用受信任软件打开。".to_string());
    }
    let converted = convert_document_bytes(&name, &extension, &data)?;
    Ok(MailAttachmentPreview {
        kind: "markdown".into(),
        name,
        content: converted.markdown,
    })
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MailDraft {
    to: Vec<String>,
    cc: Vec<String>,
    subject: String,
    body: String,
    attachment_paths: Vec<String>,
}

fn parse_recipients(values: &[String]) -> Result<Vec<lettre::message::Mailbox>, String> {
    if values.is_empty() || values.len() > MAX_SEND_RECIPIENTS {
        return Err("收件人不能为空且不能超过 50 个。".to_string());
    }
    values
        .iter()
        .map(|value| {
            value
                .trim()
                .parse::<lettre::message::Mailbox>()
                .map_err(|_| "收件人邮箱格式无效。".to_string())
        })
        .collect()
}

fn build_outgoing_message(
    account: &MailAccount,
    draft: &MailDraft,
) -> Result<lettre::Message, String> {
    use lettre::message::{header::ContentType, Attachment, Mailbox, MultiPart, SinglePart};

    if draft.subject.len() > 500 || draft.body.is_empty() || draft.body.len() > 1024 * 1024 {
        return Err("邮件主题不能超过 500 字符，正文不能为空且不能超过 1 MiB。".to_string());
    }
    if draft.attachment_paths.len() > MAX_ATTACHMENTS_PER_MAIL {
        return Err("发送附件不能超过 20 个。".to_string());
    }
    let recipients = parse_recipients(&draft.to)?;
    let cc = if draft.cc.is_empty() {
        Vec::new()
    } else {
        parse_recipients(&draft.cc)?
    };
    let from = Mailbox::new(
        Some(account.display_name.clone()),
        account
            .address
            .parse()
            .map_err(|_| "发件人邮箱格式无效。".to_string())?,
    );
    let mut builder = lettre::Message::builder()
        .from(from)
        .subject(draft.subject.trim());
    for recipient in recipients {
        builder = builder.to(recipient);
    }
    for recipient in cc {
        builder = builder.cc(recipient);
    }

    let mut multipart = MultiPart::mixed().singlepart(SinglePart::plain(draft.body.clone()));
    let mut total_bytes = draft.body.len();
    for selected in &draft.attachment_paths {
        let requested = Path::new(selected);
        let metadata =
            fs::symlink_metadata(requested).map_err(|_| "无法读取待发送附件。".to_string())?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err("待发送附件必须是普通文件且不能是符号链接。".to_string());
        }
        let canonical = requested
            .canonicalize()
            .map_err(|_| "无法验证待发送附件。".to_string())?;
        let data = fs::read(&canonical).map_err(|_| "无法读取待发送附件。".to_string())?;
        if data.len() > MAX_ATTACHMENT_BYTES {
            return Err("单个发送附件不能超过 10 MiB。".to_string());
        }
        total_bytes = total_bytes.saturating_add(data.len());
        if total_bytes > MAX_SEND_TOTAL_BYTES {
            return Err("邮件正文和附件总计不能超过 20 MiB。".to_string());
        }
        let name = canonical
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("attachment.bin")
            .chars()
            .take(180)
            .collect::<String>();
        multipart = multipart.singlepart(
            Attachment::new(name).body(
                data,
                ContentType::parse("application/octet-stream")
                    .map_err(|_| "无法构建附件类型。".to_string())?,
            ),
        );
    }
    builder
        .multipart(multipart)
        .map_err(|_| "无法构建待发送邮件。".to_string())
}

#[tauri::command]
pub async fn send_mail(
    app: tauri::AppHandle,
    account_id: String,
    draft: MailDraft,
    confirmed: bool,
) -> Result<(), String> {
    if !confirmed {
        record_audit(&app, "mail.send", "blocked");
        return Err("发送邮件前必须完成人工二次确认。".to_string());
    }
    if !network_access_enabled(&app)? {
        record_audit(&app, "mail.send", "blocked");
        return Err("网络访问总开关已关闭；邮件未发送。".to_string());
    }
    validate_account_id(&account_id)?;
    let account = load_account(&open_database(&app)?, &account_id)?;
    if is_demo_mode() && account_id == DEMO_MAIL_ID {
        build_outgoing_message(&account, &draft)?;
        record_audit(&app, "mail.send", "success");
        return Ok(());
    }
    validated_addresses(&account.smtp_host, account.smtp_port)?;
    let secrets = read_secrets(&account_id)?.ok_or_else(|| "邮箱凭据尚未保存。".to_string())?;
    let message = build_outgoing_message(&account, &draft)?;
    let smtp_host = account.smtp_host;
    let smtp_port = account.smtp_port;
    let username = account.username;
    let result = tauri::async_runtime::spawn_blocking(move || {
        use lettre::{transport::smtp::authentication::Credentials, SmtpTransport, Transport};
        let builder = if smtp_port == 465 {
            SmtpTransport::relay(&smtp_host)
        } else {
            SmtpTransport::starttls_relay(&smtp_host)
        }
        .map_err(|_| "无法初始化 SMTP TLS。".to_string())?;
        builder
            .port(smtp_port)
            .timeout(Some(Duration::from_secs(15)))
            .credentials(Credentials::new(username, secrets.smtp_password))
            .build()
            .send(&message)
            .map(|_| ())
            .map_err(|_| "SMTP 发送失败；邮件未标记为已发送。".to_string())
    })
    .await
    .map_err(|_| "SMTP 发送任务失败。".to_string())?;
    record_audit(
        &app,
        "mail.send",
        if result.is_ok() { "success" } else { "failed" },
    );
    result
}

#[tauri::command]
pub fn list_mail_accounts(app: tauri::AppHandle) -> Result<Vec<MailAccount>, String> {
    let connection = open_database(&app)?;
    let mut query = connection
        .prepare("SELECT id FROM mail_accounts ORDER BY updated_at DESC LIMIT 20")
        .map_err(|_| "无法读取邮箱账户。".to_string())?;
    let ids = query
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|_| "无法读取邮箱账户。".to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "邮箱账户记录无效。".to_string())?;
    ids.iter().map(|id| load_account(&connection, id)).collect()
}

#[tauri::command]
pub fn save_mail_secrets(
    app: tauri::AppHandle,
    account_id: String,
    imap_password: String,
    smtp_password: String,
) -> Result<(), String> {
    let result = (|| {
        validate_account_id(&account_id)?;
        if imap_password.is_empty()
            || smtp_password.is_empty()
            || imap_password.len() > 16_384
            || smtp_password.len() > 16_384
        {
            return Err("IMAP/SMTP 密码不能为空且各自不能超过 16 KiB。".to_string());
        }
        let value = serde_json::to_string(&MailSecrets {
            imap_password,
            smtp_password,
        })
        .map_err(|_| "无法编码邮箱凭据。".to_string())?;
        credential_entry(&account_id)?
            .set_password(&value)
            .map_err(|_| "无法写入系统邮箱凭据库。".to_string())
    })();
    record_audit(
        &app,
        "mail.credential-save",
        if result.is_ok() { "success" } else { "failed" },
    );
    result
}

#[tauri::command]
pub fn has_mail_secrets(account_id: String) -> Result<bool, String> {
    if is_demo_mode() && account_id == DEMO_MAIL_ID {
        return Ok(true);
    }
    Ok(read_secrets(&account_id)?.is_some())
}

#[tauri::command]
pub fn seed_demo_mail(app: tauri::AppHandle) -> Result<(), String> {
    if !is_demo_mode() {
        return Err("该命令仅供独立 Demo 构建使用。".to_string());
    }
    let mut connection = open_database(&app)?;
    connection
        .execute(
            "INSERT INTO mail_accounts
             (id,address,display_name,username,imap_host,imap_port,smtp_host,smtp_port)
             VALUES (?1,'engineer@demo.invalid','合成演示邮箱','demo','127.0.0.1',993,'127.0.0.1',465)
             ON CONFLICT(id) DO NOTHING",
            params![DEMO_MAIL_ID],
        )
        .map_err(|_| "无法初始化 Demo 邮箱账户。".to_string())?;
    let samples = [
        (101_u32, b"From: Design Review <review@demo.invalid>\r\nTo: engineer@demo.invalid\r\nSubject: [Demo] OTA bias review\r\nMessage-ID: <demo-101@local>\r\nDate: Sat, 5 Sep 2026 09:30:00 +0800\r\nContent-Type: text/plain; charset=utf-8\r\n\r\nSynthetic review: verify bias current at TT, 27 C, VDD=1.8 V.\r\nThis message contains no real project data.".as_slice()),
        (102_u32, b"From: Lab Summary <lab@demo.invalid>\r\nTo: engineer@demo.invalid\r\nSubject: [Demo] settling-time checklist\r\nMessage-ID: <demo-102@local>\r\nDate: Sat, 5 Sep 2026 10:15:00 +0800\r\nMIME-Version: 1.0\r\nContent-Type: multipart/mixed; boundary=demo-boundary\r\n\r\n--demo-boundary\r\nContent-Type: text/plain; charset=utf-8\r\n\r\nSynthetic checklist: retain units, conditions and source page before drawing conclusions.\r\nNo measurement or EDA result is represented.\r\n--demo-boundary\r\nContent-Type: text/markdown; charset=utf-8\r\nContent-Disposition: attachment; filename=synthetic-review.md\r\n\r\n# Synthetic review\r\n\r\n- VDD: 1.8 V\r\n- Status: review required\r\n--demo-boundary--\r\n".as_slice()),
    ];
    let parsed = samples
        .into_iter()
        .map(|(uid, raw)| parse_mail(DEMO_MAIL_ID, "INBOX", uid, uid == 102, raw))
        .collect::<Result<Vec<_>, _>>()?;
    save_synced(&mut connection, &parsed)?;
    record_audit(&app, "mail.demo-seed", "success");
    Ok(())
}

#[tauri::command]
pub fn delete_mail_account(app: tauri::AppHandle, account_id: String) -> Result<(), String> {
    validate_account_id(&account_id)?;
    let connection = open_database(&app)?;
    connection
        .execute(
            "DELETE FROM mail_search WHERE account_id=?1",
            params![account_id],
        )
        .and_then(|_| {
            connection.execute("DELETE FROM mail_accounts WHERE id=?1", params![account_id])
        })
        .map_err(|_| "无法移除邮箱账户记录。".to_string())?;
    let result = match credential_entry(&account_id)?.delete_credential() {
        Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
        Err(_) => Err("账户记录已移除，但系统凭据清理失败。".to_string()),
    };
    record_audit(
        &app,
        "mail.account-delete",
        if result.is_ok() { "success" } else { "failed" },
    );
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::IpAddr;

    #[test]
    fn account_metadata_and_network_targets_are_bounded() {
        let valid = MailAccountInput {
            id: None,
            address: "engineer@example.internal".into(),
            display_name: "工程邮箱".into(),
            username: "engineer".into(),
            imap_host: "mail.internal".into(),
            imap_port: 993,
            smtp_host: "mail.internal".into(),
            smtp_port: 465,
        };
        assert!(normalize_input(valid).is_ok());
        assert!(clean_host("smtp.example.com").is_err());
        assert!(validate_account_id("../mail").is_err());
    }

    #[test]
    fn resolved_mail_addresses_must_stay_intranet() {
        assert!(validated_addresses("127.0.0.1", 993).is_ok());
        assert!(validated_addresses("8.8.8.8", 993).is_err());
        assert!("127.0.0.1".parse::<IpAddr>().is_ok());
    }

    #[test]
    fn parses_plain_mail_and_builds_bounded_outgoing_message() {
        let raw = b"From: Sender <sender@example.internal>\r\nTo: engineer@example.internal\r\nSubject: Synthetic result\r\nMessage-ID: <synthetic@example.internal>\r\nContent-Type: text/plain; charset=utf-8\r\n\r\nBias current: 10 uA";
        let parsed = parse_mail("mail-0123456789abcdef", "INBOX", 7, true, raw)
            .expect("synthetic mail should parse");
        assert_eq!(parsed.record.subject, "Synthetic result");
        assert!(parsed.record.body_text.contains("10 uA"));
        assert!(parsed.record.unread);

        let account = MailAccount {
            id: "mail-0123456789abcdef".into(),
            address: "engineer@example.internal".into(),
            display_name: "Engineer".into(),
            username: "engineer".into(),
            imap_host: "mail.internal".into(),
            imap_port: 993,
            smtp_host: "mail.internal".into(),
            smtp_port: 465,
            updated_at: 1,
        };
        let draft = MailDraft {
            to: vec!["reviewer@example.internal".into()],
            cc: Vec::new(),
            subject: "Review".into(),
            body: "Please review the synthetic result.".into(),
            attachment_paths: Vec::new(),
        };
        let message =
            build_outgoing_message(&account, &draft).expect("bounded outgoing mail should build");
        assert_eq!(message.envelope().to().len(), 1);
    }

    #[test]
    fn rejects_invalid_outgoing_mail_without_network_access() {
        assert!(parse_recipients(&[]).is_err());
        assert!(parse_recipients(&["not an address".into()]).is_err());
    }
}
