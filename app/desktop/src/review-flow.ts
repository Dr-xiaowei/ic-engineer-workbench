import { updateWorkspaceState } from "./workspace-state";
export type Citation = { sourceId: string; sourceName: string; version: string; page: number; quote: string; sourceKind?: string };
export type NoteDraft = { id: string; title: string; body: string; projectId: string; citations: Citation[] };
export const emptyNote: NoteDraft = { id: "", title: "", body: "", projectId: "", citations: [] };
export type ReaderState = { sourceId: string; sourceName: string; page: number; zoom: number; summary: string; requestId: string; question: string; tab: "pdf" | "text" | "summary"; projectId: string; find: string; highlights: Citation[] };
export const emptyReader: ReaderState = { sourceId: "", sourceName: "", page: 1, zoom: 1, summary: "", requestId: "", question: "", tab: "pdf", projectId: "", find: "", highlights: [] };
export function navigateTo(id: string) { window.dispatchEvent(new CustomEvent("workbench:navigate", { detail: id })); }
export async function openSource(sourceId: string, page = 1, projectId = "") {
  await updateWorkspaceState<ReaderState>("reader", emptyReader, current => current.sourceId === sourceId ? { ...current, page, tab: "pdf" } : { ...emptyReader, sourceId, page, projectId });
  navigateTo("datasheet");
}
export async function appendToNote(title: string, body: string, citations: Citation[] = [], projectId = "") {
  await updateWorkspaceState("note-draft", emptyNote, current => ({ ...current, title: current.title || title, body: [current.body, body].filter(Boolean).join("\n\n"), projectId: current.projectId || projectId, citations: [...current.citations, ...citations.filter(c => !current.citations.some(old => old.sourceId === c.sourceId && old.page === c.page && old.quote === c.quote))] }));
  navigateTo("knowledge");
}
export function citationMarkdown(c: Citation) { return `> ${c.quote.replace(/\n/g, "\n> ")}\n\n来源：${c.sourceName} · ${citationLocation(c)} · 版本 ${c.version.slice(0, 12)}`; }

export function citationLocation(c: Citation) { return c.sourceKind && c.sourceKind !== "PDF" ? "提取文本" : `第 ${c.page} 页`; }
