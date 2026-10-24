export type WorkTemplate = { schemaVersion: 1; id: string; name: string; description: string; fields: { key: string; label: string; placeholder: string }[]; body: string; prompt: string; defaultEndpointId: string; permissions: ["prompt"] };
const prompt = "你是模拟 IC 工程评审助手。只根据用户提供的资料工作。保留单位、测试条件、来源文件和页码；无依据时标明待确认。区分原文事实、推断和建议，不捏造仿真结果、模型角或工艺参数。给出可执行的人工复核清单，不自动写工程文件或创建任务。";
const template = (id: string, name: string, description: string, fields: WorkTemplate["fields"], body: string): WorkTemplate => ({ schemaVersion: 1, id, name, description, fields, body, prompt, defaultEndpointId: "", permissions: ["prompt"] });
const field = (key: string, label: string, placeholder: string) => ({ key, label, placeholder });
export const builtInTemplates: WorkTemplate[] = [
  template("builtin-review", "设计评审", "把设计目标、证据和风险整理成可讨论的评审记录。", [field("project", "项目 / 模块", "如：低功耗带隙基准"), field("goal", "指标与评审目标", "电压、精度、功耗、温度范围及测试条件"), field("evidence", "已有证据与疑问", "粘贴脱敏数据或原文摘录，保留出处")], "# {{project}} 设计评审\n\n## 目标与边界\n{{goal}}\n\n## 已有证据与疑问\n{{evidence}}\n\n## 结论（待人工复核）\n- 达标项：\n- 风险项：\n- 缺失证据：\n\n## 后续行动\n- [ ] 复核 PVT / 启动 / 稳定性边界及来源\n- [ ] 明确责任人和复核日期"),
  template("builtin-datasheet", "Datasheet 核查", "核对数值、单位和测试条件，分清典型值与保证值。", [field("part", "器件 / 型号", "如：ZXW-100"), field("question", "本次选型或核查问题", "如：1 kHz 下 PSRR 的保证条件"), field("evidence", "参数与来源", "保留 min / typ / max、单位、条件和页码")], "# {{part}} Datasheet 核查\n\n## 核查问题\n{{question}}\n\n## 原文证据\n{{evidence}}\n\n| 参数 | Min | Typ | Max | 单位 | 测试条件 | 来源页 |\n|---|---|---|---|---|---|---|\n| 待核对 | | | | | | |\n\n## 判断（待复核）\n- 工作条件与绝对最大额定值分别核对\n- 典型曲线是否支持本次使用条件：\n- [ ] 复核规格表、曲线和脚注的一致性"),
  template("builtin-debug", "故障排查", "按现象、证据、假设和验证步骤推进，避免反复试错。", [field("module", "模块 / 电路", "如：LDO 环路"), field("symptom", "现象及复现条件", "输入、电源、负载、温度、版本及发生频率"), field("tried", "已做检查与结果", "说明变化量及测量条件，不只写“正常”")], "# {{module}} 故障排查\n\n## 现象与复现条件\n{{symptom}}\n\n## 已做检查\n{{tried}}\n\n| 假设 | 依据 / 反证 | 下一步验证 | 判定标准 |\n|---|---|---|---|\n| 待建立 | | | |\n\n## 后续行动\n- [ ] 固定复现条件和基线版本\n- [ ] 每次只改变一个变量并记录结果\n- [ ] 复核修复后的边界条件"),
  template("builtin-weekly", "项目周报", "用事实说明进展、风险和下一步，减少重复整理。", [field("project", "项目与报告周期", "如：AFE 项目 · 第 39 周"), field("progress", "本周完成与证据", "完成事项、指标变化、记录出处"), field("risks", "风险、依赖与下周计划", "需要谁协助、何时完成、尚缺什么")], "# {{project}} 周报\n\n## 本周完成\n{{progress}}\n\n## 风险、依赖与下周计划\n{{risks}}\n\n## 待确认事项\n- 决策 / 所需支持：\n- [ ] 确认下一步责任人和截止日期"),
];
export function validateTemplate(value: unknown): WorkTemplate {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new Error("模板必须是一个 JSON 对象。");
  const t = value as WorkTemplate;
  const allowed = ["schemaVersion", "id", "name", "description", "fields", "body", "prompt", "defaultEndpointId", "permissions"];
  if (Object.keys(t).some(key => !allowed.includes(key)) || t.schemaVersion !== 1 || JSON.stringify(t.permissions) !== '["prompt"]') throw new Error("仅支持 schemaVersion 1 的提示词模板；不接受脚本、命令或额外权限。");
  for (const [key, limit] of [["id", 100], ["name", 80], ["description", 400], ["body", 16000], ["prompt", 8000], ["defaultEndpointId", 100]] as const) if (typeof t[key] !== "string" || t[key].length > limit || t[key].includes("\0")) throw new Error(`模板 ${key} 无效或过长。`);
  if (!t.name.trim() || !t.body.trim() || !t.id.trim() || !Array.isArray(t.fields) || !t.fields.length || t.fields.length > 12) throw new Error("模板需名称、正文及 1–12 个填写字段。");
  const keys = new Set<string>();
  for (const f of t.fields) {
    if (!f || typeof f !== "object" || Object.keys(f).some(k => !["key", "label", "placeholder"].includes(k)) || !/^[a-zA-Z][a-zA-Z0-9_]{0,39}$/.test(f.key) || keys.has(f.key) || typeof f.label !== "string" || !f.label.trim() || f.label.length > 80 || typeof f.placeholder !== "string" || f.placeholder.length > 200) throw new Error("字段需唯一的英文标识、中文名称和填写提示。");
    keys.add(f.key);
  }
  for (const match of `${t.body}\n${t.prompt}`.matchAll(/\{\{([^{}]+)\}\}/g)) if (!keys.has(match[1])) throw new Error(`占位符 ${match[1]} 没有对应字段。`);
  return structuredClone(t);
}
export function fillTemplate(t: WorkTemplate, values: Record<string, string>, text = t.body) { return text.replace(/\{\{([^{}]+)\}\}/g, (_, key: string) => values[key]?.trim() || "待补充"); }
