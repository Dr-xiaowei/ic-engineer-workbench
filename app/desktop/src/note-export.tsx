import { renderToStaticMarkup } from "react-dom/server";
import { Document, Paragraph, TextRun, Table, TableRow, TableCell, ImageRun, Packer, HeadingLevel, WidthType, type ParagraphChild } from "docx";
import MarkdownMessage from "./MarkdownMessage";

export function noteHtml(title: string, body: string): string {
  return '<!doctype html><html><head><meta charset="utf-8"><style>body{font:11pt sans-serif;line-height:1.5}h1{font-size:22pt}h2{font-size:16pt}table{border-collapse:collapse;width:100%}td,th{border:1px solid #aaa;padding:6px}img{max-width:100%}pre{white-space:pre-wrap}</style></head><body>' + renderToStaticMarkup(<><h1>{title}</h1><MarkdownMessage allowDataImages text={body}/></>) + '</body></html>';
}

async function inline(node: Node, bold = false, italics = false): Promise<ParagraphChild[]> {
  if (node.nodeType === Node.TEXT_NODE) return [new TextRun({ text: node.textContent || "", bold, italics })];
  if (!(node instanceof Element)) return [];
  if (node.tagName === "BR") return [new TextRun({ break: 1 })];
  if (node.tagName === "INPUT") return [new TextRun(node.hasAttribute("checked") ? "☑ " : "☐ ")];
  if (node.tagName === "IMG") {
    const src = node.getAttribute("src") || "";
    if (!/^data:image\/(png|jpeg|webp);base64,/i.test(src)) return [new TextRun("[图片引用已阻止]")];
    const image = new Image(); image.src = src;
    await image.decode();
    if (!image.naturalWidth || image.naturalWidth * image.naturalHeight > 16_000_000) throw new Error("图片尺寸过大，无法导出。");
    const scale = Math.min(1, 560 / image.naturalWidth, 660 / image.naturalHeight);
    const canvas = document.createElement("canvas"); canvas.width = Math.ceil(image.naturalWidth * scale); canvas.height = Math.ceil(image.naturalHeight * scale);
    canvas.getContext("2d")!.drawImage(image, 0, 0, canvas.width, canvas.height);
    return [new ImageRun({ type: "png", data: Uint8Array.from(atob(canvas.toDataURL("image/png").split(",")[1]), c => c.charCodeAt(0)), transformation: { width: canvas.width, height: canvas.height }, altText: { title: node.getAttribute("alt") || "笔记图片", description: "本地笔记图片", name: "image" } })];
  }
  return (await Promise.all([...node.childNodes].map(child => inline(child, bold || node.tagName === "STRONG", italics || node.tagName === "EM")))).flat();
}

export async function noteDocx(title: string, body: string): Promise<string> {
  const parsed = new DOMParser().parseFromString(noteHtml(title, body), "text/html");
  const blocks: (Paragraph | Table)[] = [];
  async function block(element: Element) {
    if (element.tagName === "DIV" || ["UL", "OL", "BLOCKQUOTE"].includes(element.tagName)) {
      for (const child of element.children) await block(child);
    } else if (element.tagName === "TABLE") {
      const rows: TableRow[] = [];
      for (const row of element.querySelectorAll("tr")) {
        const cells: TableCell[] = [];
        for (const cell of row.children) cells.push(new TableCell({ children: [new Paragraph({ children: await inline(cell, cell.tagName === "TH") })] }));
        rows.push(new TableRow({ children: cells, tableHeader: row.parentElement?.tagName === "THEAD" }));
      }
      if (rows.length) blocks.push(new Table({ width: { size: 100, type: WidthType.PERCENTAGE }, rows }));
    } else {
      const heading = ({ H1: HeadingLevel.HEADING_1, H2: HeadingLevel.HEADING_2, H3: HeadingLevel.HEADING_3 } as Record<string, (typeof HeadingLevel)[keyof typeof HeadingLevel]>)[element.tagName];
      blocks.push(new Paragraph({ children: await inline(element), heading, bullet: element.tagName === "LI" ? { level: 0 } : undefined, spacing: { after: 140 } }));
    }
  }
  for (const element of parsed.body.children) await block(element);
  return Packer.toBase64String(new Document({ title, creator: "", lastModifiedBy: "", styles: { default: { document: { run: { font: "Noto Sans CJK SC", size: 22 } } } }, sections: [{ children: blocks }] }));
}
