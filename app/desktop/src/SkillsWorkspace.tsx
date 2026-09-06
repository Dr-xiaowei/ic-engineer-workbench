import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import MarkdownMessage from "./MarkdownMessage";

export type PromptSkill = {
  description: string;
  enabled: boolean;
  hasScripts: boolean;
  id: string;
  instructions: string;
  name: string;
  permissions: string[];
  restrictedPermissions: boolean;
  sourceName: string;
  sourceType: string;
  version: string;
};

function safeErrorMessage(error: unknown, fallback: string) {
  return typeof error === "string" && error.length <= 240 ? error : fallback;
}

function exportName(skill: PromptSkill) {
  const safeName = skill.name.replace(/[\\/:*?"<>|]/g, "-").slice(0, 80) || "local-skill";
  return `${safeName}.md`;
}

function exportContent(skill: PromptSkill) {
  const name = skill.name.replace(/[\r\n]/g, " ");
  const description = skill.description.replace(/[\r\n]/g, " ");
  const version = (skill.version || "unspecified").replace(/[\r\n]/g, " ");
  return `---\nname: ${name}\ndescription: ${description}\nversion: ${version}\npermissions: [${(skill.permissions || ["prompt"]).join(", ")}]\n---\n\n${skill.instructions.trim()}\n`;
}

export default function SkillsWorkspace() {
  const [skills, setSkills] = useState<PromptSkill[]>([]);
  const [selectedId, setSelectedId] = useState("");
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState("");
  const [removePendingId, setRemovePendingId] = useState("");
  const [urlDraft, setUrlDraft] = useState("");
  const [query, setQuery] = useState("");
  const selected = skills.find((skill) => skill.id === selectedId) ?? skills[0];
  const filteredSkills = useMemo(() => skills.filter((skill) => `${skill.name}\n${skill.description}\n${skill.version}`.toLowerCase().includes(query.toLowerCase())), [skills, query]);

  async function refresh(preferredId?: string) {
    try {
      const records = await invoke<PromptSkill[]>("list_skills");
      setSkills(records);
      setSelectedId(preferredId && records.some((skill) => skill.id === preferredId) ? preferredId : records[0]?.id ?? "");
    } catch (error) {
      setNotice(safeErrorMessage(error, "无法读取本地 Skills。"));
    }
  }

  useEffect(() => {
    void refresh();
  }, []);

  async function importSelection(directory: boolean) {
    setNotice("");
    try {
      const path = await open({
        directory,
        multiple: false,
        ...(directory ? {} : { filters: [{ name: "Skill 说明", extensions: ["md"] }] }),
      });
      if (typeof path !== "string") return;
      setBusy(true);
      const imported = await invoke<PromptSkill>("import_skill", { path });
      await refresh(imported.id);
      setNotice(
        imported.hasScripts
          ? "Skill 已隔离导入但保持停用：检测到脚本或可执行内容，首版不会执行。"
          : "说明型 Skill 已导入并保持停用，请查看内容后手动启用。",
      );
    } catch (error) {
      setNotice(safeErrorMessage(error, "Skill 导入失败，未执行任何内容。"));
    } finally {
      setBusy(false);
    }
  }

  async function toggleSkill(skill: PromptSkill) {
    setBusy(true);
    try {
      const updated = await invoke<PromptSkill>("set_skill_enabled", {
        skillId: skill.id,
        enabled: !skill.enabled,
      });
      setSkills((current) => current.map((item) => (item.id === updated.id ? updated : item)));
      setNotice(updated.enabled ? "Skill 已启用，可在智能对话中显式选择。" : "Skill 已停用。");
    } catch (error) {
      setNotice(safeErrorMessage(error, "无法更新 Skill 状态。"));
    } finally {
      setBusy(false);
    }
  }

  async function importIntranetUrl() {
    if (!urlDraft.trim()) return;
    setBusy(true);
    setNotice("");
    try {
      const imported = await invoke<PromptSkill>("import_skill_url", { sourceUrl: urlDraft.trim() });
      setUrlDraft("");
      await refresh(imported.id);
      setNotice("内网说明型 Skill 已下载并保持停用，请核对来源、权限和内容后手动启用。");
    } catch (error) {
      setNotice(safeErrorMessage(error, "内网 Skill 导入失败，未执行任何内容。"));
    } finally {
      setBusy(false);
    }
  }

  async function exportSkill(skill: PromptSkill) {
    try {
      const name = exportName(skill);
      const path = await save({ defaultPath: name, filters: [{ name: "Skill Markdown", extensions: ["md"] }] });
      if (!path) return;
      await invoke("export_markdown", { path, content: exportContent(skill) });
      setNotice(`已导出 ${name}。`);
    } catch (error) {
      setNotice(safeErrorMessage(error, "Skill 导出失败。"));
    }
  }

  async function removeSkill(skill: PromptSkill) {
    if (removePendingId !== skill.id) {
      setRemovePendingId(skill.id);
      setNotice("再次点击确认只移除本地 Skill 记录；来源文件不会被删除。");
      return;
    }
    setBusy(true);
    try {
      await invoke("remove_skill", { skillId: skill.id });
      setRemovePendingId("");
      await refresh();
      setNotice("Skill 记录已移除，来源文件未被删除。");
    } catch (error) {
      setNotice(safeErrorMessage(error, "无法移除 Skill 记录。"));
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="skills-workspace" aria-labelledby="skills-title">
      <header className="workspace-action-header">
        <div>
          <p className="eyebrow">CONTROLLED PROMPT LIBRARY</p>
          <h2 id="skills-title">Skills 管理</h2>
          <p>导入仅解析 UTF-8 SKILL.md；新内容默认停用，脚本类内容首版只展示、不执行。</p>
        </div>
        <div className="skills-import-actions">
          <button className="secondary-button" type="button" disabled={busy} onClick={() => importSelection(false)}>导入 SKILL.md</button>
          <button className="primary-button" type="button" disabled={busy} onClick={() => importSelection(true)}>导入 Skill 目录</button>
        </div>
      </header>

      {notice ? <p className="converter-notice" role="status">{notice}</p> : null}

      <div className="skill-url-import">
        <label>
          内网 SKILL.md 地址
          <input type="url" value={urlDraft} onChange={(event) => setUrlDraft(event.target.value)} placeholder="https://skills.internal/example/SKILL.md" />
        </label>
        <button className="secondary-button" type="button" disabled={busy || !urlDraft.trim()} onClick={importIntranetUrl}>从内网导入</button>
        <small>仅允许本机或内网地址，禁止重定向、查询参数和公网解析；网络总开关默认关闭。</small>
      </div>

      <div className="skills-layout">
        <aside className="skill-list" aria-label="本地 Skills">
          <input className="skill-search" aria-label="搜索已上传 Skills" placeholder="搜索 Skills…" value={query} onChange={(event) => setQuery(event.target.value)} />
          <div className="skill-list-heading"><strong>已上传 Skills</strong><span>{filteredSkills.length}</span></div>
          {filteredSkills.map((skill) => (
            <button className={selected?.id === skill.id ? "active" : ""} key={skill.id} type="button" onClick={() => setSelectedId(skill.id)}>
              <span className={skill.hasScripts || skill.restrictedPermissions ? "risk" : skill.enabled ? "enabled" : ""}>{skill.hasScripts ? "含脚本 · 隔离" : skill.restrictedPermissions ? "权限受限 · 隔离" : skill.enabled ? "已启用" : "已停用"}</span>
              <strong>{skill.name}</strong>
              <small>作用概述：{skill.description || "该 Skill 未提供描述，请打开核对正文。"}</small>
            </button>
          ))}
          {!skills.length ? <div className="conversion-empty">尚未导入 Skill</div> : null}
        </aside>

        <main className="skill-preview">
          {selected ? (
            <>
              <header>
                <div>
                  <span>PROMPT-ONLY PREVIEW</span>
                  <h3>{selected.name}</h3>
                  <p>{selected.description}</p>
                </div>
                <div>
                  <button className="secondary-button" type="button" onClick={() => exportSkill(selected)}>导出</button>
                  <button className="secondary-button" type="button" disabled={busy || selected.hasScripts || selected.restrictedPermissions} onClick={() => toggleSkill(selected)}>{selected.enabled ? "停用" : "启用"}</button>
                  <button className="text-button danger" type="button" disabled={busy} onClick={() => removeSkill(selected)}>{removePendingId === selected.id ? "确认只移除记录" : "移除"}</button>
                </div>
              </header>
              <div className={`skill-security ${selected.hasScripts || selected.restrictedPermissions ? "risk" : ""}`}>
                <strong>{selected.hasScripts ? "检测到可执行内容" : selected.restrictedPermissions ? "权限清单超出首版范围" : "说明型 Skill"}</strong>
                <p>{selected.hasScripts ? "该 Skill 不能启用；应用不会运行其中的脚本、命令或工具。" : selected.restrictedPermissions ? "该 Skill 声明了 prompt 以外权限，已保持隔离且不能启用。" : "启用只允许把下方说明作为受限提示词注入一次对话请求，不授予文件、网络或工具权限。"}</p>
              </div>
              <div className="skill-manifest"><span>版本：{selected.version || "unspecified"}</span><span>来源类型：{selected.sourceType || "local-file"}</span><span>权限：{(selected.permissions || ["prompt"]).join("、")}</span></div>
              <MarkdownMessage text={selected.instructions} />
              <small>来源：{selected.sourceName} · 本地记录标识：{selected.id}</small>
            </>
          ) : (
            <div className="conversion-placeholder">
              <span>✦</span>
              <strong>受控本地能力说明</strong>
              <p>选择本地 SKILL.md 或 Skill 目录开始；导入不会执行文件内容。</p>
            </div>
          )}
        </main>
      </div>
    </section>
  );
}
