import { useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import MarkdownMessage from "./MarkdownMessage";
import PdfReader from "./PdfReader";
import { useWorkspaceState } from "./workspace-state";
import DraftStatus from "./DraftStatus";
import { emptyReader, appendToNote, citationMarkdown, citationLocation, type Citation } from "./review-flow";
import { loadModelEndpoints } from "./model-config";
import { useVerifiedEndpointIds } from "./model-session";

export type ConvertedDocument = {
  id: string;
  markdown: string;
  pageCount?: number;
  sizeBytes: number;
  sourceKind: string;
  sourceName: string;
  warnings: string[];
};

type ChatResult = {
  content: string;
  requestId?: string;
};

function safeError(error: unknown, fallback: string) {
  return typeof error === "string" && error.length <= 240 ? error : fallback;
}

function analysisInput(markdown: string) {
  const limited = Array.from(markdown).slice(0, 28_000).join("");
  return {
    content: limited,
    truncated: limited.length < markdown.length,
  };
}

function outputName(sourceName: string) {
  return `${sourceName.replace(/\.pdf$/i, "").replace(/[\\/:*?"<>|]/g, "_") || "datasheet"}-overview.md`;
}

function pdfOutputName(sourceName: string) {
  return `${sourceName.replace(/\.pdf$/i, "").replace(/[\\/:*?"<>|]/g, "_") || "datasheet"}-overview.pdf`;
}

export default function DatasheetWorkspace() {
  const textView = useRef<HTMLPreElement>(null);
  const [source, setSource] = useState<ConvertedDocument | null>(null);
  const [pdfUrl, setPdfUrl] = useState("");
  const [reader, setReader, readerState, retryReader] = useWorkspaceState("reader", emptyReader);
  const { tab, summary, requestId } = reader;
  const setTab = (tab: "pdf" | "text" | "summary") => setReader(current => ({ ...current, tab }));
  const setSummary = (summary: string) => setReader(current => ({ ...current, summary }));
  const setRequestId = (requestId: string) => setReader(current => ({ ...current, requestId }));
  const [projects, setProjects] = useState<Array<{ id: string; name: string }>>([]);
  const [selection, setSelection] = useState("");
  const [scope, setScope] = useState("page");
  const [notice, setNotice] = useState("");
  const [busy, setBusy] = useState(false);
  const verifiedEndpointIds = useVerifiedEndpointIds();
  const endpoint = useMemo(
    () => loadModelEndpoints().find((item) => item.enabled && verifiedEndpointIds.includes(item.id)),
    [verifiedEndpointIds],
  );
  useEffect(() => { invoke<Array<{ id: string; name: string }>>("list_projects").then(setProjects).catch(() => undefined); }, []);
  useEffect(() => {
    if (!readerState.ready) return;
    let current = true;
    setSource(null); setPdfUrl(""); setSelection("");
    const request = reader.sourceId
      ? invoke<{ id: string; document: ConvertedDocument; pdfUrl?: string }>("load_review_source", { sourceId: reader.sourceId })
      : invoke<{ id: string; document: ConvertedDocument; pdfUrl?: string }>("register_review_source", { path: null });
    request.then(value => {
      if (!current) return;
      setSource(value.document); setPdfUrl(value.pdfUrl ?? "");
      setReader(state => ({ ...state, sourceId: value.id, sourceName: value.document.sourceName }));
    }).catch(error => { if (current && reader.sourceId) setNotice(safeError(error, "原件不可用，历史笔记和摘录仍保留。")); });
    return () => { current = false; };
  }, [reader.sourceId, readerState.ready]);
  const pages = useMemo(() => {
    if (!source) return [];
    if (source.sourceKind !== "PDF") return [{ page: 1, text: source.markdown }];
    return source.markdown.split(/## 第 (\d+) 页\n/).slice(1).reduce<Array<{ page: number; text: string }>>((all, value, index, parts) => { if (index % 2 === 0) all.push({ page: Number(value), text: parts[index + 1] ?? "" }); return all; }, []);
  }, [source]);
  const currentPage = pages.find(p => p.page === reader.page) ?? pages[0];
  const matches = reader.find.trim() ? pages.filter(p => p.text.toLowerCase().includes(reader.find.trim().toLowerCase())) : [];
  useEffect(() => {
    if (reader.tab === "text" && reader.find.trim()) textView.current?.querySelector("mark")?.scrollIntoView?.({ block: "center" });
  }, [reader.tab, reader.page, reader.find, source]);
  async function retainQuote(text: string, page: number) {
    if (!source || !reader.sourceId) return;
    const quote: Citation = { sourceId: reader.sourceId, sourceName: source.sourceName, version: source.id, page, quote: text.trim(), sourceKind: source.sourceKind };
    try {
      await invoke("validate_review_citation", { citation: quote });
      setReader(current => ({ ...current, highlights: [...current.highlights.filter(c => !(c.page === page && c.quote === quote.quote)), quote].slice(-50) }));
      setSelection(""); setNotice(`已保留第 ${page} 页原文，可整理到评审笔记。`);
    } catch (error) { setNotice(safeError(error, "引用未通过原文校验，请从按页文本重新选取。")); }
  }
  async function toNote(includeSummary = false) {
    const name = source?.sourceName || reader.sourceName;
    if (!name) return;
    try {
      const body = [includeSummary ? `## 待复核的 AI 分析\n\n${summary}` : "## 原文证据", ...reader.highlights.map(citationMarkdown)].join("\n\n");
      await appendToNote(`${name} 评审`, body, reader.highlights, reader.projectId);
    } catch { setNotice("评审草稿未保存，请重试；原始摘录仍保留。 "); }
  }

  async function importPdf() {
    setNotice("");
    try {
      const path = await open({
        directory: false,
        multiple: false,
        filters: [{ name: "PDF Datasheet", extensions: ["pdf"] }],
      });
      if (typeof path !== "string") return;
      setBusy(true);
      const registered = await invoke<{ id: string; document: ConvertedDocument; pdfUrl?: string }>("register_review_source", { path });
      const document = registered.document;
      if (document.sourceKind !== "PDF") throw new Error("请选择 PDF Datasheet。");
      setSource(document);
      setReader({ ...emptyReader, sourceId: registered.id, sourceName: document.sourceName, projectId: reader.projectId });
      setNotice(`已在本机按页提取 ${document.pageCount ?? 0} 页；尚未调用模型。`);
    } catch (error) {
      setNotice(safeError(error, "Datasheet 导入失败。"));
    } finally {
      setBusy(false);
    }
  }

  async function analyze() {
    if (!source || !endpoint) return;
    setBusy(true);
    setNotice("");
    const analyzedSourceId = reader.sourceId;
    const text = scope === "evidence" ? reader.highlights.map(citationMarkdown).join("\n\n") : scope === "page" ? `## 第 ${reader.page} 页\n${currentPage?.text ?? ""}` : source.markdown;
    const input = analysisInput(text);
    try {
      const result = await invoke<ChatResult>("send_chat_completion", {
        endpointId: endpoint.id,
        baseUrl: endpoint.baseUrl,
        model: endpoint.model,
        messages: [
          {
            role: "system",
            content: [
              "你是模拟 IC Datasheet 摘要助手。只根据给定的按页文本生成中文 Markdown。",
              "必须依次包含：器件概述、主要特性、关键电气参数表、引脚与封装、典型应用、限制与绝对最大额定值、待人工复核项、来源定位。",
              "每个参数保留数值、单位、条件，并在结论后标注[第 N 页]。无法确认的字段写“原文未确认”，严禁补造参数。",
            ].join(""),
            attachments: [],
          },
          {
            role: "user",
            content: `${reader.question.trim() ? `请优先回答：${reader.question.trim()}\n\n` : ""}${input.truncated ? "注意：文档内容因请求上限只包含前部页面。\n\n" : ""}${input.content}`,
            attachments: [],
          },
        ],
      });
      setReader(current => current.sourceId === analyzedSourceId ? { ...current, summary: result.content, tab: "summary", requestId: result.requestId ?? "" } : current);
      setNotice("概略版已生成；工程参数必须结合页码和原文复核。 ");
    } catch (error) {
      setNotice(safeError(error, "Datasheet AI 分析失败，本地按页文本仍可预览。"));
    } finally {
      setBusy(false);
    }
  }

  async function exportSummary() {
    if (!source || !summary) return;
    try {
      const name = outputName(source.sourceName);
      const destination = await save({
        defaultPath: name,
        filters: [{ name: "Markdown", extensions: ["md"] }],
      });
      if (!destination) return;
      const content = `# ${source.sourceName} 概略版\n\n> 由本地/内网模型基于用户明确选择的文件生成，必须结合来源页复核。\n\n${summary}`;
      await invoke("export_markdown", { path: destination, content });
      setNotice(`已导出 ${destination.split(/[\\/]/).pop() || name}。`);
    } catch (error) {
      setNotice(safeError(error, "Datasheet 概略版导出失败。"));
    }
  }

  async function exportSummaryPdf() {
    if (!source || !summary) return;
    try {
      const name = pdfOutputName(source.sourceName);
      const destination = await save({
        defaultPath: name,
        filters: [{ name: "PDF", extensions: ["pdf"] }],
      });
      if (!destination) return;
      const title = `${source.sourceName} 概略版`;
      const content = `# ${title}\n\n> 由本地/内网模型基于用户明确选择的文件生成，必须结合来源页复核。\n\n${summary}`;
      await invoke("export_pdf", { path: destination, title, markdown: content });
      setNotice(`已导出 ${destination.split(/[\\/]/).pop() || name}。`);
    } catch (error) {
      setNotice(safeError(error, "Datasheet PDF 导出失败。"));
    }
  }

  return (
    <section className="datasheet-workspace" aria-labelledby="datasheet-workspace-title">
      <header className="workspace-action-header">
        <div>
          <p className="eyebrow">TRACEABLE DOCUMENT INTELLIGENCE</p>
          <h2 id="datasheet-workspace-title">Datasheet 分析</h2>
          <p>本机按页提取原文，模型只接收受限文本；所有工程结论必须保留单位、条件和来源页。</p>
        </div>
        <button className="primary-button" type="button" disabled={busy} onClick={importPdf}>
          {busy ? "处理中…" : "导入 PDF Datasheet"}
        </button>
      </header>

      {notice ? <p className="converter-notice" role="status">{notice}</p> : null}

      <DraftStatus state={readerState} retry={retryReader} />
      <div className="datasheet-layout">
        <aside className="datasheet-source">
          <p className="eyebrow">SOURCE CONTROL</p>
          {source ? (
            <>
              <strong>{source.sourceName}</strong>
              <label>关联项目<select value={reader.projectId} onChange={e => setReader({ ...reader, projectId: e.target.value })}><option value="">不关联</option>{projects.map(p => <option key={p.id} value={p.id}>{p.name}</option>)}</select></label>
              <label>查找原文<input aria-label="查找资料原文" placeholder="中文 / 型号 / 参数" value={reader.find} onChange={e => setReader({ ...reader, find: e.target.value })}/></label>
              {reader.find ? <div className="source-search-results"><small>{matches.length ? `命中 ${matches.length} 页` : "未找到；扫描件需要可提取文本"}</small>{matches.slice(0, 100).map(p => <button key={p.page} onClick={() => { setSelection(""); setReader({ ...reader, page: p.page, tab: "text" }); }}>第 {p.page} 页 · {p.text.slice(Math.max(0, p.text.toLowerCase().indexOf(reader.find.toLowerCase()) - 15), p.text.toLowerCase().indexOf(reader.find.toLowerCase()) + 70)}</button>)}</div> : null}
              <span>{source.pageCount ?? pages.length} 页 · {(source.sizeBytes / 1024).toFixed(1)} KiB</span>
              <ul>{source.warnings.map((warning) => <li key={warning}>{warning}</li>)}</ul>
              <label>待核查问题<textarea aria-label="待核查问题" maxLength={2000} placeholder="如：PSRR 在什么频率与负载下成立？" value={reader.question} onChange={e => setReader({ ...reader, question: e.target.value })}/></label>
              <label>提交给模型的范围<select value={scope} onChange={e => setScope(e.target.value)}><option value="page">当前第 {reader.page} 页</option><option value="evidence">已选的 {reader.highlights.length} 条证据</option><option value="all">整份文本（最多 28,000 字）</option></select></label>
              <button className="primary-button" type="button" disabled={!endpoint || busy || (scope === "evidence" && !reader.highlights.length)} onClick={analyze}>
                {endpoint ? "生成可追溯概略版" : "连接模型后分析"}
              </button>
              {!endpoint ? <small>请先在智能对话中完成模型连接检测。</small> : null}
            </>
          ) : (
            <div className="conversion-empty">尚未选择 PDF</div>
          )}
        </aside>

        <main className="datasheet-preview">
          <div className="datasheet-tabs" aria-label="Datasheet 预览">
            <button type="button" className={tab === "pdf" ? "active" : ""} onClick={() => setTab("pdf")}>{source?.sourceKind === "PDF" ? "PDF 原件" : "来源原文"}</button>
            <button type="button" className={tab === "text" ? "active" : ""} onClick={() => setTab("text")}>按页文本</button>
            <button type="button" className={tab === "summary" ? "active" : ""} disabled={!summary} onClick={() => setTab("summary")}>AI 概略版</button>
            {summary ? (
              <div className="datasheet-export-actions">
                <button className="secondary-button" type="button" onClick={exportSummary}>导出 Markdown</button>
                <button className="secondary-button" type="button" onClick={exportSummaryPdf}>导出 PDF</button>
              </div>
            ) : null}
          </div>
          {tab === "summary" && summary ? (
            <section aria-label="Datasheet 概略版">
              <MarkdownMessage text={summary} />
              {requestId ? <small>请求 ID：{requestId}</small> : null}
            </section>
          ) : tab === "pdf" && pdfUrl ? (
            <PdfReader title="PDF 原件阅读器" source={pdfUrl} page={reader.page} zoom={reader.zoom} onPageChange={page => setReader(current => ({ ...current, page }))} onZoomChange={zoom => setReader(current => ({ ...current, zoom }))} onQuote={retainQuote} />
          ) : source ? (
            <section className="source-text-view"><div className="pdf-toolbar"><button disabled={reader.page <= 1} onClick={() => { setSelection(""); setReader({ ...reader, page: reader.page - 1 }); }}>上一页文本</button><span>第 {currentPage?.page ?? 1} / {pages.length} 页</span><button disabled={reader.page >= pages.length} onClick={() => { setSelection(""); setReader({ ...reader, page: reader.page + 1 }); }}>下一页文本</button><button disabled={!selection.trim()} onClick={() => retainQuote(selection, reader.page)}>保留选中文本</button><button disabled={!currentPage?.text.trim() || currentPage.text.trim().length > 8000} title="保留当前页完整原文；超过 8000 字时请选取所需段落" onClick={() => currentPage && retainQuote(currentPage.text, currentPage.page)}>引用本页</button></div><pre ref={textView} tabIndex={0} aria-label="当前页原文" onMouseDown={() => setSelection("")} onMouseUp={e => { const selected = window.getSelection(); if (selected?.rangeCount && e.currentTarget.contains(selected.getRangeAt(0).commonAncestorContainer)) setSelection(selected.toString().slice(0, 8000)); }}>{reader.find.trim() ? highlightText(currentPage?.text ?? "", reader.find.trim()) : currentPage?.text}</pre></section>
          ) : (
            <div className="conversion-placeholder"><span>▤</span><strong>等待本地 Datasheet</strong><p>测试与 Demo 只使用合成或脱敏资料。</p></div>
          )}
          {reader.highlights.length ? <section className="evidence-list" aria-label="原文证据"><header><h3>已选原文证据 · {reader.highlights.length}</h3><button className="primary-button" onClick={() => toNote()}>整理到评审笔记</button></header>{reader.highlights.map((c, index) => <article key={`${c.page}-${index}`}><button onClick={() => setReader({ ...reader, page: c.page, tab: "pdf" })}>{citationLocation(c)}</button><blockquote>{c.quote}</blockquote><button className="text-button" onClick={() => setReader({ ...reader, highlights: reader.highlights.filter((_, i) => i !== index) })}>移除摘录</button></article>)}</section> : null}
          {summary ? <button className="secondary-button" onClick={() => toNote(true)}>将 AI 分析和证据加入评审草稿</button> : null}
        </main>
      </div>
    </section>
  );
}

function highlightText(text: string, query: string) {
  const parts = []; let start = 0; const lower = text.toLowerCase(), needle = query.toLowerCase();
  for (let at = lower.indexOf(needle); at >= 0 && parts.length < 200; at = lower.indexOf(needle, start)) {
    parts.push(text.slice(start, at), <mark key={at}>{text.slice(at, at + query.length)}</mark>); start = at + query.length;
  }
  parts.push(text.slice(start)); return parts;
}
