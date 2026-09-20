import { useEffect, useMemo, useState, type ChangeEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
import ModelWorkspace from "./ModelWorkspace";
import FloatingAssistant, { type AssistantPageContext } from "./FloatingAssistant";
import ConverterWorkspace from "./ConverterWorkspace";
import ProjectWorkspace from "./ProjectWorkspace";
import DatasheetWorkspace from "./DatasheetWorkspace";
import SkillsWorkspace from "./SkillsWorkspace";
import MailWorkspace from "./MailWorkspace";
import CalendarWorkspace from "./CalendarWorkspace";
import CalculatorWorkspace from "./CalculatorWorkspace";
import SoftwareWorkspace from "./SoftwareWorkspace";
import KnowledgeWorkspace from "./KnowledgeWorkspace";
import { loadModelEndpoints, modelEndpointsStorageKey, saveModelEndpoints, type ModelEndpoint } from "./model-config";
import {
  defaultPreferences,
  loadPreferences,
  preferencesStorageKey,
  savePreferences,
  type Preferences,
  type Theme,
} from "./preferences";

type RuntimeStatus = {
  appVersion: string;
  dataLocation: string;
  demoMode: boolean;
  networkMode: string;
};

const embeddedDemoEndpoint: ModelEndpoint = {
  id: "embedded-demo-model",
  name: "内置 Demo 模型",
  baseUrl: "http://127.0.0.1:18080/v1",
  model: "demo-analog-assistant",
  enabled: true,
  status: "unchecked",
  statusDetail: "启用内网开关后执行连接检测；此端点由 Demo 应用内置模拟。",
  checkedAt: "",
};
type DashboardSummary = {
  projectCount: number; conversationCount: number; skillCount: number; mailAccountCount: number;
  cachedMailCount: number; recentProjects: string[]; recentDocuments: string[]; recentConversations: string[];
};
type AuditEvent = { action: string; outcome: string; createdAt: number };

type NavigationId =
  | "dashboard"
  | "chat"
  | "projects"
  | "datasheet"
  | "knowledge"
  | "skills"
  | "converter"
  | "mail"
  | "calendar"
  | "calculator"
  | "software";

type ModuleDefinition = {
  description: string;
  eyebrow: string;
  features: string[];
  title: string;
};

const navigationItems: Array<{
  id: NavigationId;
  label: string;
  icon: string;
}> = [
  { id: "dashboard", label: "工作台", icon: "⌂" },
  { id: "chat", label: "智能对话", icon: "◎" },
  { id: "projects", label: "项目管理", icon: "◇" },
  { id: "datasheet", label: "Datasheet", icon: "▤" },
  { id: "knowledge", label: "知识笔记", icon: "▥" },
  { id: "skills", label: "Skills", icon: "✦" },
  { id: "converter", label: "格式转换", icon: "⇄" },
  { id: "mail", label: "邮件助手", icon: "✉" },
  { id: "calendar", label: "日历与待办", icon: "◫" },
  { id: "calculator", label: "科学计算器", icon: "∑" },
  { id: "software", label: "常用软件", icon: "⌘" },
];

const navigationGroups: Array<{ label: string; ids: NavigationId[] }> = [
  { label: "工作空间", ids: ["dashboard", "chat", "projects", "calendar", "mail"] },
  { label: "资料与知识", ids: ["datasheet", "knowledge", "skills", "converter"] },
  { label: "常用工具", ids: ["calculator", "software"] },
];

function NavigationIcon({ id }: { id: NavigationId }) {
  const paths: Record<NavigationId, string> = {
    dashboard: "M3 10 12 3l9 7M5 9v12h5v-7h4v7h5V9",
    chat: "M4 4h16v12H9l-5 4V4Z",
    projects: "M3 6h7l2 3h9v11H3V6Z",
    calendar: "M4 5h16v16H4V5Zm0 5h16M8 3v4m8-4v4M8 14h2m4 0h2m-8 3h2",
    mail: "M3 5h18v14H3V5Zm0 1 9 7 9-7",
    datasheet: "M6 3h8l4 4v14H6V3Zm8 0v5h4M9 12h6m-6 4h6",
    knowledge: "M3 4h7l2 2 2-2h7v16h-7l-2 1-2-1H3V4Zm9 2v15",
    skills: "m12 3 2.5 6.5L21 12l-6.5 2.5L12 21l-2.5-6.5L3 12l6.5-2.5L12 3Z",
    converter: "M4 7h15l-4-4m4 4-4 4M20 17H5l4 4m-4-4 4-4",
    calculator: "M5 3h14v18H5V3Zm3 4h8M8 12h2m4 0h2m-8 4h2m4 0h2",
    software: "M3 3h7v7H3V3Zm11 0h7v7h-7V3ZM3 14h7v7H3v-7Zm11 0h7v7h-7v-7Z",
  };
  return <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d={paths[id]} /></svg>;
}

const modules: Record<Exclude<NavigationId, "dashboard">, ModuleDefinition> = {
  chat: {
    title: "智能对话",
    eyebrow: "LOCAL AI CONSOLE",
    description: "面向 IC 工程场景的独立多会话入口，支持本地历史、受限附件和可配置内网模型服务。",
    features: ["会话管理", "模型配置", "工程上下文"],
  },
  projects: {
    title: "项目管理",
    eyebrow: "PROJECT CONTROL",
    description: "集中管理项目资料、任务和工作记录，为后续模块提供统一的工程上下文。",
    features: ["项目概览", "任务跟踪", "资料索引"],
  },
  datasheet: {
    title: "Datasheet",
    eyebrow: "DOCUMENT INTELLIGENCE",
    description: "解析本地器件资料，提取关键参数并保留页码与原文位置，便于工程复核。",
    features: ["PDF 解析", "参数抽取", "来源定位"],
  },
  knowledge: {
    title: "知识笔记",
    eyebrow: "LOCAL KNOWLEDGE",
    description: "以本地 Markdown 为事实源管理个人知识，支持文档导入、简单排版和多格式导出。",
    features: ["本地映射", "图文表格", "多格式导出"],
  },
  skills: {
    title: "Skills",
    eyebrow: "AUTOMATION LIBRARY",
    description: "组织可复用的本地工作流能力，按明确权限调用工具并保留执行记录。",
    features: ["能力目录", "权限边界", "运行记录"],
  },
  converter: {
    title: "格式转换",
    eyebrow: "FILE PIPELINE",
    description: "在本机完成常见工程文件的批量转换，所有输入与输出均留在内网环境。",
    features: ["批量任务", "格式预检", "结果归档"],
  },
  mail: {
    title: "邮件助手",
    eyebrow: "SECURE COMPOSER",
    description: "基于本地上下文辅助整理邮件草稿；任何对外发送动作都必须经过人工确认。",
    features: ["草稿生成", "术语检查", "人工确认"],
  },
  calendar: {
    title: "日历与待办",
    eyebrow: "PROJECT SCHEDULE",
    description: "以本地待办为可编辑事实源，日期跟随系统时钟自动更新，不读取或修改系统日历内容。",
    features: ["月视图", "项目联动", "重点提醒"],
  },
  calculator: {
    title: "科学计算器",
    eyebrow: "LOCAL MATH",
    description: "安全解析常用科学表达式和函数，计算内容不离开当前设备。",
    features: ["科学函数", "角度模式", "本地历史"],
  },
  software: {
    title: "常用软件",
    eyebrow: "APP LAUNCHER",
    description: "管理用户显式选择的软件快捷入口，并由后端复核后启动。",
    features: ["快捷添加", "本地搜索", "安全启动"],
  },
};

const quickActions: Array<{
  detail: string;
  icon: string;
  target: Exclude<NavigationId, "dashboard">;
  title: string;
}> = [
  { title: "新建对话", detail: "启动工程问答", icon: "+", target: "chat" },
  { title: "导入资料", detail: "解析本地文档", icon: "⇧", target: "datasheet" },
  { title: "管理项目", detail: "整理任务与文件", icon: "◇", target: "projects" },
];

function Avatar({ preferences }: { preferences: Preferences }) {
  if (preferences.avatarDataUrl) {
    return <img src={preferences.avatarDataUrl} alt="用户头像" />;
  }

  const initials = preferences.displayName.trim().slice(0, 2).toUpperCase() || "IC";
  return <span aria-hidden="true">{initials}</span>;
}

type SettingsPanelProps = {
  networkAccess: boolean;
  onNetworkAccessChange: (enabled: boolean) => void;
  onClose: () => void;
  onPreferencesChange: (
    value: Preferences | ((current: Preferences) => Preferences),
  ) => void;
  preferences: Preferences;
};

function SettingsPanel({
  networkAccess,
  onNetworkAccessChange,
  onClose,
  onPreferencesChange,
  preferences,
}: SettingsPanelProps) {
  const [avatarError, setAvatarError] = useState("");
  const [networkError, setNetworkError] = useState("");
  const [dataNotice, setDataNotice] = useState("");
  const [restorePath, setRestorePath] = useState("");
  const [auditEvents, setAuditEvents] = useState<AuditEvent[]>([]);

  useEffect(() => {
    invoke<AuditEvent[]>("list_audit_events").then(setAuditEvents).catch(() => setAuditEvents([]));
  }, []);

  async function toggleNetworkAccess(enabled: boolean) {
    try {
      const value = await invoke<boolean>("set_network_access", { enabled });
      onNetworkAccessChange(value);
      setNetworkError("");
    } catch {
      setNetworkError("无法更新网络访问总开关。桌面后端不可用时保持关闭。");
    }
  }

  async function backupData() {
    try {
      const path = await saveDialog({ defaultPath: "ic-workbench-backup.icwb", filters: [{ name: "工作台备份", extensions: ["icwb"] }] });
      if (!path) return;
      const uiStateJson = JSON.stringify({
        preferencesJson: window.localStorage.getItem(preferencesStorageKey) ?? "",
        modelEndpointsJson: window.localStorage.getItem(modelEndpointsStorageKey) ?? "",
        outputDirectory: window.localStorage.getItem("ic-workbench-output-directory-v1") ?? "",
      });
      await invoke("backup_local_data", { path, uiStateJson });
      setDataNotice("本地数据库已备份。模型密钥和邮箱密码仍只在系统凭据库中，不包含在备份内。");
    } catch { setDataNotice("备份失败，现有本地数据未被修改。"); }
  }

  async function chooseRestore() {
    try {
      const path = await openDialog({ multiple: false, directory: false, filters: [{ name: "工作台备份", extensions: ["icwb"] }] });
      if (typeof path !== "string") return;
      setRestorePath(path);
      setDataNotice("已选择备份。再次点击“确认恢复”将替换当前本地数据库；凭据不会恢复，网络总开关会关闭。");
    } catch { setDataNotice("无法打开备份选择器，请重试。现有本地数据未被修改。"); }
  }

  async function restoreData() {
    if (!restorePath) { await chooseRestore(); return; }
    try {
      const uiStateJson = await invoke<string>("restore_local_data", { path: restorePath, confirmed: true });
      const restored = JSON.parse(uiStateJson) as { preferencesJson?: unknown; modelEndpointsJson?: unknown; outputDirectory?: unknown };
      if (typeof restored.preferencesJson === "string") window.localStorage.setItem(preferencesStorageKey, restored.preferencesJson);
      if (typeof restored.modelEndpointsJson === "string") window.localStorage.setItem(modelEndpointsStorageKey, restored.modelEndpointsJson);
      if (typeof restored.outputDirectory === "string" && restored.outputDirectory.length <= 4096) window.localStorage.setItem("ic-workbench-output-directory-v1", restored.outputDirectory);
      onPreferencesChange(loadPreferences());
      setRestorePath(""); onNetworkAccessChange(false);
      setDataNotice("本地数据恢复完成。请重新打开相关页面；系统凭据未被更改，网络保持关闭。");
    } catch { setDataNotice("恢复失败，所选文件未通过校验或无法读取。"); }
  }

  function handleAvatarChange(event: ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0];
    if (!file) return;
    if (!file.type.startsWith("image/")) {
      setAvatarError("请选择图片文件。");
      return;
    }
    if (file.size > 2 * 1024 * 1024) {
      setAvatarError("头像文件不能超过 2 MB。");
      return;
    }

    const reader = new FileReader();
    reader.onload = () => {
      if (typeof reader.result === "string") {
        onPreferencesChange((current) => ({
          ...current,
          avatarDataUrl: reader.result as string,
        }));
        setAvatarError("");
      }
    };
    reader.onerror = () => setAvatarError("头像读取失败，请重新选择。");
    reader.readAsDataURL(file);
    event.target.value = "";
  }

  function setTheme(theme: Theme) {
    onPreferencesChange((current) => ({ ...current, theme }));
  }

  return (
    <div
      className="settings-layer"
      role="presentation"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <section
        className="settings-panel"
        role="dialog"
        aria-modal="true"
        aria-labelledby="settings-title"
      >
        <header className="settings-header">
          <div>
            <p className="eyebrow">LOCAL PREFERENCES</p>
            <h2 id="settings-title">工作台设置</h2>
          </div>
          <button className="icon-button" type="button" onClick={onClose} aria-label="关闭设置">
            ×
          </button>
        </header>

        <div className="settings-content">
          <section className="setting-group">
            <div>
              <h3>个人资料</h3>
              <p>头像与名称仅保存在当前设备。</p>
            </div>
            <div className="avatar-editor">
              <div className="avatar avatar-large">
                <Avatar preferences={preferences} />
              </div>
              <div>
                <label className="secondary-button" htmlFor="avatar-input">
                  选择本地图片
                </label>
                <input
                  id="avatar-input"
                  className="visually-hidden"
                  type="file"
                  accept="image/*"
                  onChange={handleAvatarChange}
                />
                {preferences.avatarDataUrl ? (
                  <button
                    className="text-button"
                    type="button"
                    onClick={() =>
                      onPreferencesChange((current) => ({
                        ...current,
                        avatarDataUrl: "",
                      }))
                    }
                  >
                    移除头像
                  </button>
                ) : null}
                <p className="input-hint">支持常见图片格式，最大 2 MB。</p>
                {avatarError ? <p className="input-error">{avatarError}</p> : null}
              </div>
            </div>
            <label className="field-label" htmlFor="display-name">
              显示名称
            </label>
            <input
              id="display-name"
              className="text-input"
              maxLength={24}
              value={preferences.displayName}
              onChange={(event) =>
                onPreferencesChange((current) => ({
                  ...current,
                  displayName: event.target.value,
                }))
              }
            />
          </section>

          <section className="setting-group">
            <div>
              <h3>显示主题</h3>
              <p>两套主题均为本地配置，可随时切换。</p>
            </div>
            <div className="theme-options">
              {(["dark", "light"] as const).map((theme) => (
                <label
                  className={`theme-option ${preferences.theme === theme ? "selected" : ""}`}
                  key={theme}
                >
                  <input
                    type="radio"
                    name="theme"
                    value={theme}
                    checked={preferences.theme === theme}
                    onChange={() => setTheme(theme)}
                  />
                  <span className={`theme-preview ${theme}`} aria-hidden="true">
                    <i />
                    <b />
                  </span>
                  <strong>{theme === "dark" ? "深色工程" : "日常浅色"}</strong>
                </label>
              ))}
            </div>
          </section>
          <section className="setting-group">
            <div>
              <h3>内网访问总开关</h3>
              <p>默认关闭。开启后模型和邮箱模块仍只允许本机、私有网段或内网主机，并各自要求显式操作。</p>
            </div>
            <label className="network-toggle">
              <input type="checkbox" checked={networkAccess} onChange={(event) => void toggleNetworkAccess(event.target.checked)} />
              <span>{networkAccess ? "已允许受控内网连接" : "完全离线，拒绝模型与邮箱连接"}</span>
            </label>
            {networkError ? <p className="input-error">{networkError}</p> : null}
          </section>
          <section className="setting-group">
            <div>
              <h3>本地数据备份与恢复</h3>
              <p>备份包含配置元数据、会话、项目索引、Skills 和邮件缓存，不包含操作系统凭据库中的任何密码或密钥。</p>
            </div>
            <div className="data-actions">
              <button className="secondary-button" type="button" onClick={() => void backupData()}>导出本地备份</button>
              <button className={restorePath ? "primary-button" : "secondary-button"} type="button" onClick={() => void restoreData()}>{restorePath ? "确认恢复并替换" : "选择备份恢复"}</button>
            </div>
            {dataNotice ? <p className="input-hint" role="status">{dataNotice}</p> : null}
          </section>
          <section className="setting-group">
            <div><h3>本地审计记录</h3><p>只记录动作类型、结果和时间，不记录密钥、密码、正文、路径或服务器响应内容；最多保留 2000 条。</p></div>
            <ol className="audit-list">{auditEvents.slice(0, 8).map((event) => <li key={`${event.createdAt}-${event.action}`}><code>{event.action}</code><span>{event.outcome} · {new Date(event.createdAt * 1000).toLocaleString("zh-CN")}</span></li>)}{!auditEvents.length ? <li><span>尚无审计事件</span></li> : null}</ol>
          </section>
        </div>

        <footer className="settings-footer">
          <button
            className="text-button"
            type="button"
            onClick={() => onPreferencesChange(defaultPreferences)}
          >
            恢复默认
          </button>
          <button className="primary-button" type="button" onClick={onClose}>
            完成
          </button>
        </footer>
      </section>
    </div>
  );
}

function App() {
  const [activeId, setActiveId] = useState<NavigationId>("dashboard");
  const [preferences, setPreferences] = useState<Preferences>(loadPreferences);
  const [runtime, setRuntime] = useState<RuntimeStatus | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [networkAccess, setNetworkAccess] = useState(false);
  const [dashboard, setDashboard] = useState<DashboardSummary>();

  useEffect(() => {
    document.documentElement.dataset.theme = preferences.theme;
    savePreferences(preferences);
  }, [preferences]);

  useEffect(() => {
    let active = true;
    invoke<RuntimeStatus>("get_runtime_status")
      .then((status) => {
        if (!active) return;
        setRuntime(status);
        if (status.demoMode && !window.localStorage.getItem(modelEndpointsStorageKey)) {
          saveModelEndpoints([embeddedDemoEndpoint]);
        }
        if (status.demoMode) {
          void Promise.all([invoke("seed_demo_mail"), invoke("seed_demo_workspace")])
            .then(() => invoke<DashboardSummary>("get_dashboard_summary"))
            .then((summary) => { if (active) setDashboard(summary); })
            .catch(() => undefined);
        }
      })
      .catch(() => {
        if (active) {
          setRuntime({
            appVersion: "preview",
            dataLocation: "local",
            demoMode: false,
            networkMode: "offline",
          });
        }
      });
    return () => {
      active = false;
    };
  }, []);

  useEffect(() => {
    invoke<boolean>("get_network_access")
      .then((enabled) => setNetworkAccess(enabled))
      .catch(() => setNetworkAccess(false));
  }, []);

  useEffect(() => {
    if (activeId !== "dashboard") return;
    invoke<DashboardSummary>("get_dashboard_summary").then(setDashboard).catch(() => setDashboard(undefined));
  }, [activeId]);

  useEffect(() => {
    document.documentElement.scrollTop = 0;
    document.body.scrollTop = 0;
  }, [activeId]);

  const activeItem = useMemo(
    () => navigationItems.find((item) => item.id === activeId) ?? navigationItems[0],
    [activeId],
  );
  const assistantContext = useMemo<AssistantPageContext>(() => {
    if (activeId === "dashboard") {
      return {
        id: activeId,
        label: activeItem.label,
        summary: `本页是本地工作台概览。应用核心：${runtime?.appVersion ?? "正在检查"}；数据位置：${runtime?.dataLocation ?? "正在检查"}；网络模式：${runtime?.networkMode ?? "正在检查"}。页面提供智能对话、资料导入和项目管理入口。`,
      };
    }
    const definition = modules[activeId];
    return {
      id: activeId,
      label: activeItem.label,
      summary: `${definition.description} 当前页面能力范围：${definition.features.join("、")}。`,
    };
  }, [activeId, activeItem.label, runtime]);

  function toggleTheme() {
    setPreferences((current) => ({
      ...current,
      theme: current.theme === "dark" ? "light" : "dark",
    }));
  }

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <button className="brand" type="button" onClick={() => setActiveId("dashboard")}>
          <span className="brand-mark">IC</span>
          <span>
            <strong>芯智工作台{runtime?.demoMode ? " Demo" : ""}</strong>
            <small>{runtime?.demoMode ? "独立演示空间" : "本地工程空间"}</small>
          </span>
        </button>

        <nav aria-label="主导航">
          {navigationGroups.map((group) => (
            <section className="nav-group" aria-label={group.label} key={group.label}>
              <p className="nav-group-label">{group.label}</p>
              {group.ids.map((id) => navigationItems.find((item) => item.id === id)!).map((item) => (
            <button
              className={activeId === item.id ? "active" : ""}
              aria-current={activeId === item.id ? "page" : undefined}
              key={item.id}
              type="button"
              onClick={() => setActiveId(item.id)}
            >
              <span className="nav-icon"><NavigationIcon id={item.id} /></span>
              {item.label}
            </button>
              ))}
            </section>
          ))}
        </nav>

        <div className="sidebar-footer">
          <div className="local-badge">
            <span className="status-dot" />
            {networkAccess ? "受控内网" : "完全离线"}
          </div>
          <button className="profile" type="button" onClick={() => setSettingsOpen(true)}>
            <span className="avatar"><Avatar preferences={preferences} /></span>
            <span>
              <strong>{preferences.displayName || defaultPreferences.displayName}</strong>
              <small>设置</small>
            </span>
          </button>
        </div>
      </aside>

      <main className="main-content">
        <header className="topbar">
          <div>
            <span className="breadcrumb">工作区 / {activeItem.label}</span>
            <h1>{activeItem.label}</h1>
          </div>
          <div className="topbar-actions">
            <button
              className="icon-button"
              type="button"
              onClick={toggleTheme}
              aria-label={preferences.theme === "dark" ? "切换到日常浅色主题" : "切换到深色工程主题"}
              title={preferences.theme === "dark" ? "切换到日常浅色主题" : "切换到深色工程主题"}
            >
              {preferences.theme === "dark" ? "☀" : "◐"}
            </button>
            <div className="runtime-pill">
              <span className="status-dot" />
              {networkAccess ? "内网已启用" : runtime?.appVersion && runtime.appVersion !== "preview" ? "核心已就绪 · 离线" : "界面预览"}
            </div>
          </div>
        </header>

        {activeId === "dashboard" ? (
          <section className="dashboard" aria-labelledby="dashboard-title">
            <article className="hero-panel">
              <div className="hero-copy">
                <p className="eyebrow">{runtime?.demoMode ? "SYNTHETIC DATA · EMBEDDED MODEL" : "OFFLINE-FIRST · SECURE BY DESIGN"}</p>
                <h2 id="dashboard-title">从这里，开始今天的工作。</h2>
                <p>
                  {runtime?.demoMode
                    ? "当前为独立 Demo：仅使用合成资料和内置确定性模型；开启受控内网开关后即可演示对话，结果不用于工程结论。"
                    : "面向 IC 工程师的本地桌面工作台。资料、项目与工作流留在内网，关键操作始终由你确认。"}
                </p>
              </div>
            </article>

            <section className="section-block" aria-labelledby="quick-title">
              <div className="section-heading">
                <div>
                  <h2 id="quick-title">快速开始</h2>
                </div>
                <span>选择一个工作入口</span>
              </div>
              <div className="quick-grid">
                {quickActions.map((action) => (
                  <button key={action.title} type="button" onClick={() => setActiveId(action.target)}>
                    <span className="quick-icon" aria-hidden="true">{action.icon}</span>
                    <span>
                      <strong>{action.title}</strong>
                      <small>{action.detail}</small>
                    </span>
                    <span className="arrow" aria-hidden="true">→</span>
                  </button>
                ))}
              </div>
            </section>

            <section className="status-grid" aria-label="运行状态">
              <article>
                <span className="metric-label">应用版本</span>
                <strong>{runtime?.appVersion ?? "checking"}</strong>
                <p>桌面壳与前端界面</p>
              </article>
              <article>
                <span className="metric-label">数据位置</span>
                <strong>{runtime?.dataLocation ?? "checking"}</strong>
                <p>运行数据边界</p>
              </article>
              <article>
                <span className="metric-label">网络模式</span>
                <strong>{networkAccess ? "INTRANET" : runtime?.networkMode ?? "checking"}</strong>
                <p>{networkAccess ? "受控内网已启用" : "默认拒绝网络访问"}</p>
              </article>
              <article>
                <span className="metric-label">可用配置</span>
                <strong>{loadModelEndpoints().filter((endpoint) => endpoint.enabled).length}</strong>
                <p>已启用的本地/内网模型</p>
              </article>
              <article>
                <span className="metric-label">邮件缓存</span>
                <strong>{dashboard ? `${dashboard.mailAccountCount} / ${dashboard.cachedMailCount}` : "—"}</strong>
                <p>账户 / 本地邮件</p>
              </article>
            </section>
            <section className="recent-grid" aria-label="最近工作内容">
              {[
                { title: "最近项目", count: dashboard?.projectCount, items: dashboard?.recentProjects, empty: "尚未添加项目" },
                { title: "最近文档", count: dashboard?.recentDocuments.length, items: dashboard?.recentDocuments, empty: "尚无索引文档" },
                { title: "最近对话", count: dashboard?.conversationCount, items: dashboard?.recentConversations, empty: "尚无本地会话" },
              ].map((group) => <article key={group.title}><header><strong>{group.title}</strong><span>{group.count ?? 0}</span></header>{group.items?.length ? <ul>{group.items.map((item) => <li key={item}>{item}</li>)}</ul> : <p>{group.empty}</p>}</article>)}
            </section>
          </section>
        ) : activeId === "chat" ? (
          <ModelWorkspace />
        ) : activeId === "converter" ? (
          <ConverterWorkspace />
        ) : activeId === "projects" ? (
          <ProjectWorkspace />
        ) : activeId === "datasheet" ? (
          <DatasheetWorkspace />
        ) : activeId === "skills" ? (
          <SkillsWorkspace />
        ) : activeId === "mail" ? (
          <MailWorkspace />
        ) : activeId === "calendar" ? (
          <CalendarWorkspace />
        ) : activeId === "knowledge" ? (
          <KnowledgeWorkspace />
        ) : activeId === "calculator" ? (
          <CalculatorWorkspace />
        ) : activeId === "software" ? (
          <SoftwareWorkspace />
        ) : null}
      </main>

      {settingsOpen ? (
        <SettingsPanel
          networkAccess={networkAccess}
          onNetworkAccessChange={setNetworkAccess}
          preferences={preferences}
          onPreferencesChange={setPreferences}
          onClose={() => setSettingsOpen(false)}
        />
      ) : null}
      <FloatingAssistant context={assistantContext} onOpenChat={() => setActiveId("chat")} />
    </div>
  );
}

export default App;
