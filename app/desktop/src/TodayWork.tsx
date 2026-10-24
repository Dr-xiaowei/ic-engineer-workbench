import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useWorkspaceState } from "./workspace-state";
import { emptyNote, emptyReader, navigateTo } from "./review-flow";
export default function TodayWork() {
  const [note] = useWorkspaceState("note-draft", emptyNote);
  const [reader] = useWorkspaceState("reader", emptyReader);
  const [projects, setProjects] = useState<Array<{ id: string; name: string }>>([]);
  const [tasks, setTasks] = useState<Array<{ id: string; title: string; startAt: number; status: string; projectName?: string }>>([]);
  useEffect(() => {
    invoke<Array<{ id: string; name: string }>>("list_projects").then(setProjects).catch(() => undefined);
    const end = new Date(); end.setHours(24, 0, 0, 0);
    invoke<typeof tasks>("list_calendar_tasks", { rangeStart: Math.floor(end.getTime() / 1000) - 30 * 86400, rangeEnd: Math.floor(end.getTime() / 1000) }).then(values => setTasks(values.filter(t => t.status !== "done"))).catch(() => undefined);
  }, []);
  async function complete(task: (typeof tasks)[number]) {
    try { await invoke("save_calendar_task", { input: { ...task, status: "done" } }); setTasks(current => current.filter(item => item.id !== task.id)); }
    catch { navigateTo("calendar"); }
  }
  return <section className="today-work" aria-label="继续工作与今日待办"><article><h2>继续工作</h2>{note.title || note.body ? <button onClick={() => navigateTo("knowledge")}><strong>继续笔记草稿</strong><small>{note.title || "未命名笔记"}</small></button> : null}{reader.sourceId ? <button onClick={() => navigateTo("datasheet")}><strong>继续阅读资料</strong><small>{reader.sourceName || "已选资料"} · 第 {reader.page} 页 · {reader.highlights.length} 条原文证据</small></button> : null}<button onClick={() => navigateTo("skills")}><strong>开始一份工作记录</strong><small>设计评审 / Datasheet 核查 / 故障排查 / 周报</small></button>{projects.slice(0, 3).map(p => <button key={p.id} onClick={() => { window.dispatchEvent(new CustomEvent("workbench:open-project", { detail: p.id })); navigateTo("projects"); }}><strong>{p.name}</strong><small>继续项目检索和进度记录</small></button>)}</article><article><h2>今日与近期逾期待办</h2>{tasks.length ? tasks.slice(0, 6).map(task => <div key={task.id} className="today-task-row"><button onClick={() => navigateTo("calendar")}><strong>{task.title}</strong><small>{new Date(task.startAt * 1000).toLocaleDateString("zh-CN")} · {task.projectName || "个人待办"}</small></button><button className="text-button" aria-label={`标记完成：${task.title}`} onClick={() => void complete(task)}>完成</button></div>) : <p>今日及最近 30 天没有未完成事项。</p>}<button className="secondary-button" onClick={() => navigateTo("calendar")}>查看日历与全部待办</button></article></section>;
}
