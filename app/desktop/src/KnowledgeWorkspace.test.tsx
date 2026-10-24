import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import KnowledgeWorkspace from "./KnowledgeWorkspace";
import { invoke } from "@tauri-apps/api/core";
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn(), save: vi.fn() }));
beforeEach(() => { vi.mocked(invoke).mockImplementation(async command => command === "list_notes" ? [{ id:"note-a",title:"第一篇",body:"已保存",updatedAt:0,createdAt:0 },{ id:"note-b",title:"第二篇",body:"正文",updatedAt:0,createdAt:0 }] : command === "list_projects" ? [] : null); });
it("starts new notes in edit mode and blocks discarding edits on selection", async () => {
  render(<KnowledgeWorkspace/>);
  await waitFor(() => expect(screen.getByLabelText("笔记标题")).toHaveValue("第一篇"));
  fireEvent.click(screen.getByText("新建笔记"));
  expect(screen.getByLabelText("笔记内容")).toBeVisible();
  fireEvent.change(screen.getByLabelText("笔记标题"), { target: { value:"未保存" } });
  fireEvent.click(screen.getByText("第二篇"));
  expect(screen.getByLabelText("笔记标题")).toHaveValue("未保存");
  expect(screen.getByText("请先保存当前编辑，再切换或新建笔记。")).toBeInTheDocument();
});

it("requires confirmation and preserves a note after a failed delete so it can be retried", async () => {
  let deleted = false;
  let attempts = 0;
  vi.mocked(invoke).mockImplementation(async command => {
    if (command === "list_notes") return deleted
      ? [{ id: "note-b", title: "第二篇", body: "正文", updatedAt: 0, createdAt: 0 }]
      : [{ id: "note-a", title: "第一篇", body: "已保存", updatedAt: 0, createdAt: 0 }, { id: "note-b", title: "第二篇", body: "正文", updatedAt: 0, createdAt: 0 }];
    if (command === "list_projects") return [];
    if (command === "delete_note") {
      if (++attempts === 1) throw new Error("无法移除笔记映射文件。");
      deleted = true;
    }
    return null;
  });
  render(<KnowledgeWorkspace />);
  await waitFor(() => expect(screen.getByLabelText("笔记标题")).toHaveValue("第一篇"));
  fireEvent.click(screen.getByRole("button", { name: "删除" }));
  expect(attempts).toBe(0);
  fireEvent.click(screen.getByRole("button", { name: "确认删除" }));
  await waitFor(() => expect(screen.getByText("无法删除笔记。")).toBeVisible());
  expect(screen.getByLabelText("笔记标题")).toHaveValue("第一篇");
  fireEvent.click(screen.getByRole("button", { name: "编辑" }));
  expect(screen.getByLabelText("笔记内容")).toHaveValue("已保存");
  fireEvent.click(screen.getByRole("button", { name: "确认删除" }));
  await waitFor(() => expect(screen.getByText("笔记已删除。")).toBeVisible());
  expect(screen.queryByRole("button", { name: /第一篇/ })).not.toBeInTheDocument();
  expect(screen.getByRole("button", { name: /第二篇/ })).toBeVisible();
});
