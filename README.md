# 芯智工作台 · IC Engineer Workbench

面向模拟 IC 研发人员的本地 AI 工作台。集中管理项目资料、Datasheet、个人笔记、进度、待办和邮件；普通版默认离线，AI 使用用户自行配置的本地或内网 OpenAI 兼容服务。

首个正式版本：**v1.0.0**。采用 [MIT 许可证](LICENSE)。本项目不集成具体 EDA 工具，不替代工程签核。

源码与双版本下载包已公开发布：[v1.0.0 下载页](https://github.com/Dr-xiaowei/ic-engineer-workbench/releases/tag/v1.0.0)。

## 下载与快速使用

前往本仓库的 [Releases](https://github.com/Dr-xiaowei/ic-engineer-workbench/releases) 下载 `ic-workbench-v1.0.0-macos-arm64.zip` 或 `ic-workbench-demo-v1.0.0-macos-arm64.zip`。不要把 GitHub 自动生成的 Source code 压缩包当作已编译应用。

1. 本次预编译包适用于 Apple Silicon Mac；其他平台需在目标系统自行构建和验收。下载后用 `shasum -a 256 文件名.zip` 与发布页的 `SHA256SUMS.txt` 比较。
2. 解压，将“芯智工作台.app”拖入个人 Applications 或其他可写目录，双击图标启动。Demo 是独立应用，演示资料随包集中放在“演示资料”目录；无需安装 Node 或 Rust。
3. 普通版没有预置项目、邮箱、模型密钥、笔记或聊天记录。可以先使用计算器、笔记、文档转换和日历；项目管理通过“添加项目”授权本地目录。
4. 如需 AI，在“智能对话”的模型配置中添加本地或内网地址、模型标识与凭据，打开设置中的受控内网开关，再检测连接。不要填写公网地址；密钥不写入项目文件。
5. 如需体验完整流程，先运行 Demo：开启受控连接、检测“内置 Demo 模型”，即可体验合成资料分析与流式对话。Demo 不发送真实邮件、不产生可信工程结论。

macOS 包采用 ad-hoc 本地签名，尚无 Apple Developer ID 公证。下载后的 Gatekeeper 提示和公司终端策略可能阻止直接打开；请先验证来源与校验和，按组织批准的方式放行或从源码构建。不要全局关闭系统安全保护。此限制不同于应用自身能否启动。

## 功能

| 入口 | 能力 |
| --- | --- |
| 工作台与设置 | 最近项目、文档和会话；深浅主题、头像；网络总开关；脱敏审计；备份与恢复 |
| 智能对话 | 多模型、多会话、SSE 流式回答、文档/图片附件、显式 Skills、当前页面授权浮空助手 |
| 项目管理 | 授权目录、增量索引、关键词检索与 AI 候选分析；阶段、里程碑、风险、重点、星标、备注和历史 |
| Datasheet | PDF 分页原件阅读、按页文本、来源约束的 AI 概略，Markdown/PDF 导出 |
| 知识笔记 | 三栏编辑、图文表格、搜索、本地目录映射；MD/TXT/DOCX/PDF 导入；MD/PDF/DOC/DOCX 导出 |
| Skills | 可搜索清单、作用、版本、来源、权限；提示词类 Skill 的导入、启停、导出 |
| 格式转换 | PDF、DOCX、TXT、Markdown 本地转换、批量预览及默认导出目录 |
| 邮件助手 | 内网 IMAPS/SMTP、只读缓存与搜索、附件预览/导出、写信/回复/转发与确认发送 |
| 日历与待办 | 系统日期、月视图、项目关联、状态、紧急高亮、双击编辑和应用内提醒 |
| 科学计算器 | 空白初态、直接表达式、常量/函数、DEG/RAD、安全解析及本地历史 |
| 常用软件 | 显式添加、搜索和启动本机应用，拒绝 Shell、参数和 URL |

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

如果终端位于外层多项目工作区，先执行 `cd company-intranet-workbench`；如果已经打开本项目，直接进入 `app/desktop`。迁移不改变 GitHub 仓库地址、应用标识或系统运行数据位置。

### 安装与检查

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

双包位于 `release/v1.0.0/apps/normal/` 和 `release/v1.0.0/apps/demo/`。其他目标系统使用 `pnpm bundle:release` 并按照 Tauri 平台文档配置签名和打包；仓库不宣称未实测平台已可交付。

双包构建目录按 `package.json` 中的版本号生成，若该目录已有产物则拒绝覆盖。后续迭代先在独立开发分支对齐应用版本和 Tauri 配置；确认新正式版本后按 [发布流程](PROJECT.md) 交付。日常源码验证使用 `pnpm tauri build --debug --no-bundle`，不重建历史正式包。

开发 Demo 可运行 `ICWB_APP_MODE=demo pnpm tauri dev --config src-tauri/tauri.demo.conf.json`。仅协议测试需要的可选模拟服务为 `pnpm demo:model`，它只监听回环地址。正常 Demo 不需要另开模型服务。

## 项目资料与下一轮迭代

查阅顺序：使用与下载看本文件；需求和正式发布约束看 PROJECT；待办与交接看 TASKS；验收证据看 COMPLETION_REPORT；开发过程与经验看复盘。CHANGELOG 只保留重要变化，细小修改不新增迭代记录。

后续每个经用户明确确认的正式版本，均按 [PROJECT 发布规范](PROJECT.md) 上传到同一 GitHub 仓库统一管理；本次文档整理不改动已发布的 v1.0.0 标签和安装包。

- [完成度与验收报告](COMPLETION_REPORT.md)：逐项功能、证据及限制。
- [开发复盘与迭代建议](docs/RETROSPECTIVE.md)：本次经验、发现的问题与下一轮重点。
- [项目案例 Markdown](docs/PROJECT_CASE.md) / [Word](docs/PROJECT_CASE.docx)：按实践模板整理，未虚构业务收益。
- [产品要求](PROJECT.md)、[任务交接](TASKS.md)、[版本记录](CHANGELOG.md)、[开源参考](REFERENCES.md)。
- [Demo 说明](demo/README.md)及[目标环境验收清单](demo/ACCEPTANCE_CHECKLIST.md)。
- [第三方许可证说明](app/desktop/THIRD_PARTY_NOTICES.md)。

反馈问题时请提供版本、系统/架构、复现步骤和脱敏错误说明。请勿上传真实芯片资料、内部地址、账户、密钥或含个人信息的备份。
