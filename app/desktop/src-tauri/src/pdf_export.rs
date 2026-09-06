use base64::{engine::general_purpose::STANDARD, Engine as _};
use printpdf::{
    Color, Mm, Op, ParsedFont, PdfDocument, PdfFontHandle, PdfPage, PdfSaveOptions, Pt, RawImage,
    Rgb, TextItem, TextMatrix, XObjectTransform,
};
use std::{fs, fs::OpenOptions, io::Write, path::Path};

const FONT_BYTES: &[u8] = include_bytes!("../assets/fonts/NotoSansCJKsc-Regular.otf");
const MAX_PDF_SOURCE_BYTES: usize = 8 * 1024 * 1024;

#[derive(Clone)]
struct StyledLine {
    size: f32,
    text: String,
    muted: bool,
}

fn display_units(character: char) -> f32 {
    if character.is_ascii() {
        0.55
    } else {
        1.0
    }
}

fn wrap_text(text: &str, size: f32) -> Vec<String> {
    let maximum = 49.0 * (10.0 / size).clamp(0.55, 1.4);
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut units = 0.0;
    for character in text.chars() {
        let width = display_units(character);
        if units + width > maximum && !current.is_empty() {
            lines.push(current.trim_end().to_string());
            current.clear();
            units = 0.0;
        }
        current.push(character);
        units += width;
    }
    if !current.is_empty() {
        lines.push(current.trim_end().to_string());
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn markdown_lines(markdown: &str) -> Vec<StyledLine> {
    markdown
        .lines()
        .flat_map(|raw| {
            let trimmed = raw.trim();
            if trimmed.starts_with("![") && trimmed.contains("](data:image/") {
                return vec![StyledLine {
                    size: 0.0,
                    text: trimmed.to_string(),
                    muted: false,
                }];
            }
            let (text, size, muted) = if trimmed.is_empty() {
                (" ".to_string(), 7.0, true)
            } else if let Some(text) = trimmed.strip_prefix("# ") {
                (text.to_string(), 20.0, false)
            } else if let Some(text) = trimmed.strip_prefix("## ") {
                (text.to_string(), 15.0, false)
            } else if let Some(text) = trimmed.strip_prefix("### ") {
                (text.to_string(), 12.0, false)
            } else if let Some(text) = trimmed.strip_prefix("> ") {
                (format!("说明：{text}"), 9.0, true)
            } else if trimmed.starts_with('|') {
                if trimmed
                    .chars()
                    .all(|value| matches!(value, '|' | '-' | ':' | ' '))
                {
                    return Vec::new();
                }
                (trimmed.trim_matches('|').replace('|', " │ "), 8.2, false)
            } else {
                (trimmed.replace("**", "").replace('`', ""), 9.5, false)
            };
            wrap_text(&text, size)
                .into_iter()
                .map(move |text| StyledLine { size, text, muted })
                .collect::<Vec<_>>()
        })
        .collect()
}

fn start_page() -> Vec<Op> {
    vec![Op::SaveGraphicsState, Op::StartTextSection]
}

fn finish_page(mut operations: Vec<Op>) -> PdfPage {
    operations.push(Op::EndTextSection);
    operations.push(Op::RestoreGraphicsState);
    PdfPage::new(Mm(210.0), Mm(297.0), operations)
}

pub(crate) fn build_pdf(title: &str, markdown: &str) -> Result<Vec<u8>, String> {
    if markdown.trim().is_empty() || markdown.len() > MAX_PDF_SOURCE_BYTES {
        return Err("PDF 导出内容不能为空且不能超过 8 MiB。".to_string());
    }
    let mut font_warnings = Vec::new();
    let font = ParsedFont::from_bytes(FONT_BYTES, 0, &mut font_warnings)
        .ok_or_else(|| "内置中文字体无法解析。".to_string())?;
    let mut document = PdfDocument::new(&title.chars().take(160).collect::<String>());
    let font_id = document.add_font(&font);
    let font_handle = PdfFontHandle::External(font_id);
    let mut pages = Vec::new();
    let mut operations = start_page();
    let mut y = 274.0_f32;

    for line in markdown_lines(markdown) {
        if line.size == 0.0 {
            let url = line
                .text
                .split_once("](")
                .and_then(|(_, v)| v.strip_suffix(')'))
                .ok_or("图片 Markdown 格式无效。")?;
            let (mime, encoded) = url.split_once(",").ok_or("图片数据无效。")?;
            if !matches!(
                mime,
                "data:image/png;base64" | "data:image/jpeg;base64" | "data:image/webp;base64"
            ) || encoded.len() > 3 * 1024 * 1024
            {
                return Err("PDF 仅支持不超过 2 MiB 的 PNG/JPEG/WebP 内嵌图片。".to_string());
            }
            let bytes = STANDARD.decode(encoded).map_err(|_| "图片编码无效。")?;
            let image = RawImage::decode_from_bytes(&bytes, &mut Vec::new())
                .map_err(|_| "图片解码失败。")?;
            if image.width == 0
                || image.height == 0
                || image.width.saturating_mul(image.height) > 16_000_000
            {
                return Err("图片像素尺寸超限。".to_string());
            }
            let scale = (174.0 * 72.0 / 25.4 / image.width as f32)
                .min(190.0 * 72.0 / 25.4 / image.height as f32)
                .min(1.0);
            let height_mm = image.height as f32 * scale * 25.4 / 72.0;
            if y - height_mm < 20.0 {
                pages.push(finish_page(operations));
                operations = start_page();
                y = 274.0;
            }
            y -= height_mm;
            operations.push(Op::EndTextSection);
            operations.push(Op::UseXobject {
                id: document.add_image(&image),
                transform: XObjectTransform {
                    translate_x: Some(Mm(18.0).into_pt()),
                    translate_y: Some(Mm(y).into_pt()),
                    scale_x: Some(scale),
                    scale_y: Some(scale),
                    dpi: Some(72.0),
                    ..Default::default()
                },
            });
            operations.push(Op::StartTextSection);
            y -= 5.0;
            continue;
        }
        let line_height_mm = line.size * 0.53;
        if y - line_height_mm < 20.0 {
            pages.push(finish_page(operations));
            operations = start_page();
            y = 274.0;
        }
        operations.push(Op::SetTextMatrix {
            matrix: TextMatrix::Translate(Mm(18.0).into_pt(), Mm(y).into_pt()),
        });
        operations.push(Op::SetFont {
            font: font_handle.clone(),
            size: Pt(line.size),
        });
        operations.push(Op::SetFillColor {
            col: Color::Rgb(Rgb {
                r: if line.muted { 0.35 } else { 0.08 },
                g: if line.muted { 0.40 } else { 0.13 },
                b: if line.muted { 0.44 } else { 0.18 },
                icc_profile: None,
            }),
        });
        operations.push(Op::ShowText {
            items: vec![TextItem::Text(line.text)],
        });
        y -= line_height_mm;
    }
    pages.push(finish_page(operations));
    let bytes = document
        .with_pages(pages)
        .save(&PdfSaveOptions::default(), &mut Vec::new());
    if bytes.len() < 500 || !bytes.starts_with(b"%PDF-") {
        return Err("生成的 PDF 结构无效。".to_string());
    }
    Ok(bytes)
}

fn safe_destination(path: &str) -> Result<std::path::PathBuf, String> {
    let destination = Path::new(path);
    if destination.extension().and_then(|value| value.to_str()) != Some("pdf") {
        return Err("导出文件必须使用 .pdf 扩展名。".to_string());
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
            .map(|metadata| metadata.file_type().is_symlink() || !metadata.is_file())
            .unwrap_or(true)
    {
        return Err("导出目标不是普通文件或指向符号链接。".to_string());
    }
    Ok(canonical_destination)
}

#[tauri::command]
pub fn export_pdf(path: String, title: String, markdown: String) -> Result<(), String> {
    let destination = safe_destination(&path)?;
    let bytes = build_pdf(&title, &markdown)?;
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(destination)
        .map_err(|_| "无法创建 PDF 文件。".to_string())?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| "写入 PDF 文件失败。".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_searchable_multi_page_chinese_pdf() {
        let content = format!(
            "# 合成 Datasheet 概略版\n\n## 关键参数\n\n| 参数 | 数值 | 条件 | 来源 |\n|---|---:|---|---|\n{}",
            "偏置电流 10 uA，VDD = 1.8 V。[第 1 页]\n".repeat(140)
        );
        let bytes = build_pdf("合成 Datasheet", &content).expect("PDF should build");
        if let Ok(directory) = std::env::var("ICWB_TEST_OUTPUT_DIR") {
            std::fs::write(Path::new(&directory).join("datasheet-export.pdf"), &bytes).unwrap();
        }
        let parsed = lopdf::Document::load_mem(&bytes).expect("PDF structure should load");
        assert!(parsed.get_pages().len() >= 2);
        let text = pdf_extract::extract_text_from_mem(&bytes).expect("PDF text should extract");
        assert!(text.contains("合成 Datasheet"));
        assert!(text.contains("10 uA"));
    }

    #[test]
    fn embeds_local_note_images_without_printing_base64() {
        let image = STANDARD.encode(include_bytes!("../icons/128x128.png"));
        let bytes = build_pdf("图文笔记", &format!("# 图文笔记\n\n![本地图片](data:image/png;base64,{image})\n\n## 参数\n| 名称 | 值 |\n|---|---|\n| VDD | 1.8 V |")).unwrap();
        let parsed = lopdf::Document::load_mem(&bytes).unwrap();
        assert!(parsed
            .objects
            .values()
            .any(|v| v.as_stream().is_ok_and(|s| s
                .dict
                .get(b"Subtype")
                .is_ok_and(|v| v.as_name().is_ok_and(|n| n == b"Image")))));
        let text = pdf_extract::extract_text_from_mem(&bytes).unwrap();
        assert!(text.contains("1.8 V"));
        assert!(!text.contains("base64"));
        if let Ok(directory) = std::env::var("ICWB_TEST_OUTPUT_DIR") {
            std::fs::write(Path::new(&directory).join("note-export.pdf"), bytes).unwrap();
        }
    }
}
