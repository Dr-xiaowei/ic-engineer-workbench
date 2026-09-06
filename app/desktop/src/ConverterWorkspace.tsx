import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import MarkdownMessage from "./MarkdownMessage";

type ConvertedDocument = {
  id: string;
  markdown: string;
  pageCount?: number;
  sizeBytes: number;
  sourceKind: string;
  sourceName: string;
  warnings: string[];
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

function outputName(sourceName: string) {
  return `${sourceName.replace(/\.[^.]+$/, "").replace(/[\\/:*?"<>|]/g, "_") || "document"}.md`;
}

function directoryLabel(path: string) {
  if (!path) return "每次导出时选择";
  return path.split(/[\\/]/).filter(Boolean).at(-1) || "已选择目录";
}

function safeErrorMessage(error: unknown, fallback: string) {
  return typeof error === "string" && error.length <= 240 ? error : fallback;
}

export default function ConverterWorkspace() {
  const [documents, setDocuments] = useState<ConvertedDocument[]>([]);
  const [failures, setFailures] = useState<ConversionFailure[]>([]);
  const [selectedId, setSelectedId] = useState("");
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState("");
  const [outputDirectory, setOutputDirectory] = useState(initialOutputDirectory);
  const selected = documents.find((document) => document.id === selectedId) ?? documents[0];

  async function chooseDocuments() {
    setNotice("");
    let selection: string | string[] | null;
    try {
      selection = await open({
        multiple: true,
        directory: false,
        filters: [{ name: "支持的文档", extensions: ["txt", "md", "docx", "pdf"] }],
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
        converted.push(await invoke<ConvertedDocument>("convert_document", { path }));
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
        ? `已在本机转换 ${converted.length} 个文档${rejected.length ? `，${rejected.length} 个失败` : ""}。`
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
    try {
      const name = outputName(selected.sourceName);
      const destination = await save({
        defaultPath: outputDirectory ? `${outputDirectory}/${name}` : name,
        filters: [{ name: "Markdown", extensions: ["md"] }],
      });
      if (!destination) return;
      await invoke("export_markdown", { path: destination, content: selected.markdown });
      setNotice(`已导出 ${name}。`);
    } catch (error) {
      setNotice(safeErrorMessage(error, "Markdown 导出失败。"));
    }
  }

  return (
    <section className="converter-workspace" aria-labelledby="converter-title">
      <header className="workspace-action-header">
        <div>
          <p className="eyebrow">LOCAL DOCUMENT PIPELINE</p>
          <h2 id="converter-title">文件转 Markdown</h2>
          <p>仅处理你通过系统对话框明确选择的本地文件，转换失败不会修改原文件。</p>
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
        <button className="secondary-button" type="button" onClick={chooseOutputDirectory}>设置目录</button>
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
                <button className="secondary-button" type="button" onClick={exportSelected}>另存为 .md</button>
              </header>
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
              <p>支持 UTF-8 TXT/Markdown、DOCX 和含文本层的 PDF。</p>
            </div>
          )}
        </main>
      </div>
    </section>
  );
}
