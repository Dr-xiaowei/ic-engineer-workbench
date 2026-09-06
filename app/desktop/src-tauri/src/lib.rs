mod attachments;
mod demo;
mod document;
mod mail;
mod notes;
mod pdf_export;
mod planner;
mod projects;
mod skills;
mod software;
mod storage;

use attachments::{
    validate_attachment, ChatAttachment, MAX_ATTACHMENTS_PER_MESSAGE, MAX_ATTACHMENTS_TOTAL_BYTES,
};
use keyring::{Entry, Error as KeyringError};
use reqwest::{redirect::Policy, Client};
use serde::{Deserialize, Serialize};
use std::{
    net::{IpAddr, Ipv6Addr, SocketAddr, ToSocketAddrs},
    time::Duration,
};
use tauri::Emitter;
use url::Url;

const CREDENTIAL_SERVICE: &str = "com.icengineer.workbench.model-api";
const DEMO_MODEL_BASE_URL: &str = "http://127.0.0.1:18080/v1";
const MAX_ENDPOINT_ID_LENGTH: usize = 80;
const MAX_MESSAGE_LENGTH: usize = 32_000;
const MAX_MESSAGES: usize = 80;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeStatus {
    app_version: &'static str,
    demo_mode: bool,
    network_mode: &'static str,
    data_location: &'static str,
}

pub(crate) fn is_demo_mode() -> bool {
    option_env!("ICWB_APP_MODE") == Some("demo")
}

fn is_embedded_demo_endpoint(base_url: &str) -> bool {
    is_demo_mode() && base_url.trim_end_matches('/') == DEMO_MODEL_BASE_URL
}

#[tauri::command]
fn get_runtime_status() -> RuntimeStatus {
    RuntimeStatus {
        app_version: env!("CARGO_PKG_VERSION"),
        demo_mode: is_demo_mode(),
        network_mode: "offline",
        data_location: "local",
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EndpointPolicyResult {
    allowed: bool,
    message: &'static str,
}

fn normalize_host(host: &str) -> String {
    host.to_ascii_lowercase()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .to_string()
}

fn ipv6_is_intranet(address: Ipv6Addr) -> bool {
    let first_segment = address.segments()[0];
    address.is_loopback() || first_segment & 0xfe00 == 0xfc00 || first_segment & 0xffc0 == 0xfe80
}

pub(crate) fn ip_is_intranet(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(address) => address.is_private() || address.is_loopback(),
        IpAddr::V6(address) => ipv6_is_intranet(address),
    }
}

pub(crate) fn host_is_intranet(host: &str) -> bool {
    let normalized = normalize_host(host);
    if normalized == "localhost"
        || normalized.ends_with(".local")
        || normalized.ends_with(".internal")
    {
        return true;
    }

    if !normalized.contains('.')
        && normalized
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
    {
        return true;
    }

    normalized.parse::<IpAddr>().is_ok_and(ip_is_intranet)
}

pub(crate) fn parse_intranet_url(base_url: &str) -> Result<Url, &'static str> {
    let url = Url::parse(base_url).map_err(|_| "请输入完整的 HTTP 或 HTTPS 地址。")?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err("端点只允许使用 HTTP 或 HTTPS。");
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err("端点地址中不能包含用户名或密码。");
    }
    if !url.host_str().is_some_and(host_is_intranet) {
        return Err("当前只允许本机、私有网段或内网主机名。");
    }
    Ok(url)
}

#[tauri::command]
fn validate_model_endpoint(base_url: String) -> EndpointPolicyResult {
    match parse_intranet_url(&base_url) {
        Ok(_) => EndpointPolicyResult {
            allowed: true,
            message: "地址与内网访问策略检查通过；尚未发起网络连接。",
        },
        Err(message) => EndpointPolicyResult {
            allowed: false,
            message,
        },
    }
}

fn validate_endpoint_id(endpoint_id: &str) -> Result<(), String> {
    let valid = !endpoint_id.is_empty()
        && endpoint_id.len() <= MAX_ENDPOINT_ID_LENGTH
        && endpoint_id.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
        });
    valid
        .then_some(())
        .ok_or_else(|| "端点标识无效。".to_string())
}

fn credential_entry(endpoint_id: &str) -> Result<Entry, String> {
    validate_endpoint_id(endpoint_id)?;
    Entry::new(CREDENTIAL_SERVICE, endpoint_id).map_err(|_| "无法访问系统凭据库。".to_string())
}

fn read_model_secret(endpoint_id: &str) -> Result<Option<String>, String> {
    match credential_entry(endpoint_id)?.get_password() {
        Ok(secret) => Ok(Some(secret)),
        Err(KeyringError::NoEntry) => Ok(None),
        Err(_) => Err("无法读取系统凭据库。".to_string()),
    }
}

#[tauri::command]
fn has_model_secret(endpoint_id: String) -> Result<bool, String> {
    Ok(read_model_secret(&endpoint_id)?.is_some())
}

#[tauri::command]
fn save_model_secret(
    app: tauri::AppHandle,
    endpoint_id: String,
    secret: String,
) -> Result<(), String> {
    let result = (|| {
        if secret.is_empty() || secret.len() > 16_384 {
            return Err("API 密钥不能为空且不能超过 16 KiB。".to_string());
        }
        credential_entry(&endpoint_id)?
            .set_password(&secret)
            .map_err(|_| "无法写入系统凭据库。".to_string())
    })();
    storage::record_audit(
        &app,
        "model.credential-save",
        if result.is_ok() { "success" } else { "failed" },
    );
    result
}

#[tauri::command]
fn delete_model_secret(app: tauri::AppHandle, endpoint_id: String) -> Result<(), String> {
    let result = match credential_entry(&endpoint_id)?.delete_credential() {
        Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
        Err(_) => Err("无法从系统凭据库删除凭据。".to_string()),
    };
    storage::record_audit(
        &app,
        "model.credential-delete",
        if result.is_ok() { "success" } else { "failed" },
    );
    result
}

fn resolve_and_pin(
    builder: reqwest::ClientBuilder,
    url: &Url,
) -> Result<reqwest::ClientBuilder, String> {
    let host = url
        .host_str()
        .map(normalize_host)
        .ok_or_else(|| "端点缺少主机名。".to_string())?;
    if host.parse::<IpAddr>().is_ok() {
        return Ok(builder);
    }

    let port = url
        .port_or_known_default()
        .ok_or_else(|| "端点端口无效。".to_string())?;
    let addresses: Vec<SocketAddr> = (host.as_str(), port)
        .to_socket_addrs()
        .map_err(|_| "无法解析内网端点地址。".to_string())?
        .collect();
    if addresses.is_empty()
        || addresses
            .iter()
            .any(|address| !ip_is_intranet(address.ip()))
    {
        return Err("端点解析结果包含非内网地址，已拒绝连接。".to_string());
    }

    Ok(builder.resolve(&host, addresses[0]))
}

pub(crate) fn model_http_client(url: &Url) -> Result<Client, String> {
    let builder = Client::builder()
        .connect_timeout(Duration::from_secs(4))
        .timeout(Duration::from_secs(30))
        .redirect(Policy::none())
        .user_agent("IC-Engineer-Workbench/0.0");
    resolve_and_pin(builder, url)?
        .build()
        .map_err(|_| "无法初始化模型连接。".to_string())
}

fn endpoint_resource_url(base_url: &str, resource: &str) -> Result<(Url, Client), String> {
    let base = parse_intranet_url(base_url).map_err(str::to_string)?;
    let resource_url = Url::parse(&format!(
        "{}/{}",
        base.as_str().trim_end_matches('/'),
        resource.trim_start_matches('/')
    ))
    .map_err(|_| "无法构造模型接口地址。".to_string())?;
    let client = model_http_client(&resource_url)?;
    Ok((resource_url, client))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ConnectionTestResult {
    success: bool,
    message: String,
    request_id: Option<String>,
}

#[tauri::command]
async fn test_model_connection(
    app: tauri::AppHandle,
    endpoint_id: String,
    base_url: String,
) -> Result<ConnectionTestResult, String> {
    if !storage::network_access_enabled(&app)? {
        storage::record_audit(&app, "model.connection-test", "blocked");
        return Err("网络访问总开关已关闭；请在设置中明确启用。".to_string());
    }
    validate_endpoint_id(&endpoint_id)?;
    let result = if is_embedded_demo_endpoint(&base_url) {
        Ok(ConnectionTestResult {
            success: true,
            message: "内置 Demo 模型已就绪（确定性模拟，不用于工程结论）。".to_string(),
            request_id: Some("embedded-demo".to_string()),
        })
    } else {
        let secret = read_model_secret(&endpoint_id)?;
        test_model_connection_inner(&base_url, secret.as_deref()).await
    };
    storage::record_audit(
        &app,
        "model.connection-test",
        if result.is_ok() { "success" } else { "failed" },
    );
    result
}

async fn test_model_connection_inner(
    base_url: &str,
    secret: Option<&str>,
) -> Result<ConnectionTestResult, String> {
    let (url, client) = endpoint_resource_url(&base_url, "models")?;
    let mut request = client.get(url);
    if let Some(secret) = secret {
        request = request.bearer_auth(secret);
    }

    let response = request
        .send()
        .await
        .map_err(|_| "无法连接模型服务，请检查地址、证书和服务状态。".to_string())?;
    let status = response.status();
    let request_id = response
        .headers()
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let success = status.is_success();
    Ok(ConnectionTestResult {
        success,
        message: if success {
            "模型服务连接成功。".to_string()
        } else if matches!(status.as_u16(), 401 | 403) {
            "服务可达，但凭据未通过验证。".to_string()
        } else {
            format!("服务返回 HTTP {}。", status.as_u16())
        },
        request_id,
    })
}

#[derive(Deserialize, Serialize)]
struct ChatRequestMessage {
    #[serde(default)]
    attachments: Vec<ChatAttachment>,
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ChatCompletionRequest<'a> {
    model: &'a str,
    messages: &'a [ChatApiMessage<'a>],
    stream: bool,
    store: bool,
}

#[derive(Serialize)]
struct ChatApiMessage<'a> {
    role: &'a str,
    content: ChatApiContent<'a>,
}

#[derive(Serialize)]
#[serde(untagged)]
enum ChatApiContent<'a> {
    Text(&'a str),
    Parts(Vec<ChatContentPart<'a>>),
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ChatContentPart<'a> {
    Text { text: &'a str },
    ImageUrl { image_url: ImageUrl<'a> },
}

#[derive(Serialize)]
struct ImageUrl<'a> {
    url: &'a str,
}

fn build_api_messages(messages: &[ChatRequestMessage]) -> Vec<ChatApiMessage<'_>> {
    messages
        .iter()
        .map(|message| {
            let content = if message.attachments.is_empty() {
                ChatApiContent::Text(&message.content)
            } else {
                let mut parts = vec![ChatContentPart::Text {
                    text: &message.content,
                }];
                parts.extend(message.attachments.iter().map(|attachment| {
                    ChatContentPart::ImageUrl {
                        image_url: ImageUrl {
                            url: &attachment.data_url,
                        },
                    }
                }));
                ChatApiContent::Parts(parts)
            };
            ChatApiMessage {
                role: &message.role,
                content,
            }
        })
        .collect()
}

#[derive(Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatCompletionChoice>,
}

#[derive(Deserialize)]
struct ChatCompletionChoice {
    message: ChatCompletionMessage,
}

#[derive(Deserialize)]
struct ChatCompletionMessage {
    content: String,
}

#[derive(Deserialize)]
struct ChatCompletionStreamResponse {
    choices: Vec<ChatCompletionStreamChoice>,
}

#[derive(Deserialize)]
struct ChatCompletionStreamChoice {
    delta: ChatCompletionDelta,
}

#[derive(Deserialize)]
struct ChatCompletionDelta {
    content: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ChatCompletionResult {
    content: String,
    request_id: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ChatStreamDeltaEvent {
    stream_id: String,
    delta: String,
}

enum SseEvent {
    Data(String),
    Done,
}

#[derive(Default)]
struct SseDecoder {
    buffer: Vec<u8>,
    data_lines: Vec<String>,
}

impl SseDecoder {
    fn push(&mut self, chunk: &[u8]) -> Result<Vec<SseEvent>, String> {
        self.buffer.extend_from_slice(chunk);
        let mut events = Vec::new();
        while let Some(line_end) = self.buffer.iter().position(|byte| *byte == b'\n') {
            let mut line = self.buffer.drain(..=line_end).collect::<Vec<_>>();
            line.pop();
            if line.last() == Some(&b'\r') {
                line.pop();
            }
            self.consume_line(&line, &mut events)?;
        }
        Ok(events)
    }

    fn finish(&mut self) -> Result<Vec<SseEvent>, String> {
        let mut events = Vec::new();
        if !self.buffer.is_empty() {
            let line = std::mem::take(&mut self.buffer);
            self.consume_line(&line, &mut events)?;
        }
        self.flush(&mut events);
        Ok(events)
    }

    fn consume_line(&mut self, line: &[u8], events: &mut Vec<SseEvent>) -> Result<(), String> {
        if line.is_empty() {
            self.flush(events);
            return Ok(());
        }
        if line.starts_with(b":") {
            return Ok(());
        }
        let text =
            std::str::from_utf8(line).map_err(|_| "模型流包含无效的 UTF-8 数据。".to_string())?;
        if let Some(data) = text.strip_prefix("data:") {
            self.data_lines.push(data.trim_start().to_string());
        }
        Ok(())
    }

    fn flush(&mut self, events: &mut Vec<SseEvent>) {
        if self.data_lines.is_empty() {
            return;
        }
        let data = self.data_lines.join("\n");
        self.data_lines.clear();
        if data.trim() == "[DONE]" {
            events.push(SseEvent::Done);
        } else {
            events.push(SseEvent::Data(data));
        }
    }
}

fn validate_chat_request(model: &str, messages: &[ChatRequestMessage]) -> Result<(), String> {
    if model.trim().is_empty() || model.len() > 100 {
        return Err("模型标识无效。".to_string());
    }
    if messages.is_empty() || messages.len() > MAX_MESSAGES {
        return Err("会话消息数量无效。".to_string());
    }
    if messages.iter().any(|message| {
        !matches!(
            message.role.as_str(),
            "user" | "assistant" | "system" | "developer"
        ) || (message.content.is_empty() && message.attachments.is_empty())
            || message.content.len() > MAX_MESSAGE_LENGTH
            || message.attachments.len() > MAX_ATTACHMENTS_PER_MESSAGE
            || message.attachments.iter().any(|attachment| {
                message.role != "user" || validate_attachment(attachment).is_err()
            })
    }) {
        return Err("会话消息格式或长度无效。".to_string());
    }
    let attachment_bytes = messages
        .iter()
        .flat_map(|message| &message.attachments)
        .map(|attachment| attachment.size_bytes)
        .sum::<usize>();
    if attachment_bytes > MAX_ATTACHMENTS_TOTAL_BYTES {
        return Err("本次请求的附件总大小不能超过 12 MiB。".to_string());
    }
    Ok(())
}

#[tauri::command]
async fn send_chat_completion(
    app: tauri::AppHandle,
    endpoint_id: String,
    base_url: String,
    model: String,
    messages: Vec<ChatRequestMessage>,
) -> Result<ChatCompletionResult, String> {
    if !storage::network_access_enabled(&app)? {
        storage::record_audit(&app, "model.chat-request", "blocked");
        return Err("网络访问总开关已关闭；未发送模型请求。".to_string());
    }
    validate_endpoint_id(&endpoint_id)?;
    let result = if is_embedded_demo_endpoint(&base_url) {
        embedded_demo_completion(&model, &messages)
    } else {
        let secret = read_model_secret(&endpoint_id)?;
        send_chat_completion_inner(&base_url, &model, &messages, secret.as_deref()).await
    };
    storage::record_audit(
        &app,
        "model.chat-request",
        if result.is_ok() { "success" } else { "failed" },
    );
    result
}

fn embedded_demo_completion(
    model: &str,
    messages: &[ChatRequestMessage],
) -> Result<ChatCompletionResult, String> {
    validate_chat_request(model, messages)?;
    let subject = messages
        .iter()
        .rev()
        .find(|message| message.role == "user")
        .map(|message| message.content.trim())
        .filter(|content| !content.is_empty())
        .unwrap_or("当前工程问题");
    let subject = subject.chars().take(180).collect::<String>();
    Ok(ChatCompletionResult {
        content: format!(
            "**Demo 模拟回答**\n\n已收到：{subject}\n\n- 这是离线确定性示例，不代表真实模型或 EDA 结论。\n- 实际工程判断请核对原始资料中的页码、单位与测试条件。"
        ),
        request_id: Some("embedded-demo".to_string()),
    })
}

async fn send_chat_completion_inner(
    base_url: &str,
    model: &str,
    messages: &[ChatRequestMessage],
    secret: Option<&str>,
) -> Result<ChatCompletionResult, String> {
    validate_chat_request(&model, &messages)?;
    let api_messages = build_api_messages(messages);
    let (url, client) = endpoint_resource_url(&base_url, "chat/completions")?;
    let mut request = client.post(url).json(&ChatCompletionRequest {
        model: &model,
        messages: &api_messages,
        stream: false,
        store: false,
    });
    if let Some(secret) = secret {
        request = request.bearer_auth(secret);
    }

    let mut response = request
        .send()
        .await
        .map_err(|_| "模型请求失败，请检查连接与服务状态。".to_string())?;
    let status = response.status();
    let request_id = response
        .headers()
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    if !status.is_success() {
        return Err(format!("模型服务返回 HTTP {}。", status.as_u16()));
    }
    const MAX_RESPONSE_BYTES: usize = 4 * 1024 * 1024;
    if response
        .content_length()
        .is_some_and(|length| length > MAX_RESPONSE_BYTES as u64)
    {
        return Err("模型响应超过 4 MiB 限制。".to_string());
    }
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "读取模型响应失败。".to_string())?
    {
        if body.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
            return Err("模型响应超过 4 MiB 限制。".to_string());
        }
        body.extend_from_slice(&chunk);
    }
    let payload = serde_json::from_slice::<ChatCompletionResponse>(&body)
        .map_err(|_| "模型响应格式不是受支持的 Chat Completions JSON。".to_string())?;
    let content = payload
        .choices
        .into_iter()
        .next()
        .map(|choice| choice.message.content)
        .filter(|content| !content.is_empty())
        .ok_or_else(|| "模型响应中没有文本内容。".to_string())?;

    Ok(ChatCompletionResult {
        content,
        request_id,
    })
}

#[tauri::command]
async fn send_chat_completion_stream(
    app: tauri::AppHandle,
    stream_id: String,
    endpoint_id: String,
    base_url: String,
    model: String,
    messages: Vec<ChatRequestMessage>,
) -> Result<ChatCompletionResult, String> {
    if !storage::network_access_enabled(&app)? {
        storage::record_audit(&app, "model.stream-request", "blocked");
        return Err("网络访问总开关已关闭；未发送模型请求。".to_string());
    }
    validate_endpoint_id(&endpoint_id)?;
    validate_endpoint_id(&stream_id)?;
    let emit_delta = |delta: &str| {
        app.emit(
            "model-stream-delta",
            ChatStreamDeltaEvent {
                stream_id: stream_id.clone(),
                delta: delta.to_string(),
            },
        )
        .map_err(|_| "无法向对话界面发送流式事件。".to_string())
    };
    let result = if is_embedded_demo_endpoint(&base_url) {
        embedded_demo_completion(&model, &messages).and_then(|result| {
            emit_delta(&result.content)?;
            Ok(result)
        })
    } else {
        let secret = read_model_secret(&endpoint_id)?;
        send_chat_completion_stream_inner(
            &base_url,
            &model,
            &messages,
            secret.as_deref(),
            emit_delta,
        )
        .await
    };
    storage::record_audit(
        &app,
        "model.stream-request",
        if result.is_ok() { "success" } else { "failed" },
    );
    result
}

async fn send_chat_completion_stream_inner<F>(
    base_url: &str,
    model: &str,
    messages: &[ChatRequestMessage],
    secret: Option<&str>,
    mut on_delta: F,
) -> Result<ChatCompletionResult, String>
where
    F: FnMut(&str) -> Result<(), String>,
{
    validate_chat_request(model, messages)?;
    let api_messages = build_api_messages(messages);
    let (url, client) = endpoint_resource_url(base_url, "chat/completions")?;
    let mut request = client.post(url).json(&ChatCompletionRequest {
        model,
        messages: &api_messages,
        stream: true,
        store: false,
    });
    if let Some(secret) = secret {
        request = request.bearer_auth(secret);
    }

    let mut response = request
        .send()
        .await
        .map_err(|_| "模型请求失败，请检查连接与服务状态。".to_string())?;
    let status = response.status();
    let request_id = response
        .headers()
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    if !status.is_success() {
        return Err(format!("模型服务返回 HTTP {}。", status.as_u16()));
    }

    const MAX_STREAM_BYTES: usize = 4 * 1024 * 1024;
    let mut decoder = SseDecoder::default();
    let mut received_bytes = 0_usize;
    let mut content = String::new();
    let mut done = false;
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "读取模型流失败。".to_string())?
    {
        received_bytes = received_bytes.saturating_add(chunk.len());
        if received_bytes > MAX_STREAM_BYTES {
            return Err("模型流超过 4 MiB 限制。".to_string());
        }
        for event in decoder.push(&chunk)? {
            match event {
                SseEvent::Done => done = true,
                SseEvent::Data(data) => {
                    let payload = serde_json::from_str::<ChatCompletionStreamResponse>(&data)
                        .map_err(|_| {
                            "模型流事件格式不是受支持的 Chat Completions JSON。".to_string()
                        })?;
                    if let Some(delta) = payload
                        .choices
                        .into_iter()
                        .next()
                        .and_then(|choice| choice.delta.content)
                        .filter(|delta| !delta.is_empty())
                    {
                        content.push_str(&delta);
                        on_delta(&delta)?;
                    }
                }
            }
        }
        if done {
            break;
        }
    }
    if !done {
        for event in decoder.finish()? {
            if let SseEvent::Data(data) = event {
                let payload =
                    serde_json::from_str::<ChatCompletionStreamResponse>(&data).map_err(|_| {
                        "模型流事件格式不是受支持的 Chat Completions JSON。".to_string()
                    })?;
                if let Some(delta) = payload
                    .choices
                    .into_iter()
                    .next()
                    .and_then(|choice| choice.delta.content)
                    .filter(|delta| !delta.is_empty())
                {
                    content.push_str(&delta);
                    on_delta(&delta)?;
                }
            }
        }
    }
    if content.is_empty() {
        return Err("模型流中没有文本内容。".to_string());
    }
    Ok(ChatCompletionResult {
        content,
        request_id,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            get_runtime_status,
            demo::seed_demo_workspace,
            validate_model_endpoint,
            has_model_secret,
            save_model_secret,
            delete_model_secret,
            test_model_connection,
            send_chat_completion,
            send_chat_completion_stream,
            document::convert_document,
            document::load_pdf_data_url,
            document::load_demo_datasheet,
            document::export_markdown,
            pdf_export::export_pdf,
            projects::add_project,
            projects::list_projects,
            projects::remove_project,
            projects::index_project,
            projects::get_project,
            projects::get_indexed_document,
            projects::search_project,
            projects::project_semantic_candidates,
            planner::list_project_progress,
            planner::save_project_progress,
            planner::list_project_progress_history,
            planner::list_calendar_tasks,
            planner::save_calendar_task,
            planner::delete_calendar_task,
            software::add_software_launcher,
            software::list_software_launchers,
            software::remove_software_launcher,
            software::launch_software,
            skills::import_skill,
            skills::import_skill_url,
            skills::list_skills,
            skills::set_skill_enabled,
            skills::remove_skill,
            mail::save_mail_account,
            mail::seed_demo_mail,
            mail::list_mail_accounts,
            mail::save_mail_secrets,
            mail::has_mail_secrets,
            mail::delete_mail_account,
            mail::test_mail_account,
            mail::list_mail_folders,
            mail::sync_mail_folder,
            mail::list_local_mail,
            mail::search_local_mail,
            mail::export_mail_attachment,
            mail::preview_mail_attachment,
            mail::send_mail,
            notes::get_notes_root,
            notes::set_notes_root,
            notes::list_notes,
            notes::save_note,
            notes::delete_note,
            notes::export_note,
            storage::get_network_access,
            storage::set_network_access,
            storage::backup_local_data,
            storage::restore_local_data,
            storage::get_dashboard_summary,
            storage::list_audit_events,
            storage::load_conversations,
            storage::save_conversation
        ])
        .run(tauri::generate_context!())
        .expect("failed to run IC engineer workbench");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::TcpListener,
        sync::mpsc::{self, Receiver},
        thread,
    };

    fn spawn_local_response(body: &'static str) -> (String, Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("local test listener should bind");
        let address = listener
            .local_addr()
            .expect("listener should have an address");
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("test request should connect");
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .expect("read timeout should apply");
            let mut request = Vec::new();
            let mut chunk = [0_u8; 4096];
            loop {
                let count = stream
                    .read(&mut chunk)
                    .expect("test request should be readable");
                if count == 0 {
                    break;
                }
                request.extend_from_slice(&chunk[..count]);
                let request_text = String::from_utf8_lossy(&request);
                if let Some(header_end) = request_text.find("\r\n\r\n") {
                    let content_length = request_text[..header_end]
                        .lines()
                        .find_map(|line| {
                            let (name, value) = line.split_once(':')?;
                            name.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse::<usize>().ok())
                                .flatten()
                        })
                        .unwrap_or(0);
                    if request.len() >= header_end + 4 + content_length {
                        break;
                    }
                }
            }
            sender
                .send(String::from_utf8_lossy(&request).into_owned())
                .expect("request capture should succeed");
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nX-Request-Id: req-local-test\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream
                .write_all(response.as_bytes())
                .expect("test response should be writable");
        });
        (format!("http://{address}/v1"), receiver)
    }

    fn synthetic_attachment() -> ChatAttachment {
        ChatAttachment {
            data_url: "data:image/png;base64,iVBORw0KGgpzeW50aGV0aWM=".into(),
            id: "attachment-test".into(),
            mime_type: "image/png".into(),
            name: "synthetic.png".into(),
            size_bytes: 17,
        }
    }

    #[test]
    fn runtime_starts_offline_and_local() {
        let status = get_runtime_status();
        assert_eq!(status.network_mode, "offline");
        assert_eq!(status.data_location, "local");
    }

    #[test]
    fn endpoint_policy_allows_only_local_or_intranet_hosts() {
        assert!(validate_model_endpoint("http://127.0.0.1:11434/v1".into()).allowed);
        assert!(validate_model_endpoint("https://10.8.0.2/v1".into()).allowed);
        assert!(validate_model_endpoint("http://[fd00::8]:8000/v1".into()).allowed);
        assert!(validate_model_endpoint("https://models.internal/v1".into()).allowed);
        assert!(!validate_model_endpoint("https://api.example.com/v1".into()).allowed);
        assert!(!validate_model_endpoint("http://user:secret@localhost/v1".into()).allowed);
    }

    #[test]
    fn endpoint_identifier_and_chat_bounds_are_enforced() {
        assert!(validate_endpoint_id("lab-model_01").is_ok());
        assert!(validate_endpoint_id("../model secret").is_err());
        assert!(validate_chat_request(
            "local-model",
            &[ChatRequestMessage {
                attachments: Vec::new(),
                role: "user".into(),
                content: "检查偏置约束".into(),
            }]
        )
        .is_ok());
        assert!(validate_chat_request(
            "local-model",
            &[ChatRequestMessage {
                attachments: Vec::new(),
                role: "tool".into(),
                content: "unexpected".into(),
            }]
        )
        .is_err());
    }

    #[test]
    fn embedded_demo_completion_is_bounded_and_clearly_labeled() {
        let result = embedded_demo_completion(
            "demo-analog-assistant",
            &[ChatRequestMessage {
                attachments: Vec::new(),
                role: "user".into(),
                content: "请概括偏置电流条件".into(),
            }],
        )
        .expect("synthetic demo messages should be supported");
        assert!(result.content.contains("Demo 模拟回答"));
        assert!(result.content.contains("不代表真实模型或 EDA 结论"));
        assert_eq!(result.request_id.as_deref(), Some("embedded-demo"));
    }

    #[test]
    fn chat_completion_request_disables_remote_storage() {
        let messages = vec![ChatRequestMessage {
            attachments: Vec::new(),
            role: "user".into(),
            content: "hello".into(),
        }];
        let api_messages = build_api_messages(&messages);
        let payload = serde_json::to_value(ChatCompletionRequest {
            model: "local-model",
            messages: &api_messages,
            stream: false,
            store: false,
        })
        .expect("request should serialize");
        assert_eq!(payload["stream"], false);
        assert_eq!(payload["store"], false);

        let stream_payload = serde_json::to_value(ChatCompletionRequest {
            model: "local-model",
            messages: &api_messages,
            stream: true,
            store: false,
        })
        .expect("stream request should serialize");
        assert_eq!(stream_payload["stream"], true);
        assert_eq!(stream_payload["store"], false);

        let multimodal = vec![ChatRequestMessage {
            attachments: vec![synthetic_attachment()],
            role: "user".into(),
            content: "分析图片".into(),
        }];
        assert!(validate_chat_request("local-model", &multimodal).is_ok());
        let api_messages = build_api_messages(&multimodal);
        let payload = serde_json::to_value(ChatCompletionRequest {
            model: "local-model",
            messages: &api_messages,
            stream: true,
            store: false,
        })
        .expect("multimodal request should serialize");
        assert_eq!(payload["messages"][0]["content"][0]["type"], "text");
        assert_eq!(payload["messages"][0]["content"][1]["type"], "image_url");
        assert_eq!(
            payload["messages"][0]["content"][1]["image_url"]["url"],
            synthetic_attachment().data_url
        );
    }

    #[test]
    fn sse_decoder_handles_fragmented_lines_and_done_event() {
        let mut decoder = SseDecoder::default();
        assert!(decoder
            .push(b"data: {\"choices\":[{\"delta\":{\"content\":\"\xe6\x9c")
            .unwrap()
            .is_empty());
        let events = decoder
            .push(b"\xac\xe5\x9c\xb0\"}}]}\r\n\r\ndata: [DONE]\n\n")
            .expect("fragmented UTF-8 should decode after a full line");
        assert_eq!(events.len(), 2);
        match &events[0] {
            SseEvent::Data(data) => assert!(data.contains("本地")),
            SseEvent::Done => panic!("first event should contain data"),
        }
        assert!(matches!(events[1], SseEvent::Done));
    }

    #[test]
    fn local_connection_and_chat_adapter_use_expected_protocol() {
        let (models_url, models_request) = spawn_local_response(r#"{"data":[]}"#);
        let connection = tauri::async_runtime::block_on(test_model_connection_inner(
            &models_url,
            Some("synthetic-token"),
        ))
        .expect("local connection test should succeed");
        assert!(connection.success);
        let captured_models_request = models_request
            .recv()
            .expect("models request should be captured");
        assert!(captured_models_request.starts_with("GET /v1/models HTTP/1.1"));
        assert!(captured_models_request.contains("authorization: Bearer synthetic-token"));

        let response_body = r#"{"choices":[{"message":{"content":"本地模拟回答"}}]}"#;
        let (chat_url, chat_request) = spawn_local_response(response_body);
        let messages = vec![ChatRequestMessage {
            attachments: Vec::new(),
            role: "user".into(),
            content: "测试问题".into(),
        }];
        let result = tauri::async_runtime::block_on(send_chat_completion_inner(
            &chat_url,
            "local-model",
            &messages,
            None,
        ))
        .expect("local chat request should succeed");
        assert_eq!(result.content, "本地模拟回答");
        assert_eq!(result.request_id.as_deref(), Some("req-local-test"));
        let captured_chat_request = chat_request
            .recv()
            .expect("chat request should be captured");
        assert!(captured_chat_request.starts_with("POST /v1/chat/completions HTTP/1.1"));
        assert!(captured_chat_request.contains("\"stream\":false"));
        assert!(captured_chat_request.contains("\"store\":false"));

        let stream_body = concat!(
            "data: {\"choices\":[{\"delta\":{\"content\":\"流式\"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"回答\"}}]}\n\n",
            "data: [DONE]\n\n"
        );
        let (stream_url, stream_request) = spawn_local_response(stream_body);
        let mut deltas = Vec::new();
        let stream_result = tauri::async_runtime::block_on(send_chat_completion_stream_inner(
            &stream_url,
            "local-model",
            &messages,
            None,
            |delta| {
                deltas.push(delta.to_string());
                Ok(())
            },
        ))
        .expect("local stream request should succeed");
        assert_eq!(deltas, vec!["流式", "回答"]);
        assert_eq!(stream_result.content, "流式回答");
        let captured_stream_request = stream_request
            .recv()
            .expect("stream request should be captured");
        assert!(captured_stream_request.contains("\"stream\":true"));
        assert!(captured_stream_request.contains("\"store\":false"));
    }
}
