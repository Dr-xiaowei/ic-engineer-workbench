import { describe, expect, it } from "vitest";
import { noteDocx, noteHtml } from "./note-export";

describe("portable note export", () => {
  it("renders headings and tables and excludes executable or remote content", () => {
    const html = noteHtml("测试", "## 参数\n\n| 名称 | 值 |\n|---|---|\n| 电流 | 10 uA |\n\n**重点**\n\n<script>alert(1)</script>\n\n![外链](https://example.com/a.png)");
    expect(html).toContain("<h2>参数</h2>");
    expect(html).toContain("<table>");
    expect(html).toContain("<strong>重点</strong>");
    expect(html).not.toContain("<script>");
    expect(html).not.toContain('src="https://');
  });
  it("creates a real OOXML archive from structured note content", async () => {
    const bytes = await noteDocx("导出标题", "## 二级标题\n\n**粗体**文字\n\n| 参数 | 数值 |\n|---|---|\n| VDD | 1.8 V |");
    expect(atob(bytes).startsWith("PK")).toBe(true);
    expect(bytes.length).toBeGreaterThan(4000);
  });
});
