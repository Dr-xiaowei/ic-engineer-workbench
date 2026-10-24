import { describe, expect, it } from "vitest";
import { createRequire } from "node:module";
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { documentText, documentTitleBody, formattedDocument } from "./document-export";
import { presentationBase64, wrapPresentationText } from "./presentation-export";

const require = createRequire(import.meta.url);
const JSZip = createRequire(require.resolve("pptxgenjs"))("jszip");
const body = "## 偏置检查\n\n条件：VDD=1.8 V，T=25°C。\n\n- 偏置电流 10 µA\n- **失调 ±2 mV**\n\n| 参数 | 数值 |\n|---|---|\n| GBW | 10 MHz |\n| 相位裕度 | 60° |\n\n---\n\n## 结论\n\n这是合成验收资料。<script>steal()</script>\n\n![外部图](https://example.invalid/private.png)";

describe("document conversion export", () => {
  it("reuses note DOCX export and generates editable PPTX without remote resources", async () => {
    const pptx = await presentationBase64("模拟 IC 评审", body);
    const archive = await JSZip.loadAsync(pptx, { base64: true });
    const slides = Object.keys(archive.files).filter(name => /^ppt\/slides\/slide\d+\.xml$/.test(name));
    expect(slides.length).toBeGreaterThanOrEqual(3);
    const text = (await Promise.all(slides.map(name => archive.file(name).async("string")))).join("\n");
    expect(text).toContain("10 µA"); expect(text).toContain("±2 mV"); expect(text).toContain("<a:tbl>");
    expect(text).toContain("10 MHz"); expect(text).toContain("60°");
    expect(text).not.toContain("<script>");
    expect(Object.keys(archive.files).some(name => /vbaProject/i.test(name))).toBe(false);
    const rels = await Promise.all(Object.keys(archive.files).filter(name => name.endsWith(".rels")).map(name => archive.file(name).async("string")));
    expect(rels.join("\n")).not.toContain("example.invalid");
    const docx = await formattedDocument("docx", "模拟 IC 评审", body);
    const word = await JSZip.loadAsync(docx, { base64: true });
    expect(await word.file("word/document.xml").async("string")).toContain("10 µA");
    if (process.env.ICWB_CONVERSION_FIXTURES) {
      const directory = process.env.ICWB_CONVERSION_FIXTURES; mkdirSync(directory, { recursive: true });
      writeFileSync(join(directory, "review.pptx"), Buffer.from(pptx, "base64"));
      writeFileSync(join(directory, "review.docx"), Buffer.from(docx!, "base64"));
      writeFileSync(join(directory, "review.md"), `# 模拟 IC 评审\n\n${body}\n`);
      writeFileSync(join(directory, "review.txt"), documentText("模拟 IC 评审", body));
    }
  });

  it("splits long CJK paragraphs and table cells without losing their tails", async () => {
    const long = "带宽10MHz，电流10µA；".repeat(170) + "结束标记";
    expect(wrapPresentationText(long).join("")).toBe(long);
    const data = await presentationBase64("长内容", `## 正文\n\n${long}\n\n| 参数 | 内容 |\n|---|---|\n| 测试 | ${long} |`);
    const archive = await JSZip.loadAsync(data, { base64: true });
    const names = Object.keys(archive.files).filter(name => /^ppt\/slides\/slide\d+\.xml$/.test(name));
    expect(names.length).toBeGreaterThan(3);
    const text = (await Promise.all(names.map(name => archive.file(name).async("string")))).map(xml => new DOMParser().parseFromString(xml, "text/xml").documentElement.textContent).join("").replace(/\s/g, "");
    expect(text.match(/结束标记/g)).toHaveLength(2);
    if (process.env.ICWB_CONVERSION_FIXTURES) {
      mkdirSync(process.env.ICWB_CONVERSION_FIXTURES, { recursive: true });
      writeFileSync(join(process.env.ICWB_CONVERSION_FIXTURES, "long-review.pptx"), Buffer.from(data, "base64"));
    }
  });

  it("keeps Markdown content distinct from presentation title and strips text markup safely", () => {
    expect(documentTitleBody("review.md", "# 标题\n\n正文")).toEqual({ title: "标题", body: "正文" });
    const text = documentText("标题", "**重点**\n\n| 参数 | 值 |\n|---|---|\n| 电流 | 10 µA |\n\n<script>bad()</script>");
    expect(text).toContain("重点"); expect(text).not.toContain("**"); expect(text).toContain("电流\t10 µA"); expect(text).not.toContain("bad()");
  });

  it("rejects oversized presentation requests with a clear error", async () => {
    await expect(presentationBase64("长内容", "x".repeat(1_000_001))).rejects.toThrow("过长");
    await expect(presentationBase64("长内容", Array.from({ length: 201 }, (_, i) => `## 页${i}\n\n内容`).join("\n\n"))).rejects.toThrow("200 页");
  });
});
