import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import App from "./App";
import { modelEndpointsStorageKey } from "./model-config";
import { clearVerifiedEndpoints, markEndpointVerified } from "./model-session";
import { preferencesStorageKey } from "./preferences";

const invokeMock = vi.hoisted(() => vi.fn());
const dialogOpenMock = vi.hoisted(() => vi.fn());
const dialogSaveMock = vi.hoisted(() => vi.fn());
const eventState = vi.hoisted(() => ({
  handler: undefined as undefined | ((event: { payload: { delta: string; streamId: string } }) => void),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn((_name: string, handler: typeof eventState.handler) => {
    eventState.handler = handler;
    return Promise.resolve(() => undefined);
  }),
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: dialogOpenMock,
  save: dialogSaveMock,
}));

describe("App", () => {
  beforeEach(() => {
    window.localStorage.clear();
    clearVerifiedEndpoints();
    delete document.documentElement.dataset.theme;
    eventState.handler = undefined;
    dialogOpenMock.mockReset();
    dialogSaveMock.mockReset();
    invokeMock.mockImplementation((command: string) => {
      if (command === "has_model_secret") return Promise.resolve(false);
      if (command === "validate_model_endpoint") return Promise.reject(new Error("browser preview"));
      return new Promise(() => undefined);
    });
  });

  it("renders the local-first dashboard", () => {
    render(<App />);

    expect(screen.getByRole("heading", { name: "工作台", level: 1 })).toBeInTheDocument();
    expect(screen.getByText("完全离线")).toBeInTheDocument();
    expect(screen.getByText(/资料、项目与工作流留在内网/)).toBeInTheDocument();
  });

  it("initializes the independent demo with its embedded model", async () => {
    invokeMock.mockImplementation((command: string) => {
      if (command === "get_runtime_status") {
        return Promise.resolve({ appVersion: "1.0.0", dataLocation: "local", demoMode: true, networkMode: "offline" });
      }
      if (command === "get_network_access") return Promise.resolve(false);
      if (command === "get_dashboard_summary") {
        return Promise.resolve({ projectCount: 0, conversationCount: 0, skillCount: 0, mailAccountCount: 0, cachedMailCount: 0, recentProjects: [], recentDocuments: [], recentConversations: [] });
      }
      if (command === "has_model_secret") return Promise.resolve(false);
      return new Promise(() => undefined);
    });

    render(<App />);
    expect(await screen.findByText(/当前为独立 Demo/)).toBeInTheDocument();
    await waitFor(() => expect(window.localStorage.getItem(modelEndpointsStorageKey)).toContain("embedded-demo-model"));
    fireEvent.click(screen.getByRole("button", { name: "智能对话" }));
    expect(await screen.findByRole("option", { name: /内置 Demo 模型/ })).toBeInTheDocument();
  });

  it("opens the Datasheet workspace from navigation", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Datasheet" }));

    expect(screen.getByRole("heading", { name: "Datasheet", level: 1 })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Datasheet 分析", level: 2 })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "导入 PDF Datasheet" })).toBeEnabled();
    expect(screen.getByText(/保留单位、条件和来源页/)).toBeInTheDocument();
  });

  it("opens calendar, knowledge, calculator, and software workspaces from navigation", () => {
    render(<App />);

    for (const name of ["日历与待办", "知识笔记", "科学计算器", "常用软件"]) {
      fireEvent.click(screen.getByRole("button", { name }));
      expect(screen.getByRole("heading", { name, level: 1 })).toBeInTheDocument();
      const workspaceHeading = name === "知识笔记" ? "个人知识笔记" : name;
      expect(screen.getByRole("heading", { name: workspaceHeading, level: 2 })).toBeInTheDocument();
    }
    fireEvent.click(screen.getByRole("button", { name: "科学计算器" }));
    expect(screen.getByLabelText("科学计算表达式")).toHaveValue("");
  });

  it("switches and persists the visual theme", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "切换到日常浅色主题" }));

    expect(document.documentElement.dataset.theme).toBe("light");
    expect(window.localStorage.getItem(preferencesStorageKey)).toContain('\"theme\":\"light\"');
  });

  it("updates the locally stored display name in settings", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: /本地工作区设置/ }));

    expect(screen.getByRole("dialog", { name: "工作台设置" })).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText("显示名称"), {
      target: { value: "模拟 IC 工程师" },
    });
    fireEvent.click(screen.getByRole("button", { name: "完成" }));

    expect(screen.queryByRole("dialog", { name: "工作台设置" })).not.toBeInTheDocument();
    expect(screen.getByText("模拟 IC 工程师")).toBeInTheDocument();
  });

  it("requires an explicit settings action to enable controlled intranet access", async () => {
    invokeMock.mockImplementation((command: string, args?: { enabled?: boolean }) => {
      if (command === "get_network_access") return Promise.resolve(false);
      if (command === "set_network_access") return Promise.resolve(Boolean(args?.enabled));
      if (command === "list_audit_events") return Promise.resolve([]);
      return new Promise(() => undefined);
    });
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: /本地工作区设置/ }));
    fireEvent.click(screen.getByRole("checkbox"));
    expect(await screen.findByText("已允许受控内网连接")).toBeInTheDocument();
    expect(invokeMock).toHaveBeenCalledWith("set_network_access", { enabled: true });
  });

  it("requires explicit current-page authorization and revokes it on navigation", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "打开浮空助手" }));

    expect(screen.getByText("尚未授权页面内容")).toBeInTheDocument();
    expect(screen.getByLabelText("仅询问当前页面")).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: "授权当前页面" }));

    expect(screen.getByText(/已授权本页/)).toBeInTheDocument();
    expect(screen.getByLabelText("仅询问当前页面")).toBeEnabled();
    fireEvent.click(screen.getByRole("button", { name: "Datasheet" }));

    expect(screen.getByText("尚未授权页面内容")).toBeInTheDocument();
    expect(screen.getByLabelText("仅询问当前页面")).toBeDisabled();
    expect(screen.queryByText(/本页是本地工作台概览/)).not.toBeInTheDocument();
  });

  it("creates a local conversation draft without implying a model request", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "智能对话" }));
    fireEvent.click(screen.getByRole("button", { name: /新建对话/ }));
    fireEvent.change(screen.getByLabelText("对话内容"), {
      target: { value: "检查这个偏置电路的设计约束" },
    });
    fireEvent.click(screen.getByRole("button", { name: "保存本地消息" }));

    expect(screen.getAllByText("检查这个偏置电路的设计约束")).toHaveLength(3);
    expect(screen.getByText(/不发送网络请求/)).toBeInTheDocument();
  });

  it("converts a batch of explicitly selected documents and exports a preview", async () => {
    dialogOpenMock.mockResolvedValue(["/synthetic/notes.txt", "/synthetic/spec.pdf"]);
    dialogSaveMock.mockResolvedValue("/synthetic/notes.md");
    invokeMock.mockImplementation((command: string, args?: { path?: string }) => {
      if (command === "convert_document") {
        const pdf = args?.path?.endsWith(".pdf");
        return Promise.resolve({
          id: pdf ? "doc-pdf" : "doc-txt",
          markdown: pdf ? "# spec.pdf\n\n## 参数" : "# notes.txt\n\n偏置电流 10 uA",
          pageCount: pdf ? 2 : undefined,
          sizeBytes: pdf ? 4200 : 24,
          sourceKind: pdf ? "PDF" : "TXT",
          sourceName: pdf ? "spec.pdf" : "notes.txt",
          warnings: pdf ? ["需要人工复核页码映射。"] : [],
        });
      }
      if (command === "load_pdf_data_url") {
        return Promise.resolve("data:application/pdf;base64,JVBERi0xLjQ=");
      }
      if (command === "export_markdown" || command === "export_pdf") return Promise.resolve();
      return new Promise(() => undefined);
    });

    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "格式转换" }));
    fireEvent.click(screen.getByRole("button", { name: "选择文件 / 批量转换" }));

    expect(await screen.findByText("已在本机转换 2 个文档。")).toBeInTheDocument();
    expect(screen.getByText("偏置电流 10 uA")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /PDFspec\.pdf/ }));
    expect(screen.getByRole("heading", { name: "参数", level: 2 })).toBeInTheDocument();
    expect(screen.getByText("需要人工复核页码映射。")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "另存为 .md" }));

    expect(await screen.findByText("已导出 spec.md。")).toBeInTheDocument();
    expect(invokeMock).toHaveBeenCalledWith(
      "export_markdown",
      expect.objectContaining({ path: "/synthetic/notes.md", content: expect.stringContaining("# spec.pdf") }),
    );
  });

  it("authorizes, indexes, searches, previews, and safely removes a local project", async () => {
    const summary = {
      id: "project-0123abcdef",
      name: "synthetic-project",
      rootName: "synthetic-project",
      fileCount: 1,
      indexedCount: 1,
      updatedAt: 1,
    };
    const detail = {
      summary,
      files: [
        {
          documentId: "doc-notes",
          modifiedAt: 1,
          relativePath: "docs/notes.txt",
          sizeBytes: 32,
          sourceKind: "TXT",
          sourceName: "notes.txt",
          warnings: [],
        },
      ],
    };
    dialogOpenMock.mockResolvedValue("/synthetic/synthetic-project");
    invokeMock.mockImplementation((command: string) => {
      if (command === "list_projects") return Promise.resolve([]);
      if (command === "add_project") return Promise.resolve(summary);
      if (command === "index_project" || command === "get_project") return Promise.resolve(detail);
      if (command === "get_indexed_document") {
        return Promise.resolve({
          markdown: "# notes.txt\n\n偏置电流为 10 uA。",
          relativePath: "docs/notes.txt",
          sourceKind: "TXT",
          sourceName: "notes.txt",
          warnings: [],
        });
      }
      if (command === "search_project") {
        return Promise.resolve([
          {
            documentId: "doc-notes",
            relativePath: "docs/notes.txt",
            score: -1,
            snippet: "[偏置]电流为 10 uA。",
            sourceName: "notes.txt",
          },
        ]);
      }
      if (command === "remove_project") return Promise.resolve();
      return new Promise(() => undefined);
    });

    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "项目管理" }));
    fireEvent.click(screen.getByRole("button", { name: "添加本地项目" }));

    expect(await screen.findByText(/已授权并索引 1 个文档/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /TXTdocs\/notes\.txt/ }));
    expect(await screen.findByText("偏置电流为 10 uA。")).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText("搜索项目文档"), { target: { value: "偏置" } });
    fireEvent.click(screen.getByRole("button", { name: "本地搜索" }));
    expect(await screen.findByText("找到 1 条本地来源。")).toBeInTheDocument();
    expect(screen.getByText("[偏置]电流为 10 uA。")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "移除项目" }));
    fireEvent.click(screen.getByRole("button", { name: "确认只移除记录" }));
    expect(await screen.findByText(/源目录和文件未被删除/)).toBeInTheDocument();
    expect(invokeMock).toHaveBeenCalledWith("remove_project", { projectId: summary.id });
  });

  it("extracts a PDF by page and generates a source-constrained Datasheet overview", async () => {
    window.localStorage.setItem(
      modelEndpointsStorageKey,
      JSON.stringify([
        {
          id: "datasheet-model",
          name: "内网模型",
          baseUrl: "http://127.0.0.1:11434/v1",
          model: "local-model",
          enabled: true,
          status: "unchecked",
          statusDetail: "测试",
          checkedAt: "",
        },
      ]),
    );
    markEndpointVerified("datasheet-model");
    dialogOpenMock.mockResolvedValue("/synthetic/synthetic-datasheet.pdf");
    dialogSaveMock.mockResolvedValue("/synthetic/synthetic-datasheet-overview.md");
    invokeMock.mockImplementation((command: string) => {
      if (command === "convert_document") {
        return Promise.resolve({
          id: "doc-datasheet",
          markdown: "# synthetic-datasheet.pdf\n\n## 第 1 页\n\nSupply current 180 uA",
          pageCount: 2,
          sizeBytes: 3488,
          sourceKind: "PDF",
          sourceName: "synthetic-datasheet.pdf",
          warnings: ["复杂表格需要人工复核。"],
        });
      }
      if (command === "send_chat_completion") {
        return Promise.resolve({
          content: "## 关键电气参数\n\n| 参数 | 值 | 来源 |\n|---|---:|---|\n| 电源电流 | 180 uA | 第 1 页 |",
          requestId: "req-datasheet",
        });
      }
      if (command === "load_pdf_data_url") {
        return Promise.resolve("data:application/pdf;base64,JVBERi0xLjQ=");
      }
      if (command === "export_markdown" || command === "export_pdf") return Promise.resolve();
      return new Promise(() => undefined);
    });

    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Datasheet" }));
    fireEvent.click(screen.getByRole("button", { name: "导入 PDF Datasheet" }));
    expect(await screen.findByText(/已在本机按页提取 2 页/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "按页文本" }));
    expect(screen.getByRole("heading", { name: "第 1 页", level: 2 })).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "生成可追溯概略版" }));
    expect(await screen.findByText("180 uA")).toBeInTheDocument();
    expect(screen.getByText("请求 ID：req-datasheet")).toBeInTheDocument();
    const analysisCall = invokeMock.mock.calls.find(([command]) => command === "send_chat_completion");
    expect(JSON.stringify(analysisCall?.[1])).toContain("严禁补造参数");
    expect(JSON.stringify(analysisCall?.[1])).toContain("## 第 1 页");

    fireEvent.click(screen.getByRole("button", { name: "导出 Markdown" }));
    expect(await screen.findByText(/已导出 synthetic-datasheet-overview\.md/)).toBeInTheDocument();
    dialogSaveMock.mockResolvedValueOnce("/synthetic/synthetic-datasheet-overview.pdf");
    fireEvent.click(screen.getByRole("button", { name: "导出 PDF" }));
    expect(await screen.findByText(/已导出 synthetic-datasheet-overview\.pdf/)).toBeInTheDocument();
    expect(invokeMock).toHaveBeenCalledWith("export_pdf", expect.objectContaining({
      path: "/synthetic/synthetic-datasheet-overview.pdf",
      markdown: expect.stringContaining("180 uA"),
    }));
  });

  it("imports a prompt-only Skill disabled and requires explicit enablement", async () => {
    const record = {
      id: "skill-0123456789abcdef",
      name: "analog-review",
      description: "检查单位和条件",
      instructions: "只根据输入内容检查单位，并列出来源。",
      sourceName: "analog-review",
      enabled: false,
      hasScripts: false,
    };
    let records: typeof record[] = [];
    dialogOpenMock.mockResolvedValue("/synthetic/analog-review");
    invokeMock.mockImplementation((command: string, args?: { enabled?: boolean }) => {
      if (command === "list_skills") return Promise.resolve(records);
      if (command === "import_skill") {
        records = [record];
        return Promise.resolve(record);
      }
      if (command === "set_skill_enabled") {
        records = [{ ...record, enabled: Boolean(args?.enabled) }];
        return Promise.resolve(records[0]);
      }
      return new Promise(() => undefined);
    });

    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Skills" }));
    fireEvent.click(screen.getByRole("button", { name: "导入 Skill 目录" }));

    expect(await screen.findByText(/说明型 Skill 已导入并保持停用/)).toBeInTheDocument();
    expect(screen.getByText("只根据输入内容检查单位，并列出来源。")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "启用" }));
    expect(await screen.findByText(/可在智能对话中显式选择/)).toBeInTheDocument();
    expect(invokeMock).toHaveBeenCalledWith("set_skill_enabled", {
      skillId: record.id,
      enabled: true,
    });
  });

  it("injects only an explicitly selected prompt Skill into a model request", async () => {
    const skill = {
      id: "skill-abcdef0123456789",
      name: "source-review",
      description: "来源复核",
      instructions: "回答必须列出来源和单位。",
      sourceName: "SKILL.md",
      enabled: true,
      hasScripts: false,
    };
    invokeMock.mockImplementation((command: string) => {
      if (command === "has_model_secret") return Promise.resolve(false);
      if (command === "load_conversations") return Promise.resolve([]);
      if (command === "list_skills") return Promise.resolve([skill]);
      if (command === "test_model_connection") return Promise.resolve({ success: true, message: "连接成功" });
      if (command === "save_conversation") return Promise.resolve();
      if (command === "send_chat_completion_stream") return Promise.resolve({ content: "已复核", requestId: "req-skill" });
      return Promise.reject(new Error("unsupported test command"));
    });

    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "智能对话" }));
    fireEvent.click(screen.getByRole("button", { name: "管理模型 API" }));
    fireEvent.click(screen.getByRole("button", { name: "连接检测" }));
    expect(await screen.findByText("连接已验证")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "收起 API 管理" }));
    fireEvent.change(await screen.findByLabelText("本次对话 Skill"), { target: { value: skill.id } });
    fireEvent.change(screen.getByLabelText("对话内容"), { target: { value: "检查偏置参数" } });
    fireEvent.click(screen.getByRole("button", { name: "发送" }));

    expect(await screen.findByText("已复核")).toBeInTheDocument();
    const request = invokeMock.mock.calls.find(([command]) => command === "send_chat_completion_stream");
    expect(request?.[1]).toEqual(expect.objectContaining({
      messages: expect.arrayContaining([
        expect.objectContaining({ role: "developer", content: expect.stringContaining("回答必须列出来源和单位") }),
        expect.objectContaining({ role: "user", content: "检查偏置参数" }),
      ]),
    }));
  });

  it("stores mail metadata separately from system credentials without connecting", async () => {
    const account = {
      id: "mail-0123456789abcdef",
      address: "engineer@example.internal",
      displayName: "工程邮箱",
      username: "engineer",
      imapHost: "mail.internal",
      imapPort: 993,
      smtpHost: "mail.internal",
      smtpPort: 465,
      updatedAt: 1,
    };
    let saved = false;
    invokeMock.mockImplementation((command: string) => {
      if (command === "list_mail_accounts") return Promise.resolve(saved ? [account] : []);
      if (command === "save_mail_account") { saved = true; return Promise.resolve(account); }
      if (command === "save_mail_secrets") return Promise.resolve();
      if (command === "has_mail_secrets") return Promise.resolve(true);
      if (command === "send_mail") return Promise.resolve();
      return new Promise(() => undefined);
    });

    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "邮件助手" }));
    fireEvent.change(screen.getByLabelText("显示名称"), { target: { value: "工程邮箱" } });
    fireEvent.change(screen.getByLabelText("邮箱地址"), { target: { value: account.address } });
    fireEvent.change(screen.getByLabelText("登录用户名"), { target: { value: account.username } });
    fireEvent.change(screen.getByLabelText("IMAP 内网主机"), { target: { value: account.imapHost } });
    fireEvent.change(screen.getByLabelText("SMTP 内网主机"), { target: { value: account.smtpHost } });
    fireEvent.change(screen.getByLabelText("IMAP 密码"), { target: { value: "synthetic-imap" } });
    fireEvent.change(screen.getByLabelText("SMTP 密码"), { target: { value: "synthetic-smtp" } });
    fireEvent.click(screen.getByRole("button", { name: "安全保存账户" }));

    expect(await screen.findByText(/密码只进入系统凭据库/)).toBeInTheDocument();
    expect(screen.getAllByText(account.address).length).toBeGreaterThanOrEqual(2);
    expect(invokeMock).not.toHaveBeenCalledWith("test_mail_account", expect.anything());

    fireEvent.click(screen.getByRole("button", { name: "写邮件" }));
    fireEvent.change(screen.getByLabelText("收件人"), { target: { value: "reviewer@example.internal" } });
    fireEvent.change(screen.getByLabelText("主题"), { target: { value: "合成复核" } });
    fireEvent.change(screen.getByLabelText("正文"), { target: { value: "请复核合成数据。" } });
    fireEvent.click(screen.getByRole("button", { name: "检查并准备发送" }));
    expect(screen.getByRole("dialog", { name: "确认发送这封邮件？" })).toBeInTheDocument();
    expect(invokeMock).not.toHaveBeenCalledWith("send_mail", expect.anything());
    fireEvent.click(screen.getByRole("button", { name: "确认并发送" }));
    expect(await screen.findByText(/SMTP 已确认接收邮件/)).toBeInTheDocument();
    expect(invokeMock).toHaveBeenCalledWith("send_mail", expect.objectContaining({ confirmed: true }));
  });

  it("attaches an explicitly selected supported local image", async () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "智能对话" }));
    const image = new File(
      [new Uint8Array([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a])],
      "synthetic.png",
      { type: "image/png" },
    );
    fireEvent.change(screen.getByLabelText("添加图片附件"), {
      target: { files: [image] },
    });

    expect(await screen.findByText("synthetic.png")).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText("对话内容"), {
      target: { value: "分析这张合成图片" },
    });
    fireEvent.click(screen.getByRole("button", { name: "保存本地消息" }));

    expect(await screen.findByRole("img", { name: "synthetic.png" })).toBeInTheDocument();
    expect(invokeMock).toHaveBeenCalledWith(
      "save_conversation",
      expect.objectContaining({
        conversation: expect.objectContaining({
          messages: expect.arrayContaining([
            expect.objectContaining({
              attachments: [
                expect.objectContaining({ mimeType: "image/png", name: "synthetic.png" }),
              ],
            }),
          ]),
        }),
      }),
    );
  });

  it("parses an explicitly selected document before attaching it to chat", async () => {
    dialogOpenMock.mockResolvedValue("/synthetic/bias-notes.docx");
    invokeMock.mockImplementation((command: string) => {
      if (command === "has_model_secret") return Promise.resolve(false);
      if (command === "load_conversations" || command === "list_skills") return Promise.resolve([]);
      if (command === "convert_document") return Promise.resolve({
        id: "doc-bias",
        markdown: "# bias-notes.docx\n\n偏置电流 10 uA，条件 VDD=1.8 V。",
        sourceKind: "DOCX",
        sourceName: "bias-notes.docx",
        warnings: ["复杂版式需要人工复核。"],
      });
      if (command === "save_conversation") return Promise.resolve();
      return new Promise(() => undefined);
    });

    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "智能对话" }));
    fireEvent.click(screen.getByRole("button", { name: "＋ 文档" }));
    expect(await screen.findByText(/bias-notes\.docx/)).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText("对话内容"), { target: { value: "复核文档参数" } });
    fireEvent.click(screen.getByRole("button", { name: "保存本地消息" }));

    expect(await screen.findByText(/DOCX · bias-notes\.docx/)).toBeInTheDocument();
    expect(invokeMock).toHaveBeenCalledWith(
      "save_conversation",
      expect.objectContaining({
        conversation: expect.objectContaining({
          messages: expect.arrayContaining([
            expect.objectContaining({
              documents: [expect.objectContaining({ sourceName: "bias-notes.docx", markdown: expect.stringContaining("10 uA") })],
            }),
          ]),
        }),
      }),
    );
  });

  it("checks endpoint metadata without making a network claim", async () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "智能对话" }));
    fireEvent.click(screen.getByRole("button", { name: "管理模型 API" }));
    fireEvent.click(screen.getByRole("button", { name: "检查配置" }));

    expect(await screen.findByText(/地址与内网访问策略检查通过/)).toBeInTheDocument();
    expect(screen.getByText(/尚未发起网络连接/)).toBeInTheDocument();
    expect(screen.getByText(/API 密钥只写入操作系统凭据库/)).toBeInTheDocument();
  });

  it("edits a configured model endpoint and requires a new connection test", async () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "智能对话" }));
    fireEvent.click(screen.getByRole("button", { name: "管理模型 API" }));
    fireEvent.click(screen.getByRole("button", { name: "编辑" }));
    fireEvent.change(screen.getByLabelText("本地 OpenAI 兼容服务 编辑名称"), {
      target: { value: "本机 Demo 模型" },
    });
    fireEvent.change(screen.getByLabelText("本地 OpenAI 兼容服务 编辑基础地址"), {
      target: { value: "http://127.0.0.1:18080/v1/" },
    });
    fireEvent.change(screen.getByLabelText("本地 OpenAI 兼容服务 编辑模型标识"), {
      target: { value: "demo-analog-assistant" },
    });
    fireEvent.click(screen.getByRole("button", { name: "保存编辑" }));

    expect(await screen.findByText("本机 Demo 模型")).toBeInTheDocument();
    expect(screen.getByText("http://127.0.0.1:18080/v1")).toBeInTheDocument();
    expect(screen.getByText("端点已编辑，请重新执行连接检测。")).toBeInTheDocument();
  });

  it("enables model sending only after a successful connection test", async () => {
    invokeMock.mockImplementation((command: string, args?: { streamId?: string }) => {
      if (command === "has_model_secret") return Promise.resolve(false);
      if (command === "test_model_connection") {
        return Promise.resolve({ success: true, message: "模型服务连接成功。", requestId: "req-test" });
      }
      if (command === "send_chat_completion_stream") {
        if (args?.streamId && eventState.handler) {
          eventState.handler({ payload: { delta: "模拟模型", streamId: args.streamId } });
        }
        return Promise.resolve({ content: "模拟模型回答", requestId: "req-chat" });
      }
      if (command === "save_conversation") return Promise.resolve();
      return Promise.reject(new Error("unsupported test command"));
    });

    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "智能对话" }));
    fireEvent.click(screen.getByRole("button", { name: "管理模型 API" }));
    fireEvent.click(screen.getByRole("button", { name: "连接检测" }));

    expect(await screen.findByText("连接已验证")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "收起 API 管理" }));
    fireEvent.change(screen.getByLabelText("对话内容"), { target: { value: "测试问题" } });
    fireEvent.click(screen.getByRole("button", { name: "发送" }));

    expect(await screen.findByText("模拟模型回答")).toBeInTheDocument();
    expect(screen.getByText("请求 ID：req-chat")).toBeInTheDocument();
    expect(invokeMock).toHaveBeenCalledWith(
      "send_chat_completion_stream",
      expect.objectContaining({ streamId: expect.stringMatching(/^stream-/) }),
    );
  });

  it("sends only the explicitly authorized page summary to the floating assistant", async () => {
    invokeMock.mockImplementation((command: string) => {
      if (command === "has_model_secret") return Promise.resolve(false);
      if (command === "test_model_connection") {
        return Promise.resolve({ success: true, message: "模型服务连接成功。" });
      }
      if (command === "send_chat_completion") {
        return Promise.resolve({ content: "## 页面回答\n\n请复核来源。", requestId: "req-float" });
      }
      if (command === "save_conversation") return Promise.resolve();
      return new Promise(() => undefined);
    });

    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "智能对话" }));
    fireEvent.click(screen.getByRole("button", { name: "管理模型 API" }));
    fireEvent.click(screen.getByRole("button", { name: "连接检测" }));
    expect(await screen.findByText("连接已验证")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Datasheet" }));
    fireEvent.click(screen.getByRole("button", { name: "打开浮空助手" }));
    fireEvent.click(screen.getByRole("button", { name: "授权当前页面" }));
    fireEvent.change(screen.getByLabelText("仅询问当前页面"), {
      target: { value: "这个页面能做什么？" },
    });
    fireEvent.click(screen.getByRole("button", { name: "发送本页问题" }));

    expect(await screen.findByRole("heading", { name: "页面回答", level: 2 })).toBeInTheDocument();
    expect(screen.getByText("请求 ID：req-float")).toBeInTheDocument();
    const floatingCall = invokeMock.mock.calls.find(([command]) => command === "send_chat_completion");
    expect(floatingCall?.[1]).toEqual(
      expect.objectContaining({
        messages: expect.arrayContaining([
          expect.objectContaining({
            role: "user",
            content: expect.stringContaining("当前页面：Datasheet"),
          }),
        ]),
      }),
    );
    expect(JSON.stringify(floatingCall?.[1])).not.toContain("本页是本地工作台概览");
  });

  it("restores conversations from the local database", async () => {
    invokeMock.mockImplementation((command: string) => {
      if (command === "has_model_secret") return Promise.resolve(false);
      if (command === "load_conversations") {
        return Promise.resolve([
          {
            id: "conversation-restored",
            title: "已恢复会话",
            messages: [{ id: "message-restored", role: "user", text: "来自 SQLite 的内容" }],
          },
        ]);
      }
      return Promise.reject(new Error("unsupported test command"));
    });

    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "智能对话" }));

    expect(await screen.findAllByText("已恢复会话")).toHaveLength(2);
    expect(screen.getByText("来自 SQLite 的内容")).toBeInTheDocument();
    expect(screen.getByText(/保存到本机 SQLite/)).toBeInTheDocument();
  });

  it("sends secrets to the credential command without persisting them in browser storage", async () => {
    invokeMock.mockImplementation((command: string) => {
      if (command === "has_model_secret") return Promise.resolve(false);
      if (command === "save_model_secret") return Promise.resolve();
      return Promise.reject(new Error("unsupported test command"));
    });

    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "智能对话" }));
    fireEvent.click(screen.getByRole("button", { name: "管理模型 API" }));
    fireEvent.change(screen.getByLabelText("本地 OpenAI 兼容服务 API 密钥"), {
      target: { value: "synthetic-test-token" },
    });
    fireEvent.click(screen.getByRole("button", { name: "安全保存" }));

    expect(await screen.findByText("已存入系统凭据库")).toBeInTheDocument();
    expect(window.localStorage.getItem(modelEndpointsStorageKey)).not.toContain("synthetic-test-token");
  });
});
