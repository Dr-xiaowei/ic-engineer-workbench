import { useEffect, useMemo, useState, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import MarkdownMessage from "./MarkdownMessage";
import { loadModelEndpoints } from "./model-config";
import { useVerifiedEndpointIds } from "./model-session";
import ProjectProgressPanel from "./ProjectProgressPanel";

type ProjectSummary = {
  fileCount: number;
  id: string;
  indexedCount: number;
  name: string;
  rootName: string;
  updatedAt: number;
};

type ProjectFile = {
  documentId?: string;
  indexError?: string;
  modifiedAt: number;
  pageCount?: number;
  relativePath: string;
  sizeBytes: number;
  sourceKind: string;
  sourceName: string;
  warnings: string[];
};

type ProjectDetail = {
  files: ProjectFile[];
  summary: ProjectSummary;
};

type IndexedDocument = {
  markdown: string;
  pageCount?: number;
  relativePath: string;
  sourceKind: string;
  sourceName: string;
  warnings: string[];
};

type SearchHit = {
  documentId: string;
  relativePath: string;
  score: number;
  snippet: string;
  sourceName: string;
};

type ChatResult = {
  content: string;
  requestId?: string;
};

function safeError(error: unknown, fallback: string) {
  return typeof error === "string" && error.length <= 240 ? error : fallback;
}

function humanSize(bytes: number) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KiB`;
  return `${(bytes / 1024 / 1024).toFixed(1)} MiB`;
}

export default function ProjectWorkspace() {
  const [projects, setProjects] = useState<ProjectSummary[]>([]);
  const [activeProjectId, setActiveProjectId] = useState("");
  const [detail, setDetail] = useState<ProjectDetail | null>(null);
  const [document, setDocument] = useState<IndexedDocument | null>(null);
  const [query, setQuery] = useState("");
  const [hits, setHits] = useState<SearchHit[]>([]);
  const [analysis, setAnalysis] = useState("");
  const [requestId, setRequestId] = useState("");
  const [notice, setNotice] = useState("");
  const [busy, setBusy] = useState(false);
  const [confirmRemove, setConfirmRemove] = useState(false);
  const verifiedEndpointIds = useVerifiedEndpointIds();
  const endpoint = useMemo(
    () => loadModelEndpoints().find((item) => item.enabled && verifiedEndpointIds.includes(item.id)),
    [verifiedEndpointIds],
  );

  useEffect(() => {
    let active = true;
    invoke<ProjectSummary[]>("list_projects")
      .then((items) => {
        if (!active) return;
        setProjects(items);
        setActiveProjectId(items[0]?.id ?? "");
      })
      .catch(() => {
        if (active) setNotice("项目数据仅在桌面应用中可用。");
      });
    return () => {
      active = false;
    };
  }, []);

  useEffect(() => {
    if (!activeProjectId) {
      setDetail(null);
      return;
    }
    let active = true;
    invoke<ProjectDetail>("get_project", { projectId: activeProjectId })
      .then((result) => {
        if (active) setDetail(result);
      })
      .catch((error) => {
        if (active) setNotice(safeError(error, "无法读取项目索引。"));
      });
    return () => {
      active = false;
    };
  }, [activeProjectId]);

  async function addProject() {
    setNotice("");
    try {
      const path = await open({ directory: true, multiple: false });
      if (typeof path !== "string") return;
      setBusy(true);
      const summary = await invoke<ProjectSummary>("add_project", { path });
      const indexed = await invoke<ProjectDetail>("index_project", { projectId: summary.id });
      setProjects((current) => [indexed.summary, ...current.filter((item) => item.id !== summary.id)]);
      setActiveProjectId(summary.id);
      setDetail(indexed);
      setDocument(null);
      setHits([]);
      setNotice(`已授权并索引 ${indexed.summary.indexedCount} 个文档；不会修改项目源文件。`);
    } catch (error) {
      setNotice(safeError(error, "无法添加项目目录。"));
    } finally {
      setBusy(false);
    }
  }

  async function refreshProject() {
    if (!activeProjectId) return;
    setBusy(true);
    setNotice("");
    try {
      const indexed = await invoke<ProjectDetail>("index_project", { projectId: activeProjectId });
      setDetail(indexed);
      setProjects((current) => current.map((item) => item.id === activeProjectId ? indexed.summary : item));
      setDocument(null);
      setHits([]);
      setNotice(`索引已更新：${indexed.summary.indexedCount}/${indexed.summary.fileCount} 个文档可检索。`);
    } catch (error) {
      setNotice(safeError(error, "项目索引更新失败。"));
    } finally {
      setBusy(false);
    }
  }

  async function removeProject() {
    if (!activeProjectId) return;
    try {
      await invoke("remove_project", { projectId: activeProjectId });
      const remaining = projects.filter((project) => project.id !== activeProjectId);
      setProjects(remaining);
      setActiveProjectId(remaining[0]?.id ?? "");
      setDetail(null);
      setDocument(null);
      setHits([]);
      setConfirmRemove(false);
      setNotice("已移除本地项目记录与索引，源目录和文件未被删除。 ");
    } catch (error) {
      setNotice(safeError(error, "无法移除项目记录。"));
    }
  }

  async function openDocument(file: ProjectFile) {
    if (!activeProjectId || !file.documentId) return;
    setNotice("");
    try {
      setDocument(
        await invoke<IndexedDocument>("get_indexed_document", {
          projectId: activeProjectId,
          relativePath: file.relativePath,
        }),
      );
    } catch (error) {
      setNotice(safeError(error, "无法读取本地文档预览。"));
    }
  }

  async function search(event: FormEvent) {
    event.preventDefault();
    if (!activeProjectId || !query.trim()) return;
    setBusy(true);
    setAnalysis("");
    setRequestId("");
    try {
      const results = await invoke<SearchHit[]>("search_project", {
        projectId: activeProjectId,
        query: query.trim(),
      });
      setHits(results);
      setNotice(results.length ? `找到 ${results.length} 条本地来源。` : "没有找到匹配的本地来源。");
    } catch (error) {
      setNotice(safeError(error, "项目搜索失败。"));
    } finally {
      setBusy(false);
    }
  }

  async function analyzeHits() {
    if (!endpoint || !query.trim() || !hits.length) return;
    setBusy(true);
    setAnalysis("");
    setRequestId("");
    try {
      const sources = hits
        .slice(0, 8)
        .map((hit, index) => `[来源 ${index + 1}] ${hit.relativePath}\n${hit.snippet}`)
        .join("\n\n");
      const result = await invoke<ChatResult>("send_chat_completion", {
        endpointId: endpoint.id,
        baseUrl: endpoint.baseUrl,
        model: endpoint.model,
        messages: [
          {
            role: "system",
            content: "只根据提供的本地项目检索片段回答。每个结论标注[来源 N]；证据不足时明确说明，不生成或修改任何项目文件。",
            attachments: [],
          },
          { role: "user", content: `问题：${query.trim()}\n\n${sources}`, attachments: [] },
        ],
      });
      setAnalysis(result.content);
      setRequestId(result.requestId ?? "");
    } catch (error) {
      setNotice(safeError(error, "AI 分析失败，本地搜索结果仍然可用。"));
    } finally {
      setBusy(false);
    }
  }

  async function semanticSearch() {
    if (!endpoint || !activeProjectId || !query.trim()) return;
    setBusy(true);
    setAnalysis("");
    setRequestId("");
    try {
      const candidates = await invoke<SearchHit[]>("project_semantic_candidates", { projectId: activeProjectId });
      if (!candidates.length) {
        setNotice("当前项目没有可供语义检索的文档。 ");
        return;
      }
      const sources = candidates
        .map((hit, index) => `[来源 ${index + 1}] ${hit.relativePath}\n${hit.snippet}`)
        .join("\n\n")
        .slice(0, 28_000);
      const result = await invoke<ChatResult>("send_chat_completion", {
        endpointId: endpoint.id,
        baseUrl: endpoint.baseUrl,
        model: endpoint.model,
        messages: [
          {
            role: "system",
            content: "你负责对本地项目候选片段做语义检索并回答。只使用提供的内容，先判断与问题语义相关的来源，再回答；每个结论标注[来源 N]，证据不足时明确说明。不得生成或修改项目文件。",
            attachments: [],
          },
          { role: "user", content: `问题：${query.trim()}\n\n${sources}`, attachments: [] },
        ],
      });
      setHits(candidates);
      setAnalysis(result.content);
      setRequestId(result.requestId ?? "");
      setNotice(`已对 ${candidates.length} 个本地候选文档完成 AI 语义检索；结果须按来源复核。`);
    } catch (error) {
      setNotice(safeError(error, "AI 语义检索失败，未修改任何项目文件。"));
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="project-workspace" aria-labelledby="project-workspace-title">
      <header className="workspace-action-header">
        <div>
          <p className="eyebrow">READ-ONLY PROJECT INDEX</p>
          <h2 id="project-workspace-title">本地项目与检索</h2>
          <p>目录授权、结构化预览和全文索引均在本机完成；首版不会修改任何项目源文件。</p>
        </div>
        <button className="primary-button" type="button" onClick={addProject} disabled={busy}>
          {busy ? "处理中…" : "添加本地项目"}
        </button>
      </header>

      {notice ? <p className="converter-notice" role="status">{notice}</p> : null}

      <details className="progress-disclosure" open>
        <summary>项目进度、里程碑、重点与风险</summary>
        <ProjectProgressPanel projects={projects} />
      </details>

      <div className="project-layout">
        <aside className="project-sidebar" aria-label="本地项目">
          {projects.map((project) => (
            <button
              className={project.id === activeProjectId ? "active" : ""}
              key={project.id}
              type="button"
              onClick={() => {
                setActiveProjectId(project.id);
                setDocument(null);
                setHits([]);
                setConfirmRemove(false);
              }}
            >
              <span>◇</span>
              <strong>{project.name}</strong>
              <small>{project.indexedCount}/{project.fileCount} 已索引 · {project.rootName}</small>
            </button>
          ))}
          {!projects.length ? <div className="conversion-empty">尚未授权项目目录</div> : null}
        </aside>

        <main className="project-main">
          {detail ? (
            <>
              <div className="project-controls">
                <form onSubmit={search}>
                  <input aria-label="搜索项目文档" value={query} onChange={(event) => setQuery(event.target.value)} maxLength={500} placeholder="搜索文件内容、参数或单位…" />
                  <button className="secondary-button" type="submit" disabled={busy || !query.trim()}>本地搜索</button>
                </form>
                <button className="secondary-button" type="button" disabled={busy || !endpoint || !query.trim()} onClick={semanticSearch}>{endpoint ? "AI 语义检索" : "连接模型后语义检索"}</button>
                <button className="text-button" type="button" disabled={busy} onClick={refreshProject}>更新索引</button>
                {confirmRemove ? (
                  <button className="text-button danger" type="button" onClick={removeProject}>确认只移除记录</button>
                ) : (
                  <button className="text-button danger" type="button" onClick={() => setConfirmRemove(true)}>移除项目</button>
                )}
              </div>

              {hits.length ? (
                <section className="search-results" aria-label="项目搜索结果">
                  <header>
                    <strong>本地来源</strong>
                    <button className="secondary-button" type="button" disabled={!endpoint || busy} onClick={analyzeHits}>
                      {endpoint ? "AI 分析检索结果" : "连接模型后可分析"}
                    </button>
                  </header>
                  {hits.map((hit) => (
                    <button key={`${hit.documentId}-${hit.relativePath}`} type="button" onClick={() => {
                      const file = detail.files.find((item) => item.relativePath === hit.relativePath);
                      if (file) void openDocument(file);
                    }}>
                      <strong>{hit.sourceName}</strong>
                      <span>{hit.relativePath}</span>
                      <p>{hit.snippet}</p>
                    </button>
                  ))}
                </section>
              ) : null}

              {analysis ? (
                <section className="project-analysis" aria-label="项目 AI 分析">
                  <MarkdownMessage text={analysis} />
                  {requestId ? <small>请求 ID：{requestId}</small> : null}
                </section>
              ) : null}

              <div className="project-content-grid">
                <aside className="project-file-list" aria-label="项目文件">
                  {detail.files.map((file) => (
                    <button key={file.relativePath} type="button" disabled={!file.documentId} onClick={() => openDocument(file)}>
                      <span>{file.sourceKind}</span>
                      <strong>{file.relativePath}</strong>
                      <small>{humanSize(file.sizeBytes)}{file.pageCount ? ` · ${file.pageCount} 页` : ""}</small>
                      {file.indexError ? <em>{file.indexError}</em> : null}
                    </button>
                  ))}
                </aside>
                <section className="project-document-preview" aria-label="项目文档预览">
                  {document ? (
                    <>
                      {document.warnings.length ? <ul>{document.warnings.map((warning) => <li key={warning}>{warning}</li>)}</ul> : null}
                      <MarkdownMessage text={document.markdown} />
                    </>
                  ) : (
                    <div className="conversion-placeholder"><span>▤</span><strong>选择已索引文件</strong><p>预览来自 SQLite 缓存，不会在点击时重新读取源目录。</p></div>
                  )}
                </section>
              </div>
            </>
          ) : (
            <div className="conversion-placeholder"><span>◇</span><strong>添加一个本地项目</strong><p>系统文件夹对话框是唯一的目录授权入口。</p></div>
          )}
        </main>
      </div>
    </section>
  );
}
