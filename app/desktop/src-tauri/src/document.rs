use crate::is_demo_mode;
use quick_xml::{events::Event, Reader, XmlVersion};
use serde::{Deserialize, Serialize};
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

#[derive(Clone, Debug, Deserialize, Serialize)]
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

pub(crate) fn source_file(path: &str) -> Result<(PathBuf, Vec<u8>), String> {
    crate::file_safety::validate_path(path)?;
    if path.is_empty() || path.len() > 4096 {
        return Err("文件路径无效。".to_string());
    }
    let requested = Path::new(path);
    let link_metadata = fs::symlink_metadata(requested)
        .map_err(|_| "无法读取所选文件，请确认文件仍然存在。".to_string())?;
    if crate::file_safety::is_link(&link_metadata) || !link_metadata.is_file() {
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
        "txt" => convert_txt(&bytes, &name)?,
        "md" | "markdown" => {
            if bytes.contains(&0) || bytes.len() > MAX_EXTRACTED_BYTES {
                return Err("Markdown 包含空字节或超过 4 MiB。".into());
            }
            let text = std::str::from_utf8(bytes).map_err(|_| "Markdown 仅支持 UTF-8 编码。")?;
            (
                text.trim_start_matches('\u{feff}').to_string(),
                None,
                Vec::new(),
            )
        }
        "docx" => convert_docx(&bytes, &name)?,
        "pdf" => convert_pdf(&bytes, &name)?,
        "pptx" => {
            let (text, pages) = crate::presentation::extract(bytes)?;
            (
                format!("{}{}", markdown_header(name, "PPTX", bytes.len() as u64), text),
                Some(pages),
                vec!["PPTX 保留页序、正文、列表、基本表格与演讲者备注；图片、图表、动画、合并单元格及复杂版式不还原，公式和阅读顺序请复核。".into()],
            )
        }
        "doc" | "ppt" => {
            return Err(
                "旧式 DOC/PPT 二进制格式尚不支持，请先在 Office 或 LibreOffice 另存为 DOCX/PPTX。"
                    .into(),
            )
        }
        _ => return Err("仅支持 TXT、Markdown、DOCX、PPTX 和含文本层的 PDF。".to_string()),
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

/// Uses the existing note exporters; conversion never writes into a selected input.
#[tauri::command]
pub fn export_converted_document(
    path: String,
    format: String,
    title: String,
    body: String,
    formatted: Option<String>,
    source_paths: Vec<String>,
) -> Result<(), String> {
    if !matches!(format.as_str(), "md" | "pdf" | "docx" | "pptx" | "txt") {
        return Err("不支持的文档导出格式。".into());
    }
    if body.is_empty() || body.len() > MAX_EXTRACTED_BYTES || title.chars().count() > 180 {
        return Err("导出正文不能为空且不超过 4 MiB，标题不超过 180 字。".into());
    }
    let destination = crate::notes::safe_destination(&path, &format)?;
    protect_sources(&destination, &source_paths)?;
    let bytes = match format.as_str() {
        "md" => body.into_bytes(),
        "pdf" => crate::pdf_export::build_pdf(&title, &format!("# {title}\n\n{body}"))?,
        "docx" => crate::notes::formatted_docx_bytes(&formatted.ok_or("缺少 DOCX 排版数据。")?)?,
        "pptx" => {
            let data = formatted.ok_or("缺少 PPTX 排版数据。")?;
            if data.len() > 16 * 1024 * 1024 {
                return Err("PPTX 导出超过 16 MiB。".into());
            }
            let bytes = STANDARD.decode(data).map_err(|_| "PPTX 编码无效。")?;
            crate::presentation::extract(&bytes)?;
            bytes
        }
        _ => {
            let text = formatted.ok_or("缺少纯文本内容。")?;
            if text.len() > MAX_EXTRACTED_BYTES {
                return Err("纯文本超过 4 MiB。".into());
            }
            text.into_bytes()
        }
    };
    // create_new also protects against hard-link aliases and destination races on Windows.
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&destination)
        .map_err(|_| "目标已存在或不可写，请另选新文件名；没有覆盖已有文件。")?;
    if file
        .write_all(&bytes)
        .and_then(|_| file.sync_all())
        .is_err()
    {
        drop(file);
        let _ = fs::remove_file(&destination);
        return Err("写入导出文件失败，请检查磁盘空间。".into());
    }
    Ok(())
}

fn protect_sources(destination: &Path, source_paths: &[String]) -> Result<(), String> {
    if source_paths.is_empty() || source_paths.len() > 20 {
        return Err("缺少有效的来源文件清单。".into());
    }
    for source in source_paths {
        crate::file_safety::validate_path(source)?;
        let source = Path::new(source);
        // Fail closed if the source disappeared after conversion.
        let canonical = source
            .canonicalize()
            .map_err(|_| "来源文件已移动，请重新选择后导出。")?;
        if destination.canonicalize().is_ok_and(|p| p == canonical) {
            return Err("请另存为新文件，不能覆盖本次选择的来源文件。".into());
        }
        #[cfg(unix)]
        if let (Ok(a), Ok(b)) = (source.metadata(), destination.metadata()) {
            use std::os::unix::fs::MetadataExt;
            if a.dev() == b.dev() && a.ino() == b.ino() {
                return Err("导出目标是来源文件的硬链接，请另存为新文件。".into());
            }
        }
    }
    Ok(())
}

#[tauri::command]
pub fn export_markdown(path: String, content: String) -> Result<(), String> {
    crate::file_safety::validate_path(&path)?;
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
    if fs::symlink_metadata(&canonical_destination).is_ok()
        && fs::symlink_metadata(&canonical_destination)
            .map(|metadata| crate::file_safety::is_link(&metadata) || !metadata.is_file())
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

    struct ConversionDir(PathBuf);
    impl ConversionDir {
        fn new() -> Self {
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir()
                .join(format!("icwb-conversion-{}-{stamp}", std::process::id()));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for ConversionDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn markdown_is_preserved_and_legacy_formats_are_explained() {
        let original = "# 工程记录\n\n| 参数 | 数值 |\n|---|---|\n| 偏置 | 10 µA |\n";
        for extension in ["md", "MD", "markdown"] {
            assert_eq!(
                convert_document_bytes("review.md", extension, original.as_bytes())
                    .unwrap()
                    .markdown,
                original
            );
        }
        for extension in ["doc", "ppt"] {
            assert!(convert_document_bytes("legacy", extension, b"legacy bytes")
                .unwrap_err()
                .contains("另存"));
        }
        assert!(convert_document_bytes("bad.md", "md", b"text\0text").is_err());
    }

    #[test]
    fn conversion_export_preserves_inputs_existing_outputs_and_rejects_bad_payloads() {
        let dir = ConversionDir::new();
        let source = dir.0.join("source.md");
        fs::write(&source, "original 10 µA").unwrap();
        let sources = vec![source.to_string_lossy().into_owned()];
        let run = |path: &Path, format: &str, data: Option<String>| {
            export_converted_document(
                path.to_string_lossy().into_owned(),
                format.into(),
                "测试".into(),
                "正文".into(),
                data,
                sources.clone(),
            )
        };
        assert!(run(&source, "md", None).is_err());
        let output = dir.0.join("review.md");
        fs::write(&output, "existing output").unwrap();
        assert!(run(&output, "md", None).is_err());
        assert_eq!(fs::read_to_string(output).unwrap(), "existing output");
        #[cfg(unix)]
        {
            let linked = dir.0.join("alias.md");
            fs::hard_link(&source, &linked).unwrap();
            assert!(run(&linked, "md", None).is_err());
        }
        assert!(run(&dir.0.join("wrong.txt"), "md", None).is_err());
        assert!(run(
            &dir.0.join("bad.pptx"),
            "pptx",
            Some(STANDARD.encode("not an archive"))
        )
        .is_err());
        assert!(!dir.0.join("bad.pptx").exists());
        assert_eq!(fs::read_to_string(&source).unwrap(), "original 10 µA");
        assert!(run(&dir.0.join("new.md"), "md", None).is_ok());
    }

    #[test]
    #[ignore = "requires frontend export fixtures; set ICWB_CONVERSION_FIXTURES"]
    fn generated_frontend_formats_pass_backend_export_and_read_back() {
        let root = PathBuf::from(
            std::env::var("ICWB_CONVERSION_FIXTURES").expect("fixture directory required"),
        );
        let source = root.join("review.md");
        let body = fs::read_to_string(&source).unwrap();
        let before = fs::read(&source).unwrap();
        let dir = ConversionDir::new();
        for format in ["md", "txt", "docx", "pptx", "pdf"] {
            let formatted = match format {
                "docx" | "pptx" => {
                    Some(STANDARD.encode(fs::read(root.join(format!("review.{format}"))).unwrap()))
                }
                "txt" => Some(fs::read_to_string(root.join("review.txt")).unwrap()),
                _ => None,
            };
            let target = dir.0.join(format!("result.{format}"));
            export_converted_document(
                target.to_string_lossy().into_owned(),
                format.into(),
                "模拟 IC 评审".into(),
                body.clone(),
                formatted,
                vec![source.to_string_lossy().into_owned()],
            )
            .unwrap();
            let converted = convert_document_path(&target).unwrap();
            assert!(
                converted.markdown.contains("10 µA"),
                "{format}: {}",
                converted.markdown
            );
            assert!(converted.markdown.contains("10 MHz"), "{format}");
            fs::copy(target, root.join(format!("backend-export.{format}"))).unwrap();
        }
        assert_eq!(before, fs::read(&source).unwrap());
    }

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
