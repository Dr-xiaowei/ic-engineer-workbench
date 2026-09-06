import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import MarkdownMessage from "./MarkdownMessage";

describe("MarkdownMessage", () => {
  it("renders engineering Markdown while keeping referenced resources read-only", () => {
    render(
      <MarkdownMessage
        text={[
          "## 检查结论",
          "",
          "- 偏置电流满足约束",
          "- 需要复核温度角",
          "",
          "```spice",
          ".param ibias=10u",
          "```",
          "",
          "[内网说明](http://10.0.0.8/docs)",
          "",
          "![远程波形](https://example.com/waveform.png)",
        ].join("\n")}
      />,
    );

    expect(screen.getByRole("heading", { name: "检查结论", level: 2 })).toBeInTheDocument();
    expect(screen.getByText("偏置电流满足约束")).toBeInTheDocument();
    expect(screen.getByText(".param ibias=10u")).toBeInTheDocument();
    expect(screen.queryByRole("link")).not.toBeInTheDocument();
    expect(screen.getByText("http://10.0.0.8/docs")).toBeInTheDocument();
    expect(screen.queryByRole("img")).not.toBeInTheDocument();
    expect(screen.getByText("图片引用已阻止：远程波形")).toBeInTheDocument();
  });

  it("drops raw HTML and never exposes a dangerous Markdown link", () => {
    render(
      <MarkdownMessage text={'<img src="x" onerror="alert(1)">\n\n[危险链接](javascript:alert(1))'} />,
    );

    expect(screen.queryByRole("img")).not.toBeInTheDocument();
    expect(screen.queryByRole("link")).not.toBeInTheDocument();
    expect(screen.getByText("危险链接")).toBeInTheDocument();
    expect(screen.queryByText(/onerror/)).not.toBeInTheDocument();
  });
});
