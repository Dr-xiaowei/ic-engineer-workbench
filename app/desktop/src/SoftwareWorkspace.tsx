import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

type Launcher = { id: string; name: string; targetName: string; targetKind: string; updatedAt: number };
function safeError(error: unknown) { return typeof error === "string" && error.length < 240 ? error : "操作失败，请检查所选软件。"; }

export default function SoftwareWorkspace() {
  const [items, setItems] = useState<Launcher[]>([]); const [path, setPath] = useState(""); const [name, setName] = useState(""); const [notice, setNotice] = useState(""); const [query, setQuery] = useState(""); const [removeId, setRemoveId] = useState("");
  const refresh = () => invoke<Launcher[]>("list_software_launchers").then(setItems).catch(() => setNotice("软件快捷入口仅在桌面应用中可用。"));
  useEffect(() => { void refresh(); }, []);
  async function choose() { const selected = await open({ directory: false, multiple: false }); if (typeof selected !== "string") return; setPath(selected); setName(selected.split(/[\\/]/).pop()?.replace(/\.app$/i, "") ?? "常用软件"); }
  async function add() { if (!path || !name.trim()) return; try { await invoke("add_software_launcher", { path, name: name.trim() }); setPath(""); setName(""); setNotice("快捷入口已保存；启动时仍会由后端重新校验软件路径和类型。"); await refresh(); } catch (error) { setNotice(safeError(error)); } }
  async function launch(id: string) { try { await invoke("launch_software", { launcherId: id }); setNotice("已请求系统启动所选软件。"); } catch (error) { setNotice(safeError(error)); } }
  async function remove(id: string) { if (removeId !== id) { setRemoveId(id); setNotice("再次点击移除；只删除快捷入口，不会卸载软件。"); return; } await invoke("remove_software_launcher", { launcherId: id }); setRemoveId(""); await refresh(); }
  const filtered = items.filter((item) => item.name.toLowerCase().includes(query.toLowerCase()));
  return <section className="tool-workspace" aria-labelledby="software-title"><header className="workspace-action-header"><div><p className="eyebrow">LOCAL APP LAUNCHER</p><h2 id="software-title">常用软件</h2><p>只启动你明确选择的应用包或可执行文件；不接受命令、参数、URL 或 Shell。</p></div><button className="primary-button" type="button" onClick={choose}>选择软件</button></header>{notice ? <p className="converter-notice" role="status">{notice}</p> : null}{path ? <section className="launcher-add"><div><strong>已选择</strong><span>{path.split(/[\\/]/).pop()}</span></div><label>快捷名称<input value={name} maxLength={120} onChange={(e) => setName(e.target.value)}/></label><button className="primary-button" type="button" onClick={add}>添加快捷入口</button></section> : null}<input className="tool-search" aria-label="搜索常用软件" placeholder="搜索快捷入口…" value={query} onChange={(e) => setQuery(e.target.value)}/><div className="launcher-grid">{filtered.map((item) => <article key={item.id}><button className="launcher-open" type="button" onClick={() => launch(item.id)}><span>{item.name.slice(0,2).toUpperCase()}</span><strong>{item.name}</strong><small>{item.targetName} · {item.targetKind === "application" ? "应用" : "可执行文件"}</small></button><button className="launcher-remove" type="button" onClick={() => remove(item.id)}>{removeId === item.id ? "确认移除" : "移除"}</button></article>)}{!filtered.length ? <div className="conversion-placeholder"><span>⌘</span><strong>尚无软件快捷入口</strong><p>点击“选择软件”添加。</p></div> : null}</div></section>;
}
