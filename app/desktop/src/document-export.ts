import { noteDocx, noteHtml } from "./note-export";

export const documentFormats = ["md", "pdf", "docx", "pptx", "txt"] as const;
export type DocumentFormat = typeof documentFormats[number];
export const documentExtensions = ["md", "markdown", "txt", "pdf", "docx", "pptx"];

export function documentTitleBody(sourceName: string, markdown: string) {
  const first = /^(?:\uFEFF)?# ([^\n]+)\r?\n(?:\r?\n)?/.exec(markdown);
  return {
    title: (first?.[1].trim() || sourceName.replace(/\.[^.]+$/, "") || "文档").slice(0, 60),
    body: first ? markdown.slice(first[0].length) : markdown,
  };
}

// The same sanitized renderer is used for notes and document conversion. No HTML
// from the source is executed and no image URL is fetched by this text traversal.
export function textContent(node: Node): string {
  if (node.nodeType === Node.TEXT_NODE) return node.textContent || "";
  if (!(node instanceof Element)) return "";
  if (node.tagName === "BR") return "\n";
  if (node.tagName === "IMG") return `[图片：${node.getAttribute("alt") || "未命名"}，此格式未转换图片]`;
  if (node.tagName === "INPUT") return node.hasAttribute("checked") ? "☑ " : "☐ ";
  const children = [...node.childNodes].map(textContent).join("");
  if (["TH", "TD"].includes(node.tagName)) return `${children}\t`;
  if (node.tagName === "LI") return `• ${children}\n`;
  if (["P", "H1", "H2", "H3", "H4", "H5", "H6", "TR", "PRE", "BLOCKQUOTE"].includes(node.tagName)) return `${children}\n`;
  return children;
}

export function documentText(title: string, body: string) {
  return textContent(new DOMParser().parseFromString(noteHtml(title, body), "text/html").body).trim() + "\n";
}

export async function formattedDocument(format: DocumentFormat, title: string, body: string) {
  if (format === "docx") return noteDocx(title, body);
  if (format === "pptx") return (await import("./presentation-export")).presentationBase64(title, body);
  if (format === "txt") return documentText(title, body);
  return null;
}
