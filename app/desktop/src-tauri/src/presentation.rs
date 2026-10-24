//! Bounded, read-only OOXML extraction. No archive entries are unpacked to disk.
use quick_xml::{events::Event, Reader};
use std::{
    collections::HashMap,
    io::{Cursor, Read},
};
use zip::ZipArchive;

const MAX_XML: usize = 8 * 1024 * 1024;
const MAX_TOTAL_XML: usize = 32 * 1024 * 1024;
const MAX_TEXT: usize = 4 * 1024 * 1024;
const MAX_SLIDES: usize = 200;

#[derive(Default, Debug)]
struct Node {
    name: String,
    attrs: HashMap<String, String>,
    text: String,
    children: Vec<Node>,
}

fn node(event: &quick_xml::events::BytesStart<'_>) -> Result<Node, String> {
    let mut value = Node {
        name: String::from_utf8_lossy(event.local_name().as_ref()).into_owned(),
        ..Node::default()
    };
    for attribute in event.attributes() {
        let attribute = attribute.map_err(|_| "PPTX XML 属性无效。")?;
        let text = std::str::from_utf8(&attribute.value).map_err(|_| "PPTX 属性编码无效。")?;
        let text = quick_xml::escape::unescape(text).map_err(|_| "PPTX 属性实体无效。")?;
        value.attrs.insert(
            String::from_utf8_lossy(attribute.key.as_ref()).into_owned(),
            text.into_owned(),
        );
    }
    Ok(value)
}

fn xml(bytes: &[u8]) -> Result<Node, String> {
    let mut reader = Reader::from_reader(bytes);
    let mut stack = vec![Node::default()];
    let mut count = 0;
    loop {
        match reader.read_event().map_err(|_| "PPTX XML 结构损坏。")? {
            Event::Start(event) => {
                stack.push(node(&event)?);
                count += 1;
            }
            Event::Empty(event) => {
                stack.last_mut().unwrap().children.push(node(&event)?);
                count += 1;
            }
            Event::End(_) => {
                if stack.len() <= 1 {
                    return Err("PPTX XML 标签不匹配。".into());
                }
                let value = stack.pop().unwrap();
                stack.last_mut().unwrap().children.push(value);
            }
            Event::Text(event) => stack
                .last_mut()
                .unwrap()
                .text
                .push_str(&event.decode().map_err(|_| "PPTX 文本编码无效。")?),
            Event::CData(event) => stack
                .last_mut()
                .unwrap()
                .text
                .push_str(&event.decode().map_err(|_| "PPTX 文本编码无效。")?),
            Event::GeneralRef(event) => {
                let value = if let Some(c) = event
                    .resolve_char_ref()
                    .map_err(|_| "PPTX 字符引用无效。")?
                {
                    c.to_string()
                } else {
                    match event.decode().map_err(|_| "PPTX 实体编码无效。")?.as_ref() {
                        "amp" => "&",
                        "lt" => "<",
                        "gt" => ">",
                        "quot" => "\"",
                        "apos" => "'",
                        _ => return Err("PPTX 不允许外部或自定义实体。".into()),
                    }
                    .into()
                };
                stack.last_mut().unwrap().text.push_str(&value);
            }
            Event::DocType(_) => return Err("PPTX 不允许 DTD 或外部实体。".into()),
            Event::Eof => break,
            _ => {}
        }
        if stack.len() > 64 || count > 200_000 {
            return Err("PPTX XML 结构过于复杂。".into());
        }
    }
    if stack.len() != 1 || stack[0].children.len() != 1 {
        return Err("PPTX XML 不完整。".into());
    }
    Ok(stack.pop().unwrap().children.remove(0))
}

fn part(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    name: &str,
    total: &mut usize,
) -> Result<Node, String> {
    let file = archive
        .by_name(name)
        .map_err(|_| "PPTX 缺少必要的内部文档。")?;
    if file.size() == 0 || file.size() > MAX_XML as u64 {
        return Err("PPTX 单个 XML 为空或超过 8 MiB。".into());
    }
    let mut bytes = Vec::new();
    file.take(MAX_XML as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "无法读取 PPTX XML。")?;
    *total += bytes.len();
    if bytes.len() > MAX_XML || *total > MAX_TOTAL_XML {
        return Err("PPTX 解压文本超过安全上限。".into());
    }
    xml(&bytes)
}

fn child<'a>(value: &'a Node, name: &str) -> Option<&'a Node> {
    value.children.iter().find(|n| n.name == name)
}

fn relationship_target(rels: &Node, id: &str, kind: &str, parent: &str) -> Result<String, String> {
    if rels.name != "Relationships" {
        return Err("PPTX 关系清单无效。".into());
    }
    let matches: Vec<_> = rels
        .children
        .iter()
        .filter(|r| r.attrs.get("Id").is_some_and(|v| v == id))
        .collect();
    if matches.len() != 1 {
        return Err("PPTX 页面关系缺失或重复。".into());
    }
    let relation = matches[0];
    if relation
        .attrs
        .get("TargetMode")
        .is_some_and(|v| v != "Internal")
        || !relation
            .attrs
            .get("Type")
            .is_some_and(|v| v.ends_with(&format!("/{kind}")))
    {
        return Err("PPTX 不允许外部页面或不匹配的关系。".into());
    }
    let target = relation.attrs.get("Target").ok_or("PPTX 页面路径缺失。")?;
    if target.contains(['\\', ':', '%', '?', '#', '\0']) {
        return Err("PPTX 页面路径无效。".into());
    }
    let joined = if target.starts_with('/') {
        target.trim_start_matches('/').to_string()
    } else {
        format!("{parent}/{target}")
    };
    let mut parts = Vec::new();
    for segment in joined.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                if parts.pop().is_none() {
                    return Err("PPTX 页面路径越界。".into());
                }
            }
            _ => parts.push(segment),
        }
    }
    let path = parts.join("/");
    let prefix = if kind == "slide" {
        "ppt/slides/"
    } else {
        "ppt/notesSlides/"
    };
    if !path.starts_with(prefix) || !path.ends_with(".xml") {
        return Err("PPTX 页面路径超出允许范围。".into());
    }
    Ok(path)
}

fn run_text(value: &Node, text: &mut String) {
    if value.name == "t" {
        text.push_str(&value.text);
    } else if value.name == "br" {
        text.push('\n');
    } else {
        for n in &value.children {
            run_text(n, text);
        }
    }
}

fn escaped(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        if matches!(
            c,
            '\\' | '*' | '_' | '[' | ']' | '<' | '>' | '#' | '|' | '`'
        ) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

fn blocks(value: &Node, out: &mut String, notes: bool) -> Result<(), String> {
    // Notes masters include slide numbers and headers; only the notes body is useful.
    if notes && value.name == "sp" {
        fn is_body(n: &Node) -> bool {
            (n.name == "ph" && n.attrs.get("type").is_some_and(|v| v == "body"))
                || n.children.iter().any(is_body)
        }
        if !is_body(value) {
            return Ok(());
        }
    }
    if value.name == "tbl" {
        let mut rows = Vec::new();
        for row in value.children.iter().filter(|n| n.name == "tr") {
            let mut cells = Vec::new();
            for cell in row.children.iter().filter(|n| n.name == "tc") {
                let mut body = String::new();
                blocks(cell, &mut body, false)?;
                cells.push(body.trim().replace('\n', " "));
            }
            if !cells.is_empty() {
                rows.push(cells);
            }
        }
        let width = rows.iter().map(Vec::len).max().unwrap_or(0);
        for (i, row) in rows.iter_mut().enumerate() {
            row.resize(width, String::new());
            out.push_str(&format!("| {} |\n", row.join(" | ")));
            if i == 0 {
                out.push_str(&format!("| {} |\n", vec!["---"; width].join(" | ")));
            }
        }
        out.push('\n');
    } else if value.name == "p" {
        let mut text = String::new();
        run_text(value, &mut text);
        if !text.trim().is_empty() {
            let bullet = child(value, "pPr").is_some_and(|n| {
                n.children
                    .iter()
                    .any(|p| p.name == "buChar" || p.name == "buAutoNum")
            });
            if bullet {
                out.push_str("- ");
            }
            out.push_str(&escaped(text.trim()));
            out.push_str("\n\n");
        }
    } else {
        for n in &value.children {
            blocks(n, out, notes)?;
        }
    }
    if out.len() > MAX_TEXT {
        return Err("PPTX 提取内容超过 4 MiB。".into());
    }
    Ok(())
}

pub(crate) fn extract(bytes: &[u8]) -> Result<(String, usize), String> {
    let mut archive =
        ZipArchive::new(Cursor::new(bytes)).map_err(|_| "文件不是有效的 PPTX 压缩结构。")?;
    if archive.len() > 1000 {
        return Err("PPTX 内部文件超过 1000 项。".into());
    }
    let mut total = 0;
    let manifest = part(&mut archive, "[Content_Types].xml", &mut total)?;
    if manifest.name != "Types" {
        return Err("PPTX 内容类型清单无效。".into());
    }
    let presentation = part(&mut archive, "ppt/presentation.xml", &mut total)?;
    if presentation.name != "presentation" {
        return Err("PPTX 主文档类型无效。".into());
    }
    let rels = part(&mut archive, "ppt/_rels/presentation.xml.rels", &mut total)?;
    let slides = child(&presentation, "sldIdLst").ok_or("PPTX 没有幻灯片。")?;
    if slides.children.is_empty() || slides.children.len() > MAX_SLIDES {
        return Err("PPTX 页数为空或超过 200 页。".into());
    }
    let mut out = String::new();
    let mut has_text = false;
    for (index, slide) in slides.children.iter().enumerate() {
        let id = slide
            .attrs
            .iter()
            .find(|(key, _)| key.ends_with(":id"))
            .map(|(_, v)| v)
            .ok_or("PPTX 缺少页面关系编号。")?;
        let path = relationship_target(&rels, id, "slide", "ppt")?;
        let content = part(&mut archive, &path, &mut total)?;
        if content.name != "sld" {
            return Err("PPTX 幻灯片类型无效。".into());
        }
        let mut body = String::new();
        blocks(&content, &mut body, false)?;
        let slide_name = path.rsplit('/').next().unwrap();
        let rels_path = format!("ppt/slides/_rels/{slide_name}.rels");
        if archive.file_names().any(|n| n == rels_path) {
            let notes_rels = part(&mut archive, &rels_path, &mut total)?;
            if let Some(relation) = notes_rels.children.iter().find(|n| {
                n.attrs
                    .get("Type")
                    .is_some_and(|v| v.ends_with("/notesSlide"))
            }) {
                let notes_path = relationship_target(
                    &notes_rels,
                    relation.attrs.get("Id").ok_or("PPTX 备注关系编号缺失。")?,
                    "notesSlide",
                    "ppt/slides",
                )?;
                let notes = part(&mut archive, &notes_path, &mut total)?;
                let mut text = String::new();
                blocks(&notes, &mut text, true)?;
                if !text.trim().is_empty() {
                    body.push_str("### 演讲者备注\n\n");
                    body.push_str(&text);
                }
            }
        }
        out.push_str(&format!("## 第 {} 页\n\n", index + 1));
        if body.trim().is_empty() {
            out.push_str("_本页未提取到正文文本；请检查图片或图表。_\n\n");
        } else {
            has_text = true;
            out.push_str(&body);
        }
        if out.len() > MAX_TEXT {
            return Err("PPTX 提取内容超过 4 MiB。".into());
        }
    }
    if !has_text {
        return Err("PPTX 没有可提取文本；图片文字需要 OCR。".into());
    }
    Ok((out, slides.children.len()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::{write::SimpleFileOptions, ZipWriter};
    const FIXTURE: &[u8] =
        include_bytes!("../../../../demo/samples/synthetic_document_review.pptx");

    fn replace_part(name: &str, replacement: &[u8]) -> Vec<u8> {
        let mut old = ZipArchive::new(Cursor::new(FIXTURE)).unwrap();
        let mut out = ZipWriter::new(Cursor::new(Vec::new()));
        for i in 0..old.len() {
            let mut file = old.by_index(i).unwrap();
            let entry = file.name().to_string();
            let mut content = Vec::new();
            file.read_to_end(&mut content).unwrap();
            out.start_file(
                &entry,
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated),
            )
            .unwrap();
            out.write_all(if entry == name { replacement } else { &content })
                .unwrap();
        }
        out.finish().unwrap().into_inner()
    }

    #[test]
    fn independent_office_fixture_preserves_actual_order_tables_units_and_notes() {
        let (text, count) = extract(FIXTURE).unwrap();
        assert_eq!(count, 2);
        assert!(text.find("第一页").unwrap() < text.find("第二页").unwrap());
        assert!(text.contains("| 参数 | 数值 | 条件 |"));
        assert!(text.contains("| 偏置电流 | 10 µA | VDD=1.8 V |"));
        assert!(text.contains("±2 mV"));
        assert!(text.contains("确认单位 µA；不要将 typ 当作 max。"));
        assert!(!text.contains("Click to edit"));
    }

    #[test]
    fn rejects_external_relationships_traversal_and_incomplete_xml() {
        let mut zip = ZipArchive::new(Cursor::new(FIXTURE)).unwrap();
        let mut rels = String::new();
        zip.by_name("ppt/_rels/presentation.xml.rels")
            .unwrap()
            .read_to_string(&mut rels)
            .unwrap();
        for target in [
            "https://example.invalid/evil.xml",
            "../../../private.xml",
            "slides/%2e%2e/evil.xml",
        ] {
            let data = replace_part(
                "ppt/_rels/presentation.xml.rels",
                rels.replace("slides/slide2.xml", target).as_bytes(),
            );
            assert!(extract(&data).is_err(), "{target}");
        }
        let data = replace_part(
            "ppt/_rels/presentation.xml.rels",
            rels.replace(
                "Target=\"slides/slide2.xml\"",
                "TargetMode=\"External\" Target=\"slides/slide2.xml\"",
            )
            .as_bytes(),
        );
        assert!(extract(&data).is_err());
        assert!(xml(b"<root><unclosed>").is_err());
        assert!(xml(
            b"<!DOCTYPE root [<!ENTITY steal SYSTEM 'file:///etc/passwd'>]><root>&steal;</root>"
        )
        .is_err());
        assert!(extract(b"not a pptx").is_err());
    }

    #[test]
    fn rejects_oversized_xml_and_excessively_nested_xml() {
        let data = replace_part("ppt/slides/slide2.xml", &vec![b' '; MAX_XML + 1]);
        assert!(extract(&data).unwrap_err().contains("8 MiB"));
        assert!(xml(format!("{}{}", "<a>".repeat(70), "</a>".repeat(70)).as_bytes()).is_err());
    }
}
