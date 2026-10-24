import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import ConverterWorkspace from "./ConverterWorkspace";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn(), save: vi.fn() }));
beforeEach(() => {
  vi.clearAllMocks(); localStorage.clear();
  vi.mocked(open).mockResolvedValue(["/fixture/review.md"]);
  vi.mocked(save).mockResolvedValue("/exports/renamed.md");
  vi.mocked(invoke).mockImplementation(async (command, args) => command === "convert_document" ? { id: "same-content", markdown: "# 评审\n\n偏置电流 10 µA", sizeBytes: 42, sourceKind: "MD", sourceName: String((args as {path:string}).path).split("/").at(-1), warnings: [] } : null);
});

it("reuses the unified export backend, protects inputs and reports the actual chosen filename", async () => {
  render(<ConverterWorkspace />);
  fireEvent.click(screen.getByRole("button", { name: "选择文件 / 批量转换" }));
  await screen.findByText("偏置电流 10 µA");
  expect(vi.mocked(open).mock.calls[0][0]).toMatchObject({ filters: [{ extensions: expect.arrayContaining(["pdf", "docx", "pptx", "md"]) }] });
  fireEvent.click(screen.getByRole("button", { name: "另存为新文件" }));
  await screen.findByText("已导出 renamed.md。");
  expect(invoke).toHaveBeenCalledWith("export_converted_document", expect.objectContaining({ path: "/exports/renamed.md", format: "md", body: "# 评审\n\n偏置电流 10 µA", sourcePaths: ["/fixture/review.md"] }));
  expect(save).toHaveBeenCalledWith(expect.objectContaining({ defaultPath: "review_转换.md" }));
});

it("retains converted content on cancellation or export failure and allows retry", async () => {
  render(<ConverterWorkspace />); fireEvent.click(screen.getByRole("button", { name: "选择文件 / 批量转换" }));
  await screen.findByText("偏置电流 10 µA");
  vi.mocked(save).mockResolvedValueOnce(null);
  fireEvent.click(screen.getByRole("button", { name: "另存为新文件" }));
  await waitFor(() => expect(screen.getByRole("button", { name: "另存为新文件" })).toBeEnabled());
  expect(invoke).not.toHaveBeenCalledWith("export_converted_document", expect.anything());
  vi.mocked(invoke).mockRejectedValueOnce("目标已存在，请另选新文件名。");
  fireEvent.click(screen.getByRole("button", { name: "另存为新文件" }));
  await screen.findByText("目标已存在，请另选新文件名。");
  expect(screen.getByText("偏置电流 10 µA")).toBeVisible();
});

it("distinguishes equal-content files and supports PDF export without reimporting into notes", async () => {
  vi.mocked(open).mockResolvedValue(["/fixture/first.md", "/fixture/second.md"]);
  render(<ConverterWorkspace />); fireEvent.click(screen.getByRole("button", { name: "选择文件 / 批量转换" }));
  await screen.findByText("已在本机转换 2 个文档。");
  fireEvent.click(screen.getByRole("button", { name: /second.md/ }));
  fireEvent.change(screen.getByLabelText("文档导出格式"), { target: { value: "pdf" } });
  vi.mocked(save).mockResolvedValueOnce("/exports/review.pdf");
  fireEvent.click(screen.getByRole("button", { name: "另存为新文件" }));
  await screen.findByText("已导出 review.pdf。");
  expect(invoke).toHaveBeenCalledWith("export_converted_document", expect.objectContaining({ format: "pdf", sourcePaths: ["/fixture/first.md", "/fixture/second.md"] }));
  expect(invoke).not.toHaveBeenCalledWith("save_note", expect.anything());
});
