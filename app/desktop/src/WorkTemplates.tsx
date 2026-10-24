import { useState } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { useWorkspaceState, useDraftField, updateWorkspaceState } from "./workspace-state";
import { builtInTemplates, fillTemplate, validateTemplate, type WorkTemplate } from "./work-templates";
import { appendToNote, navigateTo } from "./review-flow";
import { loadModelEndpoints } from "./model-config";
import MarkdownMessage from "./MarkdownMessage";
import DraftStatus from "./DraftStatus";
import SkillsWorkspace from "./SkillsWorkspace";

type TemplateState = { custom: WorkTemplate[]; activeId: string; values: Record<string, Record<string, string>>; editing: WorkTemplate | null; fieldsText: string };
const initial: TemplateState = { custom: [], activeId: builtInTemplates[0].id, values: {}, editing: null, fieldsText: "" };
export default function WorkTemplates() {
  const [state, setState, status, retry] = useWorkspaceState("templates", initial);
  const [notice, setNotice] = useState("");
  const [editing, setEditing] = useDraftField("templates", initial, "editing");
  const [fieldsText, setFieldsText] = useDraftField("templates", initial, "fieldsText");
  const templates = [...builtInTemplates, ...state.custom];
  const active = templates.find(t => t.id === state.activeId) ?? templates[0];
  const values = state.values[active.id] ?? {};
  const content = fillTemplate(active, values);
  function startCopy() { const copy = { ...structuredClone(active), id: `custom-${crypto.randomUUID()}`, name: `${active.name} · 我的模板` }; setEditing(copy); setFieldsText(copy.fields.map(f => `${f.key}|${f.label}|${f.placeholder}`).join("\n")); }
  function saveCustom() {
    if (!editing) return;
    try {
      const fields = fieldsText.split("\n").filter(s => s.trim()).map(row => { const [key, label, ...rest] = row.split("|"); return { key: key?.trim(), label: label?.trim(), placeholder: rest.join("|").trim() }; });
      const valid = validateTemplate({ ...editing, fields });
      setState(current => ({ ...current, custom: [...current.custom.filter(t => t.id !== valid.id), valid], activeId: valid.id })); setEditing(null); setNotice("已保存个人模板。");
    } catch (e) { setNotice(e instanceof Error ? e.message : "模板无效。"); }
  }
  async function importTemplate(file?: File) {
    if (!file) return;
    try { if (file.size > 100000 || !file.name.toLowerCase().endsWith(".json")) throw new Error("请选择不超过 100 KB 的 JSON 模板。"); const valid = validateTemplate(JSON.parse(await file.text())); valid.id = `custom-${crypto.randomUUID()}`; valid.defaultEndpointId = ""; setState(current => ({ ...current, custom: [...current.custom, valid], activeId: valid.id })); setNotice("已导入提示词模板；可预览后使用，默认模型请在本机重新选择。"); } catch (e) { setNotice(e instanceof Error ? e.message : "导入失败。"); }
  }
  async function exportTemplate() { try { const path = await save({ defaultPath: `${active.name}.json`, filters: [{ name: "JSON 工作模板", extensions: ["json"] }] }); if (!path) return; await invoke("export_work_template", { path, content: JSON.stringify({ ...active, defaultEndpointId: "" }, null, 2) }); setNotice("已导出模板定义，不包含填写内容、模型地址或密钥。"); } catch (e) { setNotice(typeof e === "string" ? e : "导出失败。"); } }
  async function toNote() { try { await appendToNote(`${active.name} · 工作记录`, content); } catch { setNotice("笔记草稿未保存，请重试。"); } }
  async function toChat() {
    try {
      // Only prefill; the user reviews the prompt and attachment boundary before sending.
      await updateWorkspaceState<Record<string, unknown>>("chat-draft", { text: "", attachments: [], documents: [], endpointId: "", skillId: "", conversationId: "" }, current => ({ ...current, text: [current.text, fillTemplate(active, values, active.prompt), content].filter(Boolean).join("\n\n"), endpointId: active.defaultEndpointId || current.endpointId }));
      navigateTo("chat");
    } catch { setNotice("对话草稿未保存，请重试。"); }
  }
  return <section className="templates-workspace" aria-label="工作模板"><header className="workspace-action-header"><div><h2>工作模板</h2><p>选一个场景，填入现有事实，预览后整理成笔记或交给助手。</p></div><button className="secondary-button" onClick={startCopy}>复制并定制</button></header><DraftStatus state={status} retry={retry}/>{notice ? <p role="status" className="converter-notice">{notice}</p> : null}<div className="template-picker">{templates.map(t => <button className={t.id === active.id ? "active" : ""} key={t.id} onClick={() => setState({ ...state, activeId: t.id })}><strong>{t.name}</strong><small>{t.description}</small></button>)}</div><div className="template-layout"><form onSubmit={e => { e.preventDefault(); void toNote(); }}><h3>{active.name}</h3>{active.fields.map(f => <label key={f.key}>{f.label}<textarea maxLength={4000} rows={3} placeholder={f.placeholder} value={values[f.key] ?? ""} onChange={e => setState(current => ({ ...current, values: { ...current.values, [active.id]: { ...values, [f.key]: e.target.value } } }))}/></label>)}<div className="template-actions"><button type="submit" className="primary-button">加入笔记草稿</button><button type="button" className="secondary-button" onClick={toChat}>交给助手继续</button></div><small>空缺保留“待补充”；使用助手仍需由你点击发送。</small></form><article className="template-preview"><h3>生成内容预览</h3><MarkdownMessage text={content}/></article></div>
    <details className="advanced-options"><summary>个性化模板与导入 / 导出</summary><p>模板只提供字段、正文和提示词，不运行代码。</p><div className="template-actions"><button onClick={startCopy}>复制当前模板</button>{active.id.startsWith("custom-") ? <button onClick={() => { setEditing(structuredClone(active)); setFieldsText(active.fields.map(f => `${f.key}|${f.label}|${f.placeholder}`).join("\n")); }}>编辑个人模板</button> : null}<button onClick={exportTemplate}>导出 JSON</button><label className="template-import">导入 JSON<input type="file" accept=".json,application/json" onChange={e => { void importTemplate(e.target.files?.[0]); e.target.value = ""; }}/></label></div></details>
    {editing ? <section className="template-custom-editor" aria-label="定制模板"><h3>定制模板</h3><label>模板名称<input maxLength={80} value={editing.name} onChange={e => setEditing({ ...editing, name: e.target.value })}/></label><label>说明<input maxLength={400} value={editing.description} onChange={e => setEditing({ ...editing, description: e.target.value })}/></label><label>填写字段（每行：英文标识 | 中文名称 | 填写提示）<textarea rows={4} value={fieldsText} onChange={e => setFieldsText(e.target.value)}/></label><label>笔记正文（使用 {'{{字段标识}}'}）<textarea rows={10} value={editing.body} onChange={e => setEditing({ ...editing, body: e.target.value })}/></label><label>助手提示词<textarea rows={5} value={editing.prompt} onChange={e => setEditing({ ...editing, prompt: e.target.value })}/></label><label>默认模型<select value={editing.defaultEndpointId} onChange={e => setEditing({ ...editing, defaultEndpointId: e.target.value })}><option value="">跟随当前模型</option>{loadModelEndpoints().map(m => <option key={m.id} value={m.id}>{m.name} · {m.model}</option>)}</select></label><div><button className="primary-button" onClick={saveCustom}>保存个人模板</button><button onClick={() => setEditing(null)}>取消编辑</button></div></section> : null}
    <details className="advanced-options"><summary>进阶：导入与管理 Skills</summary><SkillsWorkspace/></details>
  </section>;
}
