use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{Deserialize, Serialize};

pub(crate) const MAX_ATTACHMENTS_PER_MESSAGE: usize = 4;
pub(crate) const MAX_ATTACHMENT_BYTES: usize = 5 * 1024 * 1024;
pub(crate) const MAX_ATTACHMENTS_TOTAL_BYTES: usize = 12 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChatAttachment {
    pub(crate) data_url: String,
    pub(crate) id: String,
    pub(crate) mime_type: String,
    pub(crate) name: String,
    pub(crate) size_bytes: usize,
}

fn has_expected_signature(mime_type: &str, bytes: &[u8]) -> bool {
    match mime_type {
        "image/png" => bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
        "image/jpeg" => bytes.starts_with(&[0xff, 0xd8, 0xff]),
        "image/webp" => bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP",
        _ => false,
    }
}

pub(crate) fn validate_attachment(attachment: &ChatAttachment) -> Result<(), String> {
    if attachment.id.is_empty()
        || attachment.id.len() > 100
        || !attachment
            .id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        return Err("附件标识无效。".to_string());
    }
    if attachment.name.trim().is_empty()
        || attachment.name.len() > 160
        || attachment
            .name
            .chars()
            .any(|character| character.is_control() || matches!(character, '/' | '\\'))
    {
        return Err("附件名称无效。".to_string());
    }
    let prefix = format!("data:{};base64,", attachment.mime_type);
    let encoded = attachment
        .data_url
        .strip_prefix(&prefix)
        .ok_or_else(|| "附件数据格式与类型不匹配。".to_string())?;
    if encoded.len() > MAX_ATTACHMENT_BYTES.saturating_mul(4) / 3 + 8 {
        return Err("单个附件不能超过 5 MiB。".to_string());
    }
    let decoded = STANDARD
        .decode(encoded)
        .map_err(|_| "附件不是有效的 Base64 数据。".to_string())?;
    if decoded.is_empty()
        || decoded.len() > MAX_ATTACHMENT_BYTES
        || decoded.len() != attachment.size_bytes
    {
        return Err("附件大小无效或与声明不一致。".to_string());
    }
    if !has_expected_signature(&attachment.mime_type, &decoded) {
        return Err("附件内容与 PNG、JPEG 或 WebP 类型不匹配。".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn synthetic_png() -> ChatAttachment {
        let bytes = b"\x89PNG\r\n\x1a\nsynthetic";
        ChatAttachment {
            data_url: format!("data:image/png;base64,{}", STANDARD.encode(bytes)),
            id: "attachment-test".into(),
            mime_type: "image/png".into(),
            name: "synthetic.png".into(),
            size_bytes: bytes.len(),
        }
    }

    #[test]
    fn validated_images_require_matching_mime_signature_and_size() {
        assert!(validate_attachment(&synthetic_png()).is_ok());
        let mut disguised = synthetic_png();
        disguised.mime_type = "image/jpeg".into();
        disguised.data_url = disguised.data_url.replacen("image/png", "image/jpeg", 1);
        assert!(validate_attachment(&disguised).is_err());
    }
}
