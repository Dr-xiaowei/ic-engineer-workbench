import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import KnowledgeWorkspace from "./KnowledgeWorkspace";
import { invoke } from "@tauri-apps/api/core";
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn(), save: vi.fn() }));
beforeEach(() => { vi.mocked(invoke).mockImplementation(async command => command === "list_notes" ? [{ id:"note-a",title:"第一篇",body:"已保存",updatedAt:0,createdAt:0 },{ id:"note-b",title:"第二篇",body:"正文",updatedAt:0,createdAt:0 }] : null); });
it("starts new notes in edit mode and blocks discarding edits on selection", async () => {
  render(<KnowledgeWorkspace/>);
  await waitFor(() => expect(screen.getByLabelText("笔记标题")).toHaveValue("第一篇"));
  fireEvent.click(screen.getByText("新建笔记"));
  expect(screen.getByLabelText("笔记内容")).toBeVisible();
  fireEvent.change(screen.getByLabelText("笔记标题"), { target: { value:"未保存" } });
  fireEvent.click(screen.getByText("第二篇"));
  expect(screen.getByLabelText("笔记标题")).toHaveValue("未保存");
  expect(screen.getByRole("status")).toHaveTextContent("请先保存");
});
