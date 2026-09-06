import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import MarkdownMessage from "./MarkdownMessage";
import PdfReader from "./PdfReader";
import { loadModelEndpoints } from "./model-config";
import { useVerifiedEndpointIds } from "./model-session";

type ConvertedDocument = {
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
  const [source, setSource] = useState<ConvertedDocument | null>(null);
  const [pdfUrl, setPdfUrl] = useState("");
  const [tab, setTab] = useState<"pdf" | "text" | "summary">("pdf");
  const [summary, setSummary] = useState("");
  const [requestId, setRequestId] = useState("");
  const [notice, setNotice] = useState("");
  const [busy, setBusy] = useState(false);
  const verifiedEndpointIds = useVerifiedEndpointIds();
  const endpoint = useMemo(
    () => loadModelEndpoints().find((item) => item.enabled && verifiedEndpointIds.includes(item.id)),
    [verifiedEndpointIds],
  );
  useEffect(() => { invoke<{document: ConvertedDocument; pdfUrl: string}>("load_demo_datasheet").then((value) => { setSource(value.document); setPdfUrl(value.pdfUrl); setTab("pdf"); setNotice("已载入独立 Demo 的合成 Datasheet，可直接查看原件并体验 AI 概略。"); }).catch(() => undefined); }, []);

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
      const document = await invoke<ConvertedDocument>("convert_document", { path });
      if (document.sourceKind !== "PDF") throw new Error("请选择 PDF Datasheet。");
      setSource(document);
      setPdfUrl(await invoke<string>("load_pdf_data_url", { path }));
      setTab("pdf");
      setSummary("");
      setRequestId("");
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
    const input = analysisInput(source.markdown);
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
            content: `${input.truncated ? "注意：文档内容因请求上限只包含前部页面。\n\n" : ""}${input.content}`,
            attachments: [],
          },
        ],
      });
      setSummary(result.content);
      setTab("summary");
      setRequestId(result.requestId ?? "");
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
      setNotice(`已导出 ${name}。`);
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
      setNotice(`已导出 ${name}。`);
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

      <div className="datasheet-layout">
        <aside className="datasheet-source">
          <p className="eyebrow">SOURCE CONTROL</p>
          {source ? (
            <>
              <strong>{source.sourceName}</strong>
              <span>{source.pageCount} 页 · {(source.sizeBytes / 1024).toFixed(1)} KiB</span>
              <ul>{source.warnings.map((warning) => <li key={warning}>{warning}</li>)}</ul>
              <button className="primary-button" type="button" disabled={!endpoint || busy} onClick={analyze}>
                {endpoint ? "生成可追溯概略版" : "连接模型后分析"}
              </button>
              {!endpoint ? <small>请先在智能对话中完成模型连接检测。</small> : null}
            </>
          ) : (
            <div className="conversion-empty">尚未选择 PDF</div>
          )}
        </aside>

        <main className="datasheet-preview">
          <div className="datasheet-tabs" role="tablist" aria-label="Datasheet 预览">
            <button type="button" className={tab === "pdf" ? "active" : ""} onClick={() => setTab("pdf")}>PDF 原件</button>
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
            <PdfReader title="PDF 原件阅读器" source={pdfUrl} />
          ) : source ? (
            <MarkdownMessage text={source.markdown} />
          ) : (
            <div className="conversion-placeholder"><span>▤</span><strong>等待本地 Datasheet</strong><p>测试与 Demo 只使用合成或脱敏资料。</p></div>
          )}
        </main>
      </div>
    </section>
  );
}
