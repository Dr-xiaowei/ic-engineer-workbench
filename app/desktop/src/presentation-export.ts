import PptxGenJS from "pptxgenjs";
import { noteHtml } from "./note-export";
import { textContent } from "./document-export";

const MAX_SLIDES = 200;
const FONT = "Noto Sans CJK SC";
const BODY_TOP = 1.28;
const BODY_BOTTOM = 6.75;

// Conservative CJK-aware wrapping avoids shrinking dense engineering text to
// unreadable sizes. Long paragraphs and table cells continue on later slides.
export function wrapPresentationText(text: string, capacity = 44): string[] {
  const lines: string[] = [];
  for (const raw of text.split("\n")) {
    let line = ""; let width = 0;
    for (const character of raw) {
      const unit = character.charCodeAt(0) < 128 ? 0.58 : 1;
      if (width + unit > capacity && line) { lines.push(line); line = ""; width = 0; }
      line += character; width += unit;
    }
    lines.push(line);
  }
  return lines;
}

export async function presentationBase64(title: string, body: string): Promise<string> {
  if (body.length > 1_000_000) throw new Error("演示文稿内容过长，请拆分后导出。");
  const document = new DOMParser().parseFromString(noteHtml(title, body), "text/html");
  const pptx = new PptxGenJS();
  pptx.layout = "LAYOUT_WIDE";
  pptx.author = ""; pptx.company = ""; pptx.subject = ""; pptx.title = title;
  pptx.theme = { headFontFace: FONT, bodyFontFace: FONT };
  let slide: PptxGenJS.Slide | undefined;
  let heading = title;
  let y = BODY_TOP;
  let count = 0;
  let hasBody = false;

  function newSlide(continued = false) {
    if (++count > MAX_SLIDES) throw new Error("演示文稿超过 200 页，请拆分后导出。");
    slide = pptx.addSlide(); y = BODY_TOP; hasBody = false;
    const lines = wrapPresentationText(heading + (continued ? "（续）" : ""), 32);
    if (lines.length > 3) throw new Error("幻灯片标题过长，请缩短一级或二级标题。");
    slide.addText(lines.join("\n"), { x: 0.6, y: 0.28, w: 12.1, h: 0.88, fontFace: FONT, fontSize: lines.length > 2 ? 20 : 24, bold: true, color: "1E3A5F", margin: 0, breakLine: false });
    slide.addText(String(count), { x: 12, y: 7.08, w: 0.65, h: 0.2, fontSize: 10, color: "667085", align: "right", margin: 0 });
  }

  function paragraph(text: string, bold = false) {
    if (!text.trim()) return;
    const lines = wrapPresentationText(text.trim());
    while (lines.length) {
      if (!slide) newSlide();
      const available = Math.floor((BODY_BOTTOM - y) / 0.34);
      if (available <= 0) { newSlide(true); continue; }
      const chunk = lines.splice(0, available);
      const height = chunk.length * 0.34;
      slide!.addText(chunk.join("\n"), { x: 0.62, y, w: 12.04, h: height, fontSize: 18, fontFace: FONT, margin: 0, color: "182230", bold, valign: "top", breakLine: false, lineSpacingMultiple: 1.0 });
      y += height + 0.1; hasBody = true;
    }
  }

  function table(element: Element) {
    const rows = [...element.querySelectorAll("tr")].map(row => [...row.children].map(cell => textContent(cell).trim()));
    const columns = Math.max(...rows.map(row => row.length));
    if (!rows.length) return;
    if (columns > 12 || rows.length * columns > 4000) throw new Error("表格超过 12 列或 4000 个单元格，请拆分后导出。");
    // Start tables on a fresh slide, then paginate rows using measured line counts.
    if (!slide || hasBody) newSlide(hasBody);
    const widths = Array.from({ length: columns }, () => 12.1 / columns);
    const capacity = Math.max(3, (widths[0] * 72 - 14) / 16);
    let pageRows: PptxGenJS.TableRow[] = [];
    let heights: number[] = [];
    let tableTop = y;
    function flush() {
      if (!pageRows.length) return;
      slide!.addTable(pageRows, { x: 0.6, y: tableTop, w: 12.1, colW: widths, rowH: heights, fontSize: 16, fontFace: FONT, margin: 0.06, border: { type: "solid", color: "CBD5E1", pt: 0.5 }, color: "182230", fill: { color: "FFFFFF" }, valign: "top", autoPage: false });
      pageRows = []; heights = []; tableTop = y;
    }
    const header = rows[0].map(cell => wrapPresentationText(cell, capacity).join("\n"));
    const headerHeight = Math.max(...header.map(cell => cell.split("\n").length)) * 0.29 + 0.18;
    for (const row of rows) {
      while (row.length < columns) row.push("");
      const wrapped = row.map(cell => wrapPresentationText(cell, capacity));
      while (wrapped.some(cell => cell.length)) {
        let available = Math.min(12, Math.floor((BODY_BOTTOM - y - 0.2) / 0.29));
        if (available < 1) {
          flush(); newSlide(true); tableTop = y;
          if (row !== rows[0] && headerHeight < 1.4) {
            pageRows.push(header.map(text => ({ text, options: { bold: true } })));
            heights.push(headerHeight); y += headerHeight;
          }
          available = Math.min(12, Math.floor((BODY_BOTTOM - y - 0.2) / 0.29));
        }
        const chunk = wrapped.map(cell => cell.splice(0, available).join("\n"));
        const height = Math.max(...chunk.map(cell => cell.split("\n").length)) * 0.29 + 0.18;
        pageRows.push(chunk.map(text => ({ text, options: { bold: row === rows[0] } })));
        heights.push(height);
        y += height; hasBody = true;
      }
    }
    flush();
    y += 0.15;
  }

  function block(element: Element) {
    if (element.tagName === "DIV") { for (const child of element.children) block(child); }
    else if (["H1", "H2"].includes(element.tagName)) {
      const next = textContent(element).trim();
      heading = next || title; slide = undefined; hasBody = false;
    } else if (element.tagName === "HR") { slide = undefined; hasBody = false; }
    else if (element.tagName === "TABLE") table(element);
    else paragraph(textContent(element), /^H[3-6]$/.test(element.tagName));
  }
  for (const element of document.body.children) block(element);
  if (!slide) newSlide();
  return await pptx.write({ outputType: "base64", compression: true }) as string;
}
