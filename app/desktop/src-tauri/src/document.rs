use crate::is_demo_mode;
use quick_xml::{events::Event, Reader, XmlVersion};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Cursor, Read, Write},
    path::{Path, PathBuf},
};
use zip::ZipArchive;

const MAX_SOURCE_BYTES: u64 = 25 * 1024 * 1024;
const MAX_EXTRACTED_BYTES: usize = 4 * 1024 * 1024;
const MAX_DOCX_XML_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConvertedDocument {
    pub(crate) id: String,
    pub(crate) markdown: String,
    pub(crate) page_count: Option<usize>,
    pub(crate) size_bytes: u64,
    pub(crate) source_kind: String,
    pub(crate) source_name: String,
    pub(crate) warnings: Vec<String>,
}

fn source_file(path: &str) -> Result<(PathBuf, Vec<u8>), String> {
    if path.is_empty() || path.len() > 4096 {
        return Err("文件路径无效。".to_string());
    }
    let requested = Path::new(path);
    let link_metadata = fs::symlink_metadata(requested)
        .map_err(|_| "无法读取所选文件，请确认文件仍然存在。".to_string())?;
    if link_metadata.file_type().is_symlink() || !link_metadata.is_file() {
        return Err("只允许读取用户明确选择的普通文件，不接受符号链接。".to_string());
    }
    if link_metadata.len() == 0 || link_metadata.len() > MAX_SOURCE_BYTES {
        return Err("文件不能为空且不能超过 25 MiB。".to_string());
    }
    let canonical = requested
        .canonicalize()
        .map_err(|_| "无法解析所选文件路径。".to_string())?;
    let file = File::open(&canonical).map_err(|_| "无法打开所选文件。".to_string())?;
    let mut bytes = Vec::with_capacity(link_metadata.len() as usize);
    file.take(MAX_SOURCE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "读取所选文件失败。".to_string())?;
    if bytes.len() as u64 > MAX_SOURCE_BYTES {
        return Err("文件不能超过 25 MiB。".to_string());
    }
    Ok((canonical, bytes))
}

fn document_id(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest[..16]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn source_name(path: &Path) -> Result<String, String> {
    path.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .map(|name| name.chars().take(180).collect())
        .ok_or_else(|| "文件名不是受支持的文本编码。".to_string())
}

fn markdown_header(name: &str, kind: &str, size: u64) -> String {
    let safe_name = name.replace(['\r', '\n'], " ");
    format!(
        "# {safe_name}\n\n> 来源：用户显式选择的本地文件 | 格式：{kind} | 大小：{size} 字节\n\n"
    )
}

fn convert_txt(bytes: &[u8], name: &str) -> Result<(String, Option<usize>, Vec<String>), String> {
    if bytes.iter().take(8192).any(|byte| *byte == 0) {
        return Err("TXT 文件包含二进制空字节，已拒绝解析。".to_string());
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|_| "首版 TXT 转换仅支持 UTF-8 编码。".to_string())?
        .trim_start_matches('\u{feff}');
    let markdown = format!(
        "{}{}\n",
        markdown_header(name, "TXT", bytes.len() as u64),
        text
    );
    Ok((markdown, None, Vec::new()))
}

fn paragraph_prefix(style: &str) -> &'static str {
    let normalized = style.to_ascii_lowercase();
    if normalized.ends_with('1') {
        "## "
    } else if normalized.ends_with('2') {
        "### "
    } else if normalized.ends_with('3') {
        "#### "
    } else {
        ""
    }
}

fn docx_text(xml: &[u8]) -> Result<String, String> {
    let mut reader = Reader::from_reader(Cursor::new(xml));
    reader.config_mut().trim_text(false);
    let mut markdown = String::new();
    let mut paragraph = String::new();
    let mut paragraph_style = String::new();
    let mut in_paragraph = false;
    let mut buffer = Vec::new();

    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(event)) => match event.local_name().as_ref() {
                b"p" => {
                    in_paragraph = true;
                    paragraph.clear();
                    paragraph_style.clear();
                }
                b"pStyle" if in_paragraph => {
                    for attribute in event.attributes().with_checks(true).flatten() {
                        if attribute.key.local_name().as_ref() == b"val" {
                            paragraph_style = attribute
                                .normalized_value(XmlVersion::Implicit1_0)
                                .map_err(|_| "DOCX 段落样式无法解码。".to_string())?
                                .into_owned();
                        }
                    }
                }
                b"tab" if in_paragraph => paragraph.push('\t'),
                b"br" if in_paragraph => paragraph.push('\n'),
                _ => {}
            },
            Ok(Event::Empty(event)) => match event.local_name().as_ref() {
                b"pStyle" if in_paragraph => {
                    for attribute in event.attributes().with_checks(true).flatten() {
                        if attribute.key.local_name().as_ref() == b"val" {
                            paragraph_style = attribute
                                .normalized_value(XmlVersion::Implicit1_0)
                                .map_err(|_| "DOCX 段落样式无法解码。".to_string())?
                                .into_owned();
                        }
                    }
                }
                b"tab" if in_paragraph => paragraph.push('\t'),
                b"br" if in_paragraph => paragraph.push('\n'),
                _ => {}
            },
            Ok(Event::Text(text)) if in_paragraph => {
                let decoded = text
                    .decode()
                    .map_err(|_| "DOCX 文本无法解码。".to_string())?;
                let unescaped = quick_xml::escape::unescape(&decoded)
                    .map_err(|_| "DOCX 文本实体无效。".to_string())?;
                paragraph.push_str(&unescaped);
            }
            Ok(Event::GeneralRef(reference)) if in_paragraph => {
                if let Some(character) = reference
                    .resolve_char_ref()
                    .map_err(|_| "DOCX 字符引用无效。".to_string())?
                {
                    paragraph.push(character);
                } else {
                    let name = reference
                        .decode()
                        .map_err(|_| "DOCX 实体名称无法解码。".to_string())?;
                    paragraph.push_str(match name.as_ref() {
                        "amp" => "&",
                        "lt" => "<",
                        "gt" => ">",
                        "quot" => "\"",
                        "apos" => "'",
                        _ => return Err("DOCX 包含不受支持的外部实体引用。".to_string()),
                    });
                }
            }
            Ok(Event::End(event)) if event.local_name().as_ref() == b"p" => {
                let content = paragraph.trim();
                if !content.is_empty() {
                    markdown.push_str(paragraph_prefix(&paragraph_style));
                    markdown.push_str(content);
                    markdown.push_str("\n\n");
                }
                in_paragraph = false;
            }
            Ok(Event::Eof) => break,
            Err(_) => return Err("DOCX 主文档 XML 格式无效。".to_string()),
            _ => {}
        }
        if markdown.len().saturating_add(paragraph.len()) > MAX_EXTRACTED_BYTES {
            return Err("DOCX 提取内容超过 4 MiB 限制。".to_string());
        }
        buffer.clear();
    }
    if markdown.trim().is_empty() {
        return Err("DOCX 中没有可提取的正文文本。".to_string());
    }
    Ok(markdown)
}

fn convert_docx(bytes: &[u8], name: &str) -> Result<(String, Option<usize>, Vec<String>), String> {
    if !bytes.starts_with(b"PK") {
        return Err("文件扩展名是 DOCX，但内容不是有效的 Office 压缩包。".to_string());
    }
    let mut archive =
        ZipArchive::new(Cursor::new(bytes)).map_err(|_| "无法打开 DOCX 压缩结构。".to_string())?;
    if archive.len() > 1000 {
        return Err("DOCX 内部文件数量超过 1000 项限制。".to_string());
    }
    archive
        .by_name("[Content_Types].xml")
        .map_err(|_| "DOCX 缺少内容类型清单。".to_string())?;
    let document = archive
        .by_name("word/document.xml")
        .map_err(|_| "DOCX 缺少主文档内容。".to_string())?;
    if document.size() == 0 || document.size() > MAX_DOCX_XML_BYTES {
        return Err("DOCX 主文档内容为空或解压后超过 8 MiB。".to_string());
    }
    let mut xml = Vec::with_capacity(document.size() as usize);
    document
        .take(MAX_DOCX_XML_BYTES + 1)
        .read_to_end(&mut xml)
        .map_err(|_| "读取 DOCX 主文档失败。".to_string())?;
    let content = docx_text(&xml)?;
    let markdown = format!(
        "{}{}",
        markdown_header(name, "DOCX", bytes.len() as u64),
        content
    );
    Ok((
        markdown,
        None,
        vec!["DOCX 表格当前按段落顺序展开；复杂版式、公式和批注需要人工复核。".to_string()],
    ))
}

fn convert_pdf(bytes: &[u8], name: &str) -> Result<(String, Option<usize>, Vec<String>), String> {
    if !bytes.starts_with(b"%PDF-") {
        return Err("文件扩展名是 PDF，但文件签名无效。".to_string());
    }
    let document =
        lopdf::Document::load_mem(bytes).map_err(|_| "PDF 结构无效或不受支持。".to_string())?;
    let page_count = document.get_pages().len();
    if page_count == 0 || page_count > 2000 {
        return Err("PDF 页数为空或超过 2000 页限制。".to_string());
    }
    let pages = pdf_extract::extract_text_from_mem_by_pages(bytes)
        .map_err(|_| "PDF 文本提取失败；扫描件或受保护文件可能需要后续 OCR。".to_string())?;
    if pages.len() != page_count {
        return Err("PDF 页面结构与文本提取结果不一致。".to_string());
    }
    if pages.iter().all(|page| page.trim().is_empty()) {
        return Err("PDF 中没有可提取文本；首版不会自动对扫描件执行 OCR。".to_string());
    }
    if pages.iter().map(String::len).sum::<usize>() > MAX_EXTRACTED_BYTES {
        return Err("PDF 提取内容超过 4 MiB 限制。".to_string());
    }
    let mut markdown = markdown_header(name, "PDF", bytes.len() as u64);
    for (index, page) in pages.iter().enumerate() {
        markdown.push_str(&format!("## 第 {} 页\n\n", index + 1));
        if page.trim().is_empty() {
            markdown.push_str("_本页未提取到文本_\n\n");
        } else {
            markdown.push_str(page.trim());
            markdown.push_str("\n\n");
        }
    }
    Ok((
        markdown,
        Some(page_count),
        vec!["PDF 已按页保留来源位置；复杂阅读顺序、表格和公式仍需要人工复核。".to_string()],
    ))
}

#[tauri::command]
pub fn convert_document(path: String) -> Result<ConvertedDocument, String> {
    convert_document_path(Path::new(&path))
}

#[tauri::command]
pub fn load_pdf_data_url(path: String) -> Result<String, String> {
    let (canonical, bytes) = source_file(&path)?;
    if canonical
        .extension()
        .and_then(|v| v.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
        != Some("pdf")
        || !bytes.starts_with(b"%PDF-")
    {
        return Err("PDF 阅读器只接受签名匹配的 .pdf 普通文件。".to_string());
    }
    Ok(format!(
        "data:application/pdf;base64,{}",
        STANDARD.encode(bytes)
    ))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DemoDatasheet {
    document: ConvertedDocument,
    pdf_url: String,
}

#[tauri::command]
pub fn load_demo_datasheet() -> Result<DemoDatasheet, String> {
    if !is_demo_mode() {
        return Err("合成 Datasheet 只在独立 Demo 中提供。".to_string());
    }
    let bytes = include_bytes!("../../../../demo/samples/synthetic_analog_ic_datasheet.pdf");
    let document = convert_document_bytes("synthetic_analog_ic_datasheet.pdf", "pdf", bytes)?;
    Ok(DemoDatasheet {
        document,
        pdf_url: format!("data:application/pdf;base64,{}", STANDARD.encode(bytes)),
    })
}

pub(crate) fn convert_document_path(path: &Path) -> Result<ConvertedDocument, String> {
    let path = path
        .to_str()
        .ok_or_else(|| "文件路径不是受支持的文本编码。".to_string())?;
    let (canonical, bytes) = source_file(path)?;
    let name = source_name(&canonical)?;
    let extension = canonical
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .ok_or_else(|| "文件缺少受支持的扩展名。".to_string())?;
    convert_document_bytes(&name, &extension, &bytes)
}

pub(crate) fn convert_document_bytes(
    name: &str,
    extension: &str,
    bytes: &[u8],
) -> Result<ConvertedDocument, String> {
    if bytes.is_empty() || bytes.len() as u64 > MAX_SOURCE_BYTES {
        return Err("文件不能为空且不能超过 25 MiB。".to_string());
    }
    let extension = extension.to_ascii_lowercase();
    let (markdown, page_count, warnings) = match extension.as_str() {
        "txt" | "md" => convert_txt(&bytes, &name)?,
        "docx" => convert_docx(&bytes, &name)?,
        "pdf" => convert_pdf(&bytes, &name)?,
        _ => return Err("仅支持 TXT、Markdown、DOCX 和 PDF 文件。".to_string()),
    };
    Ok(ConvertedDocument {
        id: document_id(&bytes),
        markdown,
        page_count,
        size_bytes: bytes.len() as u64,
        source_kind: extension.to_ascii_uppercase(),
        source_name: name.to_string(),
        warnings,
    })
}

#[tauri::command]
pub fn export_markdown(path: String, content: String) -> Result<(), String> {
    if content.is_empty() || content.len() > MAX_EXTRACTED_BYTES {
        return Err("导出内容不能为空且不能超过 4 MiB。".to_string());
    }
    let destination = Path::new(&path);
    if destination.extension().and_then(|value| value.to_str()) != Some("md") {
        return Err("导出文件必须使用 .md 扩展名。".to_string());
    }
    let parent = destination
        .parent()
        .filter(|value| !value.as_os_str().is_empty())
        .ok_or_else(|| "导出目录无效。".to_string())?;
    let canonical_parent = parent
        .canonicalize()
        .map_err(|_| "导出目录不存在或不可访问。".to_string())?;
    let file_name = destination
        .file_name()
        .ok_or_else(|| "导出文件名无效。".to_string())?;
    let canonical_destination = canonical_parent.join(file_name);
    if canonical_destination.exists()
        && fs::symlink_metadata(&canonical_destination)
            .map(|metadata| metadata.file_type().is_symlink() || !metadata.is_file())
            .unwrap_or(true)
    {
        return Err("导出目标不是普通文件或指向符号链接。".to_string());
    }
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&canonical_destination)
        .map_err(|_| "无法创建 Markdown 文件。".to_string())?;
    file.write_all(content.as_bytes())
        .and_then(|_| file.sync_all())
        .map_err(|_| "写入 Markdown 文件失败。".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_utf8_text_without_exposing_the_source_path() {
        let (markdown, pages, warnings) = convert_txt("偏置电流：10 uA".as_bytes(), "sample.txt")
            .expect("text conversion should succeed");
        assert!(markdown.contains("# sample.txt"));
        assert!(markdown.contains("偏置电流：10 uA"));
        assert_eq!(pages, None);
        assert!(warnings.is_empty());
    }

    #[test]
    fn rejects_binary_text_and_spoofed_pdf() {
        assert!(convert_txt(b"text\0binary", "bad.txt").is_err());
        assert!(convert_pdf(b"not a pdf", "bad.pdf").is_err());
    }

    #[test]
    fn extracts_docx_paragraphs_and_headings() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
          <w:document xmlns:w="urn:test"><w:body>
            <w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>关键参数</w:t></w:r></w:p>
            <w:p><w:r><w:t>增益 &amp; 带宽</w:t></w:r></w:p>
          </w:body></w:document>"#;
        let markdown = docx_text(xml.as_bytes()).expect("docx xml should parse");
        assert!(markdown.contains("## 关键参数"));
        assert!(markdown.contains("增益 & 带宽"));
    }

    #[test]
    fn extracts_the_visual_checked_synthetic_pdf() {
        let bytes = include_bytes!("../../../../demo/samples/synthetic_analog_ic_datasheet.pdf");
        let (markdown, pages, warnings) = convert_pdf(bytes, "synthetic_analog_ic_datasheet.pdf")
            .expect("synthetic PDF should convert");
        assert_eq!(pages, Some(2));
        assert!(markdown.contains("SYNTH-AMP-01"));
        assert!(markdown.contains("Supply current"));
        assert!(markdown.contains("## 第 2 页"));
        assert_eq!(warnings.len(), 1);
    }
}
use base64::{engine::general_purpose::STANDARD, Engine as _};
