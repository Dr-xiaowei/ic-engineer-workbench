import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import CalendarWorkspace from "./CalendarWorkspace";
import SoftwareWorkspace from "./SoftwareWorkspace";

const invoke = vi.hoisted(() => vi.fn());
const open = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open }));

function task(index: number, status = "todo") {
  return { id: `task-${index}`, title: `合成待办${index}`, startAt: Math.floor(Date.now() / 1000),
    status, priority: "important", remindAt: Math.floor(Date.now() / 1000) - 60,
    projectId: "project-demo", projectName: "合成项目", notes: "仅测试", updatedAt: 0 };
}
const launcher = { id: "launcher-demo", name: "合成工具", targetName: "Synthetic.app", targetKind: "application", updatedAt: 0 };

beforeEach(() => { invoke.mockReset(); open.mockReset(); });

describe("calendar data coverage", () => {
  it("shows and edits every event even when a day contains more than four", async () => {
    invoke.mockImplementation(async (command: string) => command === "list_projects" ? [] : Array.from({ length: 7 }, (_, i) => task(i)));
    render(<CalendarWorkspace />);
    const last = (await screen.findAllByRole("button", { name: /合成待办6/ })).find(button => button.classList.contains("calendar-event"))!;
    expect(document.querySelectorAll(".calendar-event")).toHaveLength(7);
    fireEvent.doubleClick(last);
    expect(screen.getByRole("dialog", { name: "编辑待办" })).toBeInTheDocument();
    expect(screen.getByLabelText("任务名称")).toHaveValue("合成待办6");
  });

  it("saves project association, priority, status and reminder through the existing command", async () => {
    invoke.mockImplementation(async (command: string) => command === "list_projects" ? [{ id: "project-demo", name: "合成项目" }] : []);
    render(<CalendarWorkspace />);
    fireEvent.click(screen.getByRole("button", { name: "新建待办" }));
    await screen.findByRole("option", { name: "合成项目" });
    fireEvent.change(screen.getByLabelText("任务名称"), { target: { value: "评审" } });
    fireEvent.change(screen.getByLabelText("开始时间"), { target: { value: "2026-09-20T09:00" } });
    fireEvent.change(screen.getByLabelText("状态"), { target: { value: "doing" } });
    fireEvent.change(screen.getByLabelText("优先级"), { target: { value: "important" } });
    fireEvent.change(screen.getByLabelText("关联项目"), { target: { value: "project-demo" } });
    fireEvent.click(screen.getByLabelText("提前 30 分钟在应用内提醒"));
    fireEvent.click(screen.getByRole("button", { name: "保存待办" }));
    const start = new Date("2026-09-20T09:00").getTime() / 1000;
    await waitFor(() => expect(invoke).toHaveBeenCalledWith("save_calendar_task", { input: expect.objectContaining({ title: "评审", startAt: start, remindAt: start - 1800, status: "doing", priority: "important", projectId: "project-demo" }) }));
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
  });

  it("does not remind completed events and recovers from a failed deletion", async () => {
    let deleted = false;
    let fail = true;
    invoke.mockImplementation(async (command: string) => {
      if (command === "list_calendar_tasks") return deleted ? [] : [task(1, "done")];
      if (command === "delete_calendar_task") { if (fail) throw new Error("test failure"); deleted = true; }
      return [];
    });
    render(<CalendarWorkspace />);
    fireEvent.click(await screen.findByRole("button", { name: /合成待办1/ }));
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "删除" }));
    await waitFor(() => expect(within(screen.getByRole("dialog")).getByRole("status")).toHaveTextContent("无法删除待办"));
    fail = false;
    fireEvent.click(screen.getByRole("button", { name: "删除" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
    expect(screen.queryByRole("button", { name: /合成待办1/ })).not.toBeInTheDocument();
  });

  it("preserves the draft after save failure and reports unavailable data", async () => {
    invoke.mockRejectedValue("合成失败");
    render(<CalendarWorkspace />);
    await screen.findByText("日历待办仅在桌面应用中可用。");
    fireEvent.click(screen.getByRole("button", { name: "新建待办" }));
    fireEvent.change(screen.getByLabelText("任务名称"), { target: { value: "保留草稿" } });
    fireEvent.click(screen.getByRole("button", { name: "保存待办" }));
    await waitFor(() => expect(within(screen.getByRole("dialog")).getByRole("status")).toHaveTextContent("合成失败"));
    expect(screen.getByLabelText("任务名称")).toHaveValue("保留草稿");
  });
});

describe("software data coverage", () => {
  it("cancelling a picker does not add or launch anything", async () => {
    invoke.mockResolvedValue([]); open.mockResolvedValue(null);
    render(<SoftwareWorkspace />);
    fireEvent.click(screen.getByRole("button", { name: "选择软件" }));
    await waitFor(() => expect(open).toHaveBeenCalledOnce());
    expect(screen.queryByLabelText("快捷名称")).not.toBeInTheDocument();
    expect(invoke.mock.calls.every(([command]) => command === "list_software_launchers")).toBe(true);
  });

  it("adds, filters and explicitly launches only the selected entry", async () => {
    let added = false;
    invoke.mockImplementation(async (command: string) => {
      if (command === "list_software_launchers") return added ? [launcher] : [];
      if (command === "add_software_launcher") added = true;
    });
    open.mockResolvedValue("/Applications/Synthetic.app");
    render(<SoftwareWorkspace />);
    fireEvent.click(screen.getByRole("button", { name: "选择软件" }));
    fireEvent.change(await screen.findByLabelText("快捷名称"), { target: { value: "合成工具" } });
    fireEvent.click(screen.getByRole("button", { name: "添加快捷入口" }));
    await screen.findByRole("button", { name: /Synthetic.app/ });
    expect(invoke).toHaveBeenCalledWith("add_software_launcher", { path: "/Applications/Synthetic.app", name: "合成工具" });
    expect(invoke).not.toHaveBeenCalledWith("launch_software", expect.anything());
    fireEvent.change(screen.getByLabelText("搜索常用软件"), { target: { value: "不存在" } });
    expect(screen.queryByRole("button", { name: /Synthetic.app/ })).not.toBeInTheDocument();
    fireEvent.change(screen.getByLabelText("搜索常用软件"), { target: { value: "合成" } });
    fireEvent.click(screen.getByRole("button", { name: /Synthetic.app/ }));
    await waitFor(() => expect(invoke).toHaveBeenCalledWith("launch_software", { launcherId: launcher.id }));
  });

  it("requires removal confirmation and keeps the entry on failure", async () => {
    let fail = true; let removed = false;
    invoke.mockImplementation(async (command: string) => {
      if (command === "list_software_launchers") return removed ? [] : [launcher];
      if (command === "remove_software_launcher") { if (fail) throw new Error("test failure"); removed = true; }
    });
    render(<SoftwareWorkspace />);
    fireEvent.click(await screen.findByRole("button", { name: "移除" }));
    expect(invoke).not.toHaveBeenCalledWith("remove_software_launcher", expect.anything());
    fireEvent.click(screen.getByRole("button", { name: "确认移除" }));
    await screen.findByText("操作失败，请检查所选软件。");
    expect(screen.getByRole("button", { name: /Synthetic.app/ })).toBeInTheDocument();
    fail = false;
    fireEvent.click(screen.getByRole("button", { name: "确认移除" }));
    await waitFor(() => expect(screen.queryByRole("button", { name: /Synthetic.app/ })).not.toBeInTheDocument());
    expect(screen.getByRole("status")).toHaveTextContent("软件本身未被修改");
  });
});
