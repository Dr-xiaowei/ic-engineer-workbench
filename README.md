# 芯智工作台 · IC Engineer Workbench

面向模拟 IC 研发人员的本地 AI 工作台。集中管理项目资料、Datasheet、个人笔记、进度、待办和邮件；普通版默认离线，AI 使用用户自行配置的本地或内网 OpenAI 兼容服务。

当前正式版本：**v1.0.1**。采用 [MIT 许可证](LICENSE)。本项目不集成具体 EDA 工具，不替代工程签核。

源码与双版本下载包已公开发布：[v1.0.1 下载页](https://github.com/Dr-xiaowei/ic-engineer-workbench/releases/tag/v1.0.1)。

## Windows 适配（开发中，尚未验收）

Windows 构建与升级保护正在独立分支开发，尚无通过验收的 Windows 安装包。使用现有界面与功能，普通预览和 Demo 分开存储数据，不替换 macOS 正式版。构建、原生验收、升级恢复及环境状态见 [Windows 验证说明](docs/WINDOWS_VALIDATION.md)。

## 首轮四项升级 · 本地预览（2026-09-25）

已按确认范围实现，尚未发布为新正式版。2026-10-24 获准将开发源码和 Windows 验证流水线上传验证分支；以 [Windows 验证说明](docs/WINDOWS_VALIDATION.md) 的实际结果为准。本地应用与操作说明位于 `artifacts/productivity-preview/`；预览使用独立应用标识，不自动读取或替换 v1.0.1 的工作数据。

| 主入口 | 日常操作 |
| --- | --- |
| 今日 | 继续草稿或阅读、查看今日与最近 30 天逾期待办；进入日历、邮件 |
| 项目 | 关联授权目录、索引资料；用中文词或器件型号检索，保存常用查询 |
| 资料 | 阅读 Datasheet、保留原文证据、整理项目评审笔记 |
| 助手 | 智能对话；设计评审、Datasheet 核查、故障排查、项目周报四个工作模板 |
| 工具 | 科学计算器、格式转换、常用软件 |

推荐流程：在项目检索中打开资料，或在 Datasheet 导入文本型 PDF → 搜索并选中原文 → 选择关联项目 → 整理到评审笔记 → 检查并保存 → 修改标题和日期后确认创建待办。引用保留文件版本、页码和摘录，可跳回原件；原件改动或丢失时提示重新核对，历史摘录保留。非 PDF 文档标为提取文本，不伪造 Word 页码。

笔记、对话、邮件和待办草稿，以及阅读、检索和模板状态自动保存在本机；“已保存到本机”表示工作状态保存，并不代表邮件发送或正式笔记提交。跨页及重启后可继续，保存失败会显示重试。PDF 原件可选文，按页文本提供备用选文入口，也可点击“引用本页”保留当前页完整原文（上限8000字）；工程数值仍需核对完整单位与测试条件。

普通用户直接选择模板并填写少量事实；高级用户可“复制并定制”，调整字段、正文、提示词和默认模型，或通过 JSON 导入导出。内置模板始终保留，重新选择即可回到默认定义；Skills 管理收在进阶选项内。交给助手只填入草稿，仍需人工点击发送。页面右上角可以固定最多五个常用功能。

预览数据库升级为 schema 10。可显式恢复 v1.0.1 备份；新版备份不能恢复到只支持 schema 9 的旧版。备份包含草稿和证据快照，但不包含全部原始工程文件或系统凭据。新功能本机验证与业务提效尚待实测的边界见 [验收报告](COMPLETION_REPORT.md)。

## 下载与快速使用

前往本仓库的 [Releases](https://github.com/Dr-xiaowei/ic-engineer-workbench/releases) 下载 `ic-workbench-v1.0.1-macos-arm64.zip` 或 `ic-workbench-demo-v1.0.1-macos-arm64.zip`。不要把 GitHub 自动生成的 Source code 压缩包当作已编译应用。

1. 本次预编译包适用于 Apple Silicon Mac；其他平台需在目标系统自行构建和验收。下载后用 `shasum -a 256 文件名.zip` 与发布页的 `SHA256SUMS.txt` 比较。
2. 解压，将“芯智工作台.app”拖入个人 Applications 或其他可写目录，双击图标启动。Demo 是独立应用，演示资料随包集中放在“演示资料”目录；无需安装 Node 或 Rust。
3. 普通版没有预置项目、邮箱、模型密钥、笔记或聊天记录。可以先使用计算器、笔记、文档转换和日历；项目管理通过“添加项目”授权本地目录。
4. 如需 AI，在“智能对话”的模型配置中添加本地或内网地址、模型标识与凭据，打开设置中的受控内网开关，再检测连接。不要填写公网地址；密钥不写入项目文件。
5. 如需体验完整流程，先运行 Demo：开启受控连接、检测“内置 Demo 模型”，即可体验合成资料分析与流式对话。Demo 不发送真实邮件、不产生可信工程结论。

macOS 包采用 ad-hoc 本地签名，尚无 Apple Developer ID 公证。下载后的 Gatekeeper 提示和公司终端策略可能阻止直接打开；请先验证来源与校验和，按组织批准的方式放行或从源码构建。不要全局关闭系统安全保护。此限制不同于应用自身能否启动。

## v1.0.1 验收与交付

本轮整理导航、双主题、最小窗口布局，并修复日历待办可访问性、删除失败反馈和备份恢复安全问题。51 项前端、32 项 Rust、4 项 Demo 协议和 1 项实际合成备份恢复检查通过；双版真实恢复、恢复后离线、图文导出/映射、批量异常转换及 Demo 模型已实测。

普通版与 Demo 优化包的启动、退出重启、独立初态及随包 PDF 检查通过。源码标签、发布说明和三项附件已在 GitHub 核对，服务端 SHA-256 与本地一致；补充下载因超时未完成。本地 v1.0.0 应用和 ZIP 已按授权删除，运行数据、备份、Git 及远端历史保留。完整证据和外部环境限制见 [完成度报告](COMPLETION_REPORT.md)。

本地 `artifacts/ui-preview/` 仅保留验收使用的独立调试预览，内部版本仍为 1.0.0；分发请使用上方 v1.0.1 正式下载入口。

## 功能

| 入口 | 能力 |
| --- | --- |
| 工作台与设置 | 最近项目、文档和会话；深浅主题、头像；网络总开关；脱敏审计；备份与恢复 |
| 智能对话 | 多模型、多会话、SSE 流式回答、文档/图片附件、显式 Skills、当前页面授权浮空助手 |
| 项目管理 | 授权目录、增量索引、关键词检索与 AI 候选分析；阶段、里程碑、风险、重点、星标、备注和历史 |
| Datasheet | PDF 分页原件阅读、按页文本、来源约束的 AI 概略，Markdown/PDF 导出 |
| 知识笔记 | 三栏编辑、图文表格、搜索、本地目录映射；MD/TXT/DOCX/PPTX/PDF 导入；MD/PDF/DOC/DOCX 导出 |
| Skills | 可搜索清单、作用、版本、来源、权限；提示词类 Skill 的导入、启停、导出 |
| 格式转换 | PDF、DOCX、PPTX、TXT、Markdown 导入；统一另存 MD/PDF/DOCX/PPTX/TXT，批量预览与默认目录 |
| 邮件助手 | 内网 IMAPS/SMTP、只读缓存与搜索、附件预览/导出、写信/回复/转发与确认发送 |
| 日历与待办 | 系统日期、月视图、项目关联、状态、紧急高亮、双击编辑和应用内提醒 |
| 科学计算器 | 空白初态、直接表达式、常量/函数、DEG/RAD、安全解析及本地历史 |
| 常用软件 | 显式添加、搜索和启动本机应用，拒绝 Shell、参数和 URL |

## 文档互转（本地开发预览）

进入 **工具 → 格式转换 → 文档互转**，选择本地文件，查看 Markdown 预览，再选择导出格式并另存为新文件。无需 AI、联网或安装 Office；已有 PDF/DOCX 解析与笔记导出能力直接复用。笔记也可直接导入 PPTX。

| 输入 | 可导出 |
| --- | --- |
| Markdown（.md/.markdown）、UTF-8 TXT、Word（.docx）、PowerPoint（.pptx）、含文本层的 PDF | Markdown、PDF、DOCX、PPTX、TXT |

这是经 Markdown 中间内容重排的互转，不是原始版式无损转换。PPTX 导入按实际页序保留正文、列表、基本表格和演讲者备注；导出生成可编辑文字与表格，按一级/二级标题、`---` 和内容长度分页，长表格续页。图片、图表、动画、合并单元格和复杂公式不保证保留。工程数值、单位和条件须对照原件。扫描 PDF 需 OCR，旧二进制 `.doc/.ppt` 需先在 Office/LibreOffice 另存为 `.docx/.pptx`；笔记原有 DOC 导出仍为兼容 HTML，不能冒充旧式 DOC 转换。

单次最多20个文件，单文件25 MiB，PPTX最多200页；提取和导出有额外内容上限。默认输出名加“_转换”，后端拒绝覆盖任何已有文件，包括所选原件及其硬链接。取消或失败可重试，无需保存为笔记，不修改数据库或旧个人资料格式。

## 数据与使用边界

- 所有界面资源随应用打包，不依赖 CDN。安装后的本地工具无需外网；首次从源码安装依赖需要网络或组织内部镜像。
- 普通版标识为 `com.icengineer.workbench`，Demo 为 `com.icengineer.workbench.demo`；运行数据位于各自的系统应用数据目录。两者不共享数据库或界面存储。
- 正式 v1 标识与开发期标识不同，不自动带入开发测试数据；如需迁移自己保存的数据，先在旧版导出备份，再在新版本显式恢复并复核。备份不含系统凭据，恢复后离线。
- 内网模型和真实邮箱的证书、认证及公司网络策略必须由使用方联调；本仓库的合成协议测试不代表公司服务器已通过验收。
- PDF 扫描件不提供 OCR；复杂表格、公式、图形与导入 Word 的复杂排版不能保证完整还原。AI 参数须回到原件核对页码、单位和条件。
- 笔记以 Markdown 表达简单排版；DOC 是 Word 兼容 HTML，不是旧版二进制 DOC。优先用 DOCX 交换图文表格，用 Markdown 保留可编辑源；PDF 为简洁阅读版，不宣称与 Word 像素一致。
- 提醒只在应用运行且相关任务已载入时展示，不是系统后台闹钟；不读取 Apple 日历。邮箱首版不支持服务端删除、移动或已读标记同步。
- Skills 不执行上传脚本；软件管理不是通用命令执行器；项目文件只读。用户自己添加的本地地址属于运行数据，不是交付包中的开发路径。

## 从源码开发与构建

### 本地多项目工作区

本项目统一位于 `company-intranet-workbench/`，其中包含独立 `.git/`、`app/`、`demo/`、`docs/` 与 `release/`。后续其他平台使用同级目录，不共用本项目仓库。本文档中的路径均相对于本项目根目录。

外层工作区的 PROJECT.md 和 AGENTS.md 管理通用开发规则；本平台的全部功能需求、技术与发布要求在本目录内维护。独立克隆本仓库时，按仓库内的管理文件即可继续工作，无需依赖外层文件。

如果终端位于外层多项目工作区，先执行 `cd company-intranet-workbench`；如果已经打开本项目，直接进入 `app/desktop`。迁移不改变 GitHub 仓库地址、应用标识或系统运行数据位置。

#本轮预览修复了 Skill 启停后聊天选择同步、Demo 发送结果提示和模型面板关闭入口。旧预览/Demo 已存凭据需在各自应用中重新填写；正式普通版原凭据服务名保持兼容，数据库和备份仍不含密钥。生成请求上限为120秒，连接检测限制保持不变。真实小模型已跑通文档/图片和业务调用，但工程内容质量未通过；详见 [本轮验收记录](COMPLETION_REPORT.md)。

## 安装与检查

需要 Node.js 22+、pnpm 11.19.0、Rust stable，以及目标平台的 Tauri 2 编译前提（macOS 需要 Xcode Command Line Tools）。依赖版本以两份锁文件为准。

```sh
git clone https://github.com/Dr-xiaowei/ic-engineer-workbench.git
cd ic-engineer-workbench/app/desktop
pnpm install --frozen-lockfile
pnpm tauri dev
```

浏览器 `pnpm dev` 仅预览前端；完整文件、数据库、模型与邮箱功能必须使用 `pnpm tauri dev` 的桌面窗口。

```sh
# 自动回归与类型/生产构建
pnpm test
pnpm test:demo-model
cargo test --manifest-path src-tauri/Cargo.toml --locked
cargo fmt --manifest-path src-tauri/Cargo.toml --check
pnpm build

# macOS 普通版和 Demo 双包（含路径净化与签名校验）
pnpm build:clickable
# 生成双 ZIP 与 SHA-256 清单（不会覆盖已有下载包）
node scripts/package-downloads.mjs
```

双包位于 `release/v1.0.1/apps/normal/` 和 `release/v1.0.1/apps/demo/`。其他目标系统使用 `pnpm bundle:release` 并按照 Tauri 平台文档配置签名和打包；仓库不宣称未实测平台已可交付。

双包构建目录按 `package.json` 中的版本号生成，若该目录已有产物则拒绝覆盖。后续迭代先在独立开发分支对齐应用版本和 Tauri 配置；确认新正式版本后按 [发布流程](PROJECT.md) 交付。日常源码验证使用 `pnpm tauri build --debug --no-bundle`，不重建历史正式包。

开发 Demo 可运行 `ICWB_APP_MODE=demo pnpm tauri dev --config src-tauri/tauri.demo.conf.json`。仅协议测试需要的可选模拟服务为 `pnpm demo:model`，它只监听回环地址。正常 Demo 不需要另开模型服务。

## 项目资料与下一轮迭代

查阅顺序：使用与下载看本文件；需求和正式发布约束看 PROJECT；待办与交接看 TASKS；验收证据看 COMPLETION_REPORT；开发过程与经验看复盘。CHANGELOG 只保留重要变化，细小修改不新增迭代记录。

后续每个经用户明确确认的正式版本，均按 [PROJECT 发布规范](PROJECT.md) 上传到同一 GitHub 仓库统一管理；既有正式标签和远端安装包保持不变。

- [完成度与验收报告](COMPLETION_REPORT.md)：逐项功能、证据及限制。
- [开发复盘与迭代建议](docs/RETROSPECTIVE.md)：本次经验、发现的问题与下一轮重点。
- [项目案例 Markdown](docs/PROJECT_CASE.md) / [Word](docs/PROJECT_CASE.docx)：按实践模板整理，未虚构业务收益。
- [产品要求](PROJECT.md)、[任务交接](TASKS.md)、[版本记录](CHANGELOG.md)、[开源参考](REFERENCES.md)。
- [Demo 说明](demo/README.md)及[目标环境验收清单](demo/ACCEPTANCE_CHECKLIST.md)。
- [第三方许可证说明](app/desktop/THIRD_PARTY_NOTICES.md)。

反馈问题时请提供版本、系统/架构、复现步骤和脱敏错误说明。请勿上传真实芯片资料、内部地址、账户、密钥或含个人信息的备份。
