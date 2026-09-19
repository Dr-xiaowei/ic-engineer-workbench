# 🔗 GitHub 参考项目记录

> **用法：** 搜索、比较和采用的候选在此留档；重点候选按下列十项标准分析。保留可追溯证据和结论，不长期保留下载副本。以下既有清单是各行日期的历史记录，尚未按 2026-09-19 新标准全面复核，不据此宣称候选当前仍在维护或安全。

## 参考规则

- 开发前实时搜索已有开源项目、Skill 或类似方案，交叉比较多个候选；优先近期仍维护、文档完整、社区使用较多且匹配需求的方案，不以高 Star 为唯一门槛。
- 先只读查看仓库、文档、Release、Issue/PR、提交、许可和关键脚本；记录检索词、筛选/排除原因及访问日期。初筛推荐与安装实测分开，未经用户确认不运行候选代码。
- 下载内容存放在隔离目录 `reference-sandbox/`，未经检查不得运行。
- 发现恶意或可疑内容时停止执行，先在隔离区移除或修复风险，再重新检查；未通过检查的内容不得用于项目。
- 每版开发结束后删除隔离目录内的下载副本，只保留本文件中的记录。

## 重点候选的十项分析标准

每个重点候选注明仓库/官方文档链接、调研日期、所检查的 tag 或 commit SHA。每项给出“事实或结果 + 证据链接 + 未验证限制”；读取失败或资料缺失写“未核实”，不能填零或默认通过。

| 项 | 必须回答 | 核对与记录方式 |
| --- | --- | --- |
| 1. 维护状态 | 现在是否有人维护？最近一次更新何时？ | 分别记录最近有意义的代码提交日期、最近 Release 日期、维护者回应；标明时区，不只依赖仓库 pushed 时间或机器人更新 |
| 2. 社区与弃用迹象 | Star、Fork、Issue、提交情况如何？是否疑似废弃？ | 记录当日 Star/Fork、开放 Issue/PR（区分二者）、近期提交频率与作者/维护者活动、问题响应样本；检查 archived、deprecated、继任仓库及长期无人回应。说明判断依据，不能仅因长时间无提交或 Issue 多就断言废弃 |
| 3. 普通用户部署 | 不懂代码的用户安装是否麻烦？ | 检查安装包/一键安装、图形配置、系统/架构、管理员权限、命令行与构建步骤、升级卸载和常见障碍；区分“文档评估”与“已实测”，给出易/中/难及原因 |
| 4. 环境与外部服务 | 需要安装或注册什么？ | 列出运行时、版本、包管理器、数据库、容器、硬件/模型资源、第三方账号/API/费用；区分必需/可选、首次下载/运行联网及可否本地替代 |
| 5. 直接可用 | 哪些功能无需代码修改即可满足需求？ | 逐项映射用户核心需求，写清可用版本、启用条件与证据；有界面或 README 宣称不等于功能实测通过 |
| 6. 可二次开发 | 哪些相近能力可复用改造？ | 指明模块、接口/插件点、架构与界面适配、数据格式及许可条件；评估改造和后续上游升级成本，不把“可参考思路”写成“可直接复制” |
| 7. 需重新开发 | 哪些能力缺失或不适配？ | 列出缺口、改写原因、依赖与风险，区分配置、适配、新增和重写；不虚构工作量或兼容性 |
| 8. 明显安全风险 | 是否读取秘密、执行不明代码或外传？ | 静态检查 Cookie、账号密码、API Key、SSH Key、环境变量读取；安装/启动/更新脚本、动态下载执行、混淆代码、权限、依赖与漏洞公告、遥测和陌生服务器。记录文件/行或链接、目的、必要性、去向、能否禁用及风险；不把正常所需读取自动判为恶意，也不把未发现问题当成安全保证 |
| 9. 许可证 | 是否允许修改、二次开发、商用？有什么条件？ | 以对应版本 LICENSE/NOTICE 和官方条款为证据，分别说明允许/附条件/不明确；检查署名、源码提供/同许可、分发或网络服务条件、专利/商标，以及依赖、模型权重、素材的独立许可；无许可证不等于允许复用，复杂问题需法律复核 |
| 10. 综合适配 | 哪个项目最适合本次需求？ | 比较核心覆盖、普通用户部署、维护、改造成本、安全、许可及本地内网适配；给出首选、备选和排除理由，说明关键未知项，不仅按 Star 排序 |

### 调研交付与停止点

- **候选比较：** 用同一组用户需求比较重点候选。高风险或许可证不清的候选先标记不宜采用；安全初查只覆盖已查看的版本和文件，不构成全量审计。
- **A/B/C 决策：** 分别说明 A 直接使用、B 基于现有项目二次开发、C 完全从零开发的适配、部署、成本和风险；明确“推荐 A/B/C”及原因、对应项目/版本、其他方案不优先的理由与可能改变结论的条件。没有合适候选时保留检索和排除依据，不伪造最优项目。
- **最小 MVP：** 根据用户最终需求写明一个核心流程、必要输入输出、最少功能、复用与开发边界、环境/服务、验收标准和明确不做项；其余功能留待后续确认，不因参考项目功能多而全部纳入。
- **等待确认：** 把比较、推荐和 MVP 交给用户，等待确认方案与范围后再实施；不能把“已经调研”当作开发授权。

上述结构在本文件中按实际候选补充即可，不另建重复模板文件。本次仅升级规范，未重新搜索或复核历史候选，也未作出具体项目推荐。

## 项目清单

| 项目名称 | GitHub 网址 | 调研日期 / Star | 参考内容 | 许可证 | 安全检查 | 使用版本 |
| --- | --- | --- | --- | --- | --- | --- |
| Microsoft MarkItDown | https://github.com/microsoft/markitdown | 2026-09-05 / 约 176.8k | PDF、Word、文本等文件转 Markdown；按格式拆分可选依赖、默认关闭第三方插件、优先调用窄权限本地转换接口 | MIT；可选插件及其依赖需单独核验 | 官方明确提示转换器继承进程文件权限，应限制输入路径、URI 和调用面；仅完成静态初审，未下载或运行 | 未使用 |
| AppFlowy | https://github.com/AppFlowy-IO/AppFlowy | 2026-09-05 / 约 74.8k | 本地优先工作区、模块化导航、块式内容与清楚的信息层级；适合参考工作台和主题体验 | AGPL-3.0 | 仅核验官方仓库、许可证和公开说明；未下载或运行，代码暂不复用 | 未使用 |
| Docling | https://github.com/docling-project/docling | 2026-09-05 / 约 65.7k | PDF 版面、阅读顺序、表格和公式解析；统一文档中间表示；本地/隔离环境运行及 Markdown、JSON 导出 | MIT；模型权重许可证需逐项核验 | 官方仓库提供安全策略；本地执行适合敏感资料，但 OCR/VLM 模型及依赖仍需单独审计；未下载或运行 | 未使用 |
| AnythingLLM | https://github.com/Mintplex-Labs/anything-llm | 2026-09-05 / 约 65.6k | 本地优先的文档导入、工作区、检索增强问答、模型/向量库适配与引用式交互 | MIT | 官方说明可离线运行且遥测可关闭；外部模型、嵌入服务及首次模型资源仍可能联网；未下载或运行 | 未使用 |
| Cherry Studio | https://github.com/CherryHQ/cherry-studio | 2026-09-05 / 约 51.4k | 桌面端多模型配置、会话与助手、文档处理、全局搜索、明暗主题和透明窗口等交互思路 | AGPL-3.0；另有商业许可 | 官方仓库提供安全策略；仅完成静态初审，未下载或运行；闭源内网产品不直接复用其代码 | 未使用 |
| Jan | https://github.com/janhq/jan | 2026-09-05 / 约 44.1k | Tauri 桌面壳、TypeScript/Rust 分层、本地与云端模型统一接入、OpenAI 兼容接口、扩展和 MCP 思路 | Apache-2.0 | 官方仓库提供安全策略；本地模型文件、扩展权限和模型许可证需在采用时复核；未下载或运行 | 未使用 |
| Mailspring | https://github.com/Foundry376/Mailspring | 2026-09-05 / 约 17.8k | 三栏邮箱布局、统一收件箱、编辑器、主题/插件架构，以及桌面 UI 与本地同步引擎分离 | GPL-3.0 | 官方仓库提供安全策略并说明凭据不上传；邮件解析、附件和凭据存储仍属高风险面；未下载或运行，代码暂不复用 | 未使用 |
| Tauri | https://github.com/tauri-apps/tauri | 2026-09-05 / 约 110.0k | 系统 WebView + Rust 桌面架构、按窗口 capability、外部 sidecar、签名更新及跨平台打包 | MIT / Apache-2.0 | 官方仓库含安全策略、审计与供应链资料；依赖已锁定并通过桌面启动验证 | 2.11.5 |
| Electron | https://github.com/electron/electron | 2026-09-05 / 约 122.4k | 对照评估 Chromium 多进程、渲染沙箱、上下文隔离和桌面分发生态 | MIT | 官方安全清单要求隔离远程内容、限制 IPC、启用 CSP/沙箱并及时升级；本项目暂不采用 | 未使用 |
| reqwest | https://github.com/seanmonstar/reqwest | 2026-09-05 / 成熟 Rust HTTP 客户端 | Rust 后端访问 OpenAI 兼容内网端点；使用 rustls、连接/响应超时、禁止重定向并限制响应体 | MIT OR Apache-2.0 | 核验官方仓库、Cargo 元数据和许可证；锁文件校验及本机合成 HTTP 协议测试通过 | 0.13.4 |
| keyring-rs | https://github.com/hwchen/keyring-rs | 2026-09-05 / 跨平台系统凭据库 | API 密钥写入 macOS Keychain、Windows Credential Manager 或 Linux Secret Service，不进入前端配置 | MIT OR Apache-2.0 | 核验官方仓库、功能声明和许可证；依赖锁定与编译测试通过，真实凭据生命周期待签名安装包验证 | 3.6.3 |
| rusqlite | https://github.com/rusqlite/rusqlite | 2026-09-05 / 约 4.3k | Rust 类型化 SQLite 会话仓储、事务写入和 schema 版本迁移；bundled SQLite 降低内网客户端的系统依赖 | MIT；bundled SQLite 为 public domain | 核验官方仓库、发布记录、Cargo 元数据和许可证；未下载或运行仓库源码，锁定依赖、内存数据库往返和边界测试通过 | 0.40.2 |
| rust-base64 | https://github.com/marshallpierce/rust-base64 | 2026-09-05 / 约 732 | 在 Rust 边界严格解码图片 Data URL，用于核对声明大小和文件签名 | MIT OR Apache-2.0 | 核验官方仓库、发布说明和 Cargo 元数据；采用已锁定版本，合成图片的有效、伪装类型和大小检查通过 | 0.22.1 |
| react-markdown | https://github.com/remarkjs/react-markdown | 2026-09-05 / 约 15.7k | 将不可信模型 Markdown 转成 React 元素；采用组件覆盖控制链接与图片，不使用 `dangerouslySetInnerHTML` 或原始 HTML 插件 | MIT | 官方仓库有安全策略并明确默认安全边界；已锁定依赖，危险协议、原始 HTML及资源引用测试和生产构建通过 | 10.1.0 |
| remark-gfm | https://github.com/remarkjs/remark-gfm | 2026-09-05 / 约 1.2k | 为模型工程回答补充表格、删除线、任务列表和自动链接等 GFM 语法 | MIT | 仅启用语法树转换，不启用原始 HTML；已锁定依赖并通过前端测试与生产构建 | 4.0.1 |
| rehype-sanitize | https://github.com/rehypejs/rehype-sanitize | 2026-09-05 / 约 212 | 在 Markdown 插件链末端按默认安全 schema 清洗 HTML 抽象语法树，作为模型输出的纵深防御 | MIT | 官方仓库有安全策略并提醒清洗器应置于最后一个不可信变换之后；已锁定依赖并通过 XSS 边界测试 | 6.0.0 |
| printpdf | https://github.com/fschutt/printpdf | 2026-09-05 / 活跃 Rust PDF 库 | 生成嵌入中文字体、可检索且分页的概略版 Datasheet PDF | MIT | 核验官方仓库、许可证和锁定包；合成三页 PDF 已完成文本抽取、页尺寸和逐页渲染复核 | 0.12.8 |
| pdf-extract | https://github.com/jrmuizel/pdf-extract | 2026-09-05 / 成熟 Rust PDF 文本抽取库 | 在本机提取含文本层 PDF，并与页码边界组合为统一文档记录 | MIT | 仅处理用户明确选择且通过签名/大小检查的 PDF；合成双页样本测试通过 | 0.12.0 |
| quick-xml | https://github.com/tafia/quick-xml | 2026-09-05 / 约 1.5k | 以事件流读取 DOCX 中受限的 WordprocessingML，不执行宏或嵌入对象 | MIT | 锁定版本并以合成 DOCX 验证标题、段落与表格文本；输入先经过 ZIP 条目和大小限制 | 0.41.0 |
| zip2 | https://github.com/zip-rs/zip2 | 2026-09-05 / 活跃 Rust ZIP 库 | 只读打开 DOCX 容器并读取固定白名单条目 | MIT | 不解压到磁盘，不跟随归档路径；限制文件、条目与展开文本大小 | 8.6.0 |
| Noto CJK | https://github.com/notofonts/noto-cjk | 2026-09-05 / 约 21k | 为 Datasheet PDF 导出嵌入简体中文字体，避免依赖目标电脑字体 | SIL OFL 1.1 | 字体与 OFL 原文随源码保留；合成 PDF 渲染和文本搜索通过 | NotoSansCJKsc-Regular |
| async-imap | https://github.com/async-email/async-imap | 2026-09-05 / 活跃异步 IMAP 客户端 | IMAPS 登录、文件夹枚举、只读 `EXAMINE`、UID 搜索和 `BODY.PEEK[]` 同步 | MIT / Apache-2.0 | DNS 结果必须全部属于内网；系统 TLS 验证、超时、邮件数量和原文大小上限均在 Rust 边界执行 | 0.11.3 |
| lettre | https://github.com/lettre/lettre | 2026-09-05 / 约 3.5k | SMTPS 或强制 STARTTLS、认证预检、纯文本/附件 MIME 构建与发送 | MIT | 仅在网络总开关开启且用户二次确认后发送；地址、附件类型/大小和内网解析均复核 | 0.11.23 |
| mail-parser | https://github.com/stalwartlabs/mail-parser | 2026-09-05 / 活跃 MIME 解析库 | 提取主题、地址、日期、纯文本正文和受限附件供本地缓存与 FTS | MIT / Apache-2.0 | 单封原文、正文、附件数量和大小有硬上限；正文按纯文本展示，不执行 HTML | 0.11.8 |
| SiliconCompiler | https://github.com/siliconcompiler/siliconcompiler | 2026-09-06 / 约 1.2k | IC 项目清单、可追溯 manifest、flowgraph 阶段和构建摘要；用于定义项目目录、阶段与来源关联，不引入 EDA 执行能力 | Apache-2.0 | 官方仓库含安全策略；仅阅读文档，未下载或执行代码 | 未使用 |
| OpenROAD Flow Scripts | https://github.com/The-OpenROAD-Project/OpenROAD-flow-scripts | 2026-09-06 / 约 642 | 参考从综合、布局、CTS、布线到 signoff 的清晰阶段表达和允许人工干预的阶段控制；本产品不接入其 EDA 工具链 | BSD-3-Clause；所含工具/平台各自授权 | 仅阅读官方仓库；未下载、安装或执行第三方流程 | 未使用 |
| OpenProject | https://github.com/opf/openproject | 2026-09-06 / 约 15.2k | 项目日期、work package、里程碑、风险/状态、完成百分比和截止提醒的信息模型 | GPL-3.0 | 只参考公开交互和数据字段，不复用 GPL 代码；未下载或运行 | 未使用 |
| FullCalendar React | https://github.com/fullcalendar/fullcalendar-react | 2026-09-06 / 约 2.5k | React 月视图、声明式事件源、日期导航和事件内容分层；首版用原生组件实现相同信息结构以减少依赖 | MIT | 官方仓库与示例已静态阅读；不下载代码，不引入额外运行权限 | 未使用 |
| Qalculate / libqalculate | https://github.com/Qalculate/libqalculate | 2026-09-06 / 约 2.6k | 科学函数、常量、角度/精度和可理解错误的计算器能力边界 | GPL-2.0 | 只参考功能范围；不复制或链接 GPL 实现，自研受限解析器并禁止 `eval` | 未使用 |
| Wox | https://github.com/Wox-launcher/Wox | 2026-09-06 / 约 27.3k | 本地、快速、可搜索的软件启动体验和卡片/键盘入口 | GPL-3.0 | 只参考产品交互；本项目拒绝其命令/插件扩展面，不复用代码 | 未使用 |
| PDF.js | https://github.com/mozilla/pdf.js | 2026-09-06 / 约 53.4k | 离线原件分页、缩放与 Canvas 阅读 | Apache-2.0 | 官方 npm 包禁用安装脚本；worker、字体与 CMap 本地打包；不执行 PDF JS，不加载在线查看器；npm 生产审计无已知公告 | pdfjs-dist 6.3.289 |
| Joplin | https://github.com/laurent22/joplin | 2026-09-06 / 约 55k | 离线优先笔记、笔记本/列表/编辑区、Markdown 事实源、图片/附件、搜索与导入导出 | AGPL-3.0 | 官方仓库含安全策略；只参考公开产品与数据模型，不复用 AGPL 代码 | 未使用 |
| Tiptap | https://github.com/ueberdosis/tiptap | 2026-09-06 / 约 38.3k | 无头富文本编辑、工具栏、表格、图片和扩展式节点；用于定义简单排版交互 | MIT；Pro 扩展另行授权 | 官方仓库含安全策略；首轮不引入依赖或付费扩展，仅采用 Markdown 工具栏思路 | 未使用 |
| docx | https://github.com/dolanmiu/docx | 2026-09-06 / 约 5.7k | 笔记段落、表格、图片的结构化 Word 导出 | MIT | 官方 npm 包禁用安装脚本；仅从安全 Markdown DOM 生成，不加载远程图片；实际导出与渲染检查通过 | 9.7.1 |
| Plane | https://github.com/makeplane/plane | 2026-09-06 / 约 49.8k | 项目活动、周期、文档和路线图的历史呈现；用于追加式进度时间线和重点标记 | AGPL-3.0 | 官方仓库含安全策略；仅参考交互/字段，不复用代码 | 未使用 |
| Open WebUI | https://github.com/open-webui/open-webui | 2026-09-06 / 约 133k | Workspace 中 Skills 独立清单、轻量 manifest、按需加载完整说明和启停/导入导出 | Open WebUI License，含额外品牌条款；历史贡献授权不一 | 仅参考公开信息架构，不复制代码或品牌元素；保持本项目 prompt-only 权限边界 | 未使用 |
| Thunderbird Desktop | https://github.com/thunderbird/thunderbird-desktop | 2026-09-06 / 成熟桌面邮件项目 | 三栏邮件阅读、附件状态与常规写信/回复/转发/搜索操作 | MPL-2.0 及仓库内其他组件许可证 | 仅参考公开交互；本项目继续纯文本、无远程资源和受限附件预览 | 未使用 |
| Storybook | https://github.com/storybookjs/storybook | 2026-09-06 / 约 88k | 将典型、空、错误和完成状态以独立样例呈现；用于 Demo 合成数据覆盖和可复现验收 | MIT | 只参考组件状态展示方法，不引入开发服务器或在线插件 | 未使用 |
| SpeedCrunch | https://github.com/ruphy/speedcrunch | 2026-09-06 / 约 119 | 直接键入表达式、函数面板、历史和键盘优先体验；与 Qalculate 交叉验证计算器交互范围 | GPL-2.0 | 项目成熟但当前 Star 不高；仅参考产品思路，不复制代码 | 未使用 |
| GitHub CLI | https://github.com/cli/cli | 2026-09-06 | 仅用于用户授权的仓库与 Release 发布，不作为客户端依赖 | MIT | 官方 v2.100.0 macOS arm64 资产，ZIP 内容及许可证检查，SHA-256 与官方发布摘要一致：45f9a62da2f6e641a7fad57e2ce39656dfd7ef331372d80a2a2aed65abb01642；发布后已清除隔离副本 | v2.100.0 |

新增记录时替换占位行或在下方增加一行。同一项目再次使用时合并到原记录，避免重复。

## 最终采用结论

- 界面采用统一侧栏导航、按需详情与共享 AI 入口；项目、文档、邮箱和 Skills 分别保留权限边界。参考产品的交互不等于复制其代码。
- 桌面架构已确定为 Tauri/React/Rust/SQLite；Electron 仅作对照，Docling/MarkItDown 未作为运行依赖。文档使用受限 Rust 解析、随包 PDF.js 及结构化 DOCX 导出，不再保留“技术待定”的早期计划。
- GPL/AGPL 等参考项目仅吸收公开产品思路；实际采用依赖的版本、许可和安全结论以本表、锁文件及随包许可证资料为准。
- 最初的产品调研只读公开资料；后续实际采用依赖及发布工具的检查分别记录在对应行。该版参考下载副本和临时发布工具已清理，必要依赖资源与许可证继续保留。
- 表中 Star 为各行调研日期的历史近似值，不代表当前数量或安全背书。“可离线”不等于“默认无外联”；新依赖仍需逐项审计，不以此前调研结论代替检查。
