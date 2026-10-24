import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import MarkdownMessage from "./MarkdownMessage";
import { documentExtensions, documentFormats, documentTitleBody, formattedDocument, type DocumentFormat } from "./document-export";

type ConvertedDocument = {
  id: string;
  markdown: string;
  pageCount?: number;
  sizeBytes: number;
  sourceKind: string;
  sourceName: string;
  warnings: string[];
  sourcePath: string;
};

type ConversionFailure = {
  error: string;
  sourceName: string;
};

const outputDirectoryKey = "ic-workbench-output-directory-v1";

function initialOutputDirectory() {
  try {
    const value = window.localStorage.getItem(outputDirectoryKey) ?? "";
    return value.length <= 4096 ? value : "";
  } catch {
    return "";
  }
}

function fileNameFromPath(path: string) {
  return path.split(/[\\/]/).filter(Boolean).at(-1)?.slice(0, 180) || "未知文件";
}

function outputName(sourceName: string, format: DocumentFormat) {
  return `${sourceName.replace(/\.[^.]+$/, "").replace(/[\\/:*?"<>|]/g, "_") || "document"}_转换.${format}`;
}

function directoryLabel(path: string) {
  if (!path) return "每次导出时选择";
  return path.split(/[\\/]/).filter(Boolean).at(-1) || "已选择目录";
}

function safeErrorMessage(error: unknown, fallback: string) {
  if (error instanceof Error && /^[\u3400-\u9fff]/.test(error.message) && error.message.length <= 240) return error.message;
  return typeof error === "string" && error.length <= 240 ? error : fallback;
}

export default function ConverterWorkspace() {
  const [documents, setDocuments] = useState<ConvertedDocument[]>([]);
  const [failures, setFailures] = useState<ConversionFailure[]>([]);
  const [selectedId, setSelectedId] = useState("");
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState("");
  const [outputDirectory, setOutputDirectory] = useState(initialOutputDirectory);
  const [format, setFormat] = useState<DocumentFormat>("md");
  const selected = documents.find((document) => document.id === selectedId) ?? documents[0];

  async function chooseDocuments() {
    setNotice("");
    let selection: string | string[] | null;
    try {
      selection = await open({
        multiple: true,
        directory: false,
        filters: [{ name: "支持的文档", extensions: documentExtensions }],
      });
    } catch {
      setNotice("文件选择器仅在桌面应用中可用。");
      return;
    }
    const paths = Array.isArray(selection) ? selection : selection ? [selection] : [];
    if (!paths.length) return;
    setBusy(true);
    const converted: ConvertedDocument[] = [];
    const rejected: ConversionFailure[] = [];
    for (const path of paths.slice(0, 20)) {
      try {
        const document = await invoke<ConvertedDocument>("convert_document", { path });
        converted.push({ ...document, sourcePath: path, id: `${document.id}-${converted.length}` });
      } catch (error) {
        rejected.push({
          sourceName: fileNameFromPath(path),
          error: safeErrorMessage(error, "文档转换失败，原文件未被修改。"),
        });
      }
    }
    setDocuments(converted);
    setFailures(rejected);
    setSelectedId(converted[0]?.id ?? "");
    setNotice(
      converted.length
        ? `已在本机转换 ${converted.length} 个文档${rejected.length ? `，${rejected.length} 个失败` : ""}。${paths.length > 20 ? "本次只处理前 20 个，请分批选择其余文件。" : ""}`
        : "没有文档转换成功，原文件均未修改。",
    );
    setBusy(false);
  }

  async function chooseOutputDirectory() {
    try {
      const selection = await open({ directory: true, multiple: false });
      if (typeof selection !== "string") return;
      setOutputDirectory(selection);
      window.localStorage.setItem(outputDirectoryKey, selection);
      setNotice("默认输出目录已保存在当前设备；导出时仍会显示确认对话框。");
    } catch {
      setNotice("输出目录选择器仅在桌面应用中可用。");
    }
  }

  async function exportSelected() {
    if (!selected) return;
    setBusy(true);
    try {
      const name = outputName(selected.sourceName, format);
      const destination = await save({
        defaultPath: outputDirectory ? `${outputDirectory}/${name}` : name,
        filters: [{ name: format.toUpperCase(), extensions: [format] }],
      });
      if (!destination) return;
      const { title, body } = documentTitleBody(selected.sourceName, selected.markdown);
      const formatted = await formattedDocument(format, title, body);
      await invoke("export_converted_document", {
        path: destination, format, title,
        body: format === "md" ? selected.markdown : body || title,
        formatted, sourcePaths: documents.map(document => document.sourcePath),
      });
      setNotice(`已导出 ${fileNameFromPath(destination)}。`);
    } catch (error) {
      setNotice(safeErrorMessage(error, "文档导出失败，已有文件未被覆盖。"));
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="converter-workspace" aria-labelledby="converter-title">
      <header className="workspace-action-header">
        <div>
          <p className="eyebrow">LOCAL DOCUMENT PIPELINE</p>
          <h2 id="converter-title">文档互转</h2>
          <p>选择文件 → 预览内容 → 选择导出格式。全程本地处理，另存为新文件。</p>
        </div>
        <button className="primary-button" type="button" disabled={busy} onClick={chooseDocuments}>
          {busy ? "正在转换…" : "选择文件 / 批量转换"}
        </button>
      </header>

      <div className="converter-toolbar">
        <div>
          <span>默认输出目录</span>
          <strong title={outputDirectory || undefined}>{directoryLabel(outputDirectory)}</strong>
        </div>
        <button className="secondary-button" type="button" disabled={busy} onClick={chooseOutputDirectory}>设置目录</button>
        <span>单文件上限 25 MiB · 单次最多 20 个 · 不执行宏、脚本或嵌入对象</span>
      </div>

      {notice ? <p className="converter-notice" role="status">{notice}</p> : null}

      <div className="converter-layout">
        <aside className="conversion-list" aria-label="转换结果">
          {documents.map((document) => (
            <button
              className={selected?.id === document.id ? "active" : ""}
              key={document.id}
              type="button"
              disabled={busy}
              onClick={() => setSelectedId(document.id)}
            >
              <span>{document.sourceKind}</span>
              <strong>{document.sourceName}</strong>
              <small>{document.pageCount ? `${document.pageCount} 页 · ` : ""}{document.sizeBytes} 字节</small>
            </button>
          ))}
          {failures.map((failure) => (
            <article className="conversion-failure" key={failure.sourceName}>
              <strong>{failure.sourceName}</strong>
              <small>{failure.error}</small>
            </article>
          ))}
          {!documents.length && !failures.length ? (
            <div className="conversion-empty">尚未选择文档</div>
          ) : null}
        </aside>

        <main className="conversion-preview">
          {selected ? (
            <>
              <header>
                <div>
                  <span>MARKDOWN PREVIEW</span>
                  <strong>{selected.sourceName}</strong>
                </div>
                <div className="conversion-export-controls">
                  <label>导出为 <select aria-label="文档导出格式" value={format} disabled={busy} onChange={event => setFormat(event.target.value as DocumentFormat)}>
                    {documentFormats.map(value => <option key={value} value={value}>{value === "md" ? "Markdown" : value.toUpperCase()}</option>)}
                  </select></label>
                  <button className="secondary-button" type="button" disabled={busy} onClick={exportSelected}>{busy ? "处理中…" : "另存为新文件"}</button>
                </div>
              </header>
              <p className="conversion-format-help">正文转换会重新排版，复杂表格、公式和图片请复核。PPTX 导出按一级/二级标题、分隔线和内容长度分页；仅转换文字、列表与基本表格。已存在的输出文件不会被覆盖。</p>
              {selected.warnings.length ? (
                <ul className="conversion-warnings">
                  {selected.warnings.map((warning) => <li key={warning}>{warning}</li>)}
                </ul>
              ) : null}
              <MarkdownMessage text={selected.markdown} />
            </>
          ) : (
            <div className="conversion-placeholder">
              <span>⇄</span>
              <strong>本地结构化预览</strong>
              <p>PDF、Word（DOCX）、PowerPoint（PPTX）、TXT 与 Markdown；选择后可导出为 Markdown、PDF、DOCX、PPTX 或 TXT。</p>
              <p>扫描 PDF 需要 OCR；旧式 .doc/.ppt 请先另存为 .docx/.pptx。图片、动画和原始版式不保证保留。</p>
            </div>
          )}
        </main>
      </div>
    </section>
  );
}
