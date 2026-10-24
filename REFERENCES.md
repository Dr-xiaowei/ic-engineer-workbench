# 🔗 GitHub 参考项目记录

> **用法：** 搜索、比较和采用的候选在此留档；重点候选按下列十项标准分析。保留可追溯证据和结论，不长期保留下载副本。以下既有清单是各行日期的历史记录，尚未按 2026-09-19 新标准全面复核，不据此宣称候选当前仍在维护或安全。

## 2026-09-30：文档互转补齐（T-044 / R-035）

用户直接要求增加常见文档与 Markdown 互转，已有能力不重复开发。核对已有 PDF/DOCX/TXT→MD、笔记 MD→PDF/DOCX 后，选择 **B：复用既有工作台并补齐 PPTX 和统一入口**。A 直接换用独立转换器会增加安装和切换，C 从零编写 Office 导出格式增加兼容风险。最小流程为选择→Markdown 预览→另存 MD/PDF/DOCX/PPTX/TXT；不引入模型、云服务、Office 安装或数据库迁移，不承诺扫描 OCR、旧二进制 DOC/PPT 解析和无损版式。用户本次具体新增指令作为该范围实施授权。

| 十项核对 | PptxGenJS | MarkItDown | Pandoc |
| --- | --- | --- | --- |
| 来源/版本 | [gitbrent/PptxGenJS](https://github.com/gitbrent/PptxGenJS)，npm 4.0.1 | [microsoft/markitdown](https://github.com/microsoft/markitdown)，main 文档，不安装 | [jgm/pandoc](https://github.com/jgm/pandoc)，[官方手册](https://pandoc.org/MANUAL.html)，不安装 |
| 维护 | API 最近推送 2025-11-28，未归档；近期约10个月未推送，不能称日常活跃 | 最近推送 2026-09-21，未归档 | 最近推送 2026-09-29，未归档 |
| 社区 | 6,204 Star / 958 Fork / 298 open issues | 187,592 / 13,872 / 703 | 46,449 / 5,252 / 1,055 |
| 部署 | 前端按需加载，用户无额外安装 | Python 和分格式依赖需额外打包更新 | 独立二进制，PDF 输出还需排版引擎 |
| 环境/服务 | 浏览器内存生成，无需服务 | 本地可用，可选云/OCR不采用 | CLI 本地可用，需限制外部资源 |
| 可直接用 | PPTX 文本/表格生成 | PDF/DOCX/PPTX→MD | MD→DOCX/PPTX 等 |
| 可复用 | 接现有 Markdown 安全渲染和文件对话框 | 本轮仅参考覆盖边界；不重复现有转换 | 输出多与现有实现重复，仅参考 |
| 必须补齐 | 受限 PPTX 读取、分页和源文件保护；复用 ZIP/XML | 仍需反向导出和桌面集成 | 仍需 PPTX/PDF 输入和桌面集成 |
| 安全 | 官方 tarball 及新增依赖 npm SHA-512 一致，完整隔离扫描560项未检出；新增7个包，冻结锁文件且禁用安装脚本。库支持显式图片路径读文件/联网，本次只传正文/表格并用内存 write，不调用远程图片和 writeFile。依赖后续扫描结果见验收目录 | 未下载执行；插件、云和模型权限不自动启用 | 未下载执行；过滤器/引擎执行与资源读取需额外限制 |
| 许可/综合 | MIT 保留版权，采用固定版本；维护偏弱作为风险记录，扫描不等于无漏洞 | MIT；维护活跃，但增加运行时和重复功能不合算 | GPL-2.0-or-later；只参考文档，不链接/分发 |

统计来自当日官方 GitHub API/npm。Issue 响应率和完整漏洞审计未核实。PPTX 与 Markdown 属于内容转换，页序、正文、列表、表格、中文单位需验收，图片/图表/公式/动画和复杂版式明确提示损失及复核。资料仅本地处理，不执行宏/嵌入对象。

## 2026-09-26：Windows 适配与隔离验收（T-043）

**同日最新选型覆盖：用户要求清理Win11、采用Win10。** 当前使用UTM/QEMU x64仿真（4核/6GiB/96GiB动态盘），只读复用已有官方Win10 LTSC2021 x64评估ISO，不复用其他项目的系统盘或自动应答文件。下文ARM来源属于已弃用方案的历史安全记录，相关镜像与ARM专用工具已删除；共用扫描器、驱动ISO及来源资料继续复用。

- [微软LTSC2021生命周期](https://learn.microsoft.com/en-us/lifecycle/products/windows-10-enterprise-ltsc-2021)：支持至2027-01-13；普通Win10生命周期与LTSC区分。使用90天评估版做兼容性测试，不购买、不绕过激活，本轮用户回复“同意”后已接受当前安装许可。
- [微软21H2评估摘要](https://download.microsoft.com/download/9/3/b/93bfb33f-643f-43c9-a36f-cf3611c72187/Windows10Enterprise21H2HashValues.pdf)：x64 en-US ISO的SHA-256 `e4ab2e3535be5748252a8d5d57539a6e59be8d6726345ee10e7afd2cb89fefb5`，本机核对一致；61,024份去重WIM内容+903外围文件扫描完成，保留1项已验证微软签名来源的Edge告警。离线来源核验不等于杀毒厂商确认。
- [VS2022系统要求](https://learn.microsoft.com/en-us/visualstudio/releases/2022/system-requirements)与[固定发布记录](https://learn.microsoft.com/en-us/visualstudio/releases/2022/release-history)：拟用支持Win10 LTSC的VS2022 Build Tools，只需MSVC/SDK；引导器摘要`37bb0fb429d163ecebd272a865d11a37b906d152bef960da2ddb29c2e2fd6eeb`，扫描通过；17.14.41及备用固定17.14.40的目录均不符合对应渠道声明的大小/SHA-256，已隔离且不使用，组件和客户机验证仍待完成。 2026-09-29仅复核官方channel元数据一次：91,781字节，SHA-256 `fca418ba94ffbcfb7a2b25f10f16f39dd09660568d21eef4bd3f274cb0b27b8c` 与旧文件一致；未重复下载异常目录，也未新增执行依赖。
- [Node 24.21.0 x64](https://nodejs.org/dist/v24.21.0/)归档摘要`158f7685b44de51f6c0df1d153526cbcd3e1bc739a8dfc607721cef75de9e541`；[rustup1.29.1 x64](https://static.rust-lang.org/rustup/archive/1.29.1/x86_64-pc-windows-msvc/)按官方摘要核对；两者扫描通过，不在宿主运行Windows程序。llama.cpp改用相同b11146官方x64 CPU包，摘要`14cf1303ca9ac3abd94816850532f9f9a69ac66fbaca3776fc6f9061c2fac1d1`；实际扫描/使用状态看`windows10/tools-provenance.json`。
- [Win10 LTSC累计更新KB5129236](https://support.microsoft.com/en-us/servicing/os/windows-10/2026/09/kb5129236-windows-10-21h2-22h2-update)及[微软更新目录](https://www.catalog.update.microsoft.com/Search.aspx?q=KB5129236)：2026-09-14发布，目标build 19044.7727，包含9月安全更新与SSU。x64 MSU为939,850,337字节，官方目录SHA-1一致；内容SHA-256、CMS、时间戳及固定微软根链已核验，ClamAV扫描4.70GiB未检出且无超限/错误。尚未安装，Windows信任/吊销与适用性仍待确认；保留官方列明的USB音频和File History已知问题，不将系统补丁等同于应用备份验收。CAB摘要结构参考[osslsigncode维护者源码](https://github.com/mtrojnar/osslsigncode/blob/master/cab.c)，仅静态参考格式，未安装或纳入产品。


用户已直接授权现有产品的 Windows 版本和本目录验证环境。产品选择 B：复用并适配现有 Tauri/Rust/SQLite，保持全部既有功能；A 直接把 Mac 包当 Windows 包不可用，C 重写客户端会增加成本和数据兼容风险。环境选择官方 UTM/QEMU 的 ARM 硬件虚拟化、按需增长 qcow2 磁盘；不使用来源不明的精简 Windows、预装工具镜像或激活脚本。纯 x64 全系统仿真成本较高；商业虚拟化会引入额外许可/费用，本机也未发现现有安装，暂不采用。Windows x64 候选在 ARM Windows 的兼容运行与 Intel/AMD 原生验证分开记录。

| 检查维度 | 证据、选择与限制 |
| --- | --- |
| 维护 | [UTM v4.7.5](https://github.com/utmapp/UTM/releases/tag/v4.7.5) 发布 2026-01-03，仓库最近推送 2026-09-25；[ClamAV 1.5.4](https://github.com/Cisco-Talos/clamav/releases/tag/clamav-1.5.4) 发布 2026-08-07，最近推送 2026-08-27，时间均 UTC |
| 社区 | 官方 GitHub API 当日记录：UTM 35,631 Star / 1,828 Fork / 1,095 open issues；ClamAV 7,276 / 930 / 399，均未归档；响应率及漏洞清单未全面审计，Star不作为安全结论 |
| 部署 | UTM官方DMG约250MB；ClamAV官方macOS通用PKG约167MB。先核对摘要和Apple签名，再隔离提取，不自动运行PKG安装脚本、不改宿主启动项。Windows工具链仅放验证环境 |
| 环境/服务 | 宿主arm64 macOS27；已建立4核/8GiB、96GiB逻辑磁盘按实际块增长的空白虚拟机。未找到可核实的微软ARM评估下载，用户另行提供官方零售多版本ARM ISO，已核对摘要；Windows许可/版本待确认。无需云端上传私有数据 |
| 直接可用 | UTM已有Windows ARM、UEFI/Secure Boot/TPM支持；[Tauri Windows官方说明](https://v2.tauri.app/distribute/windows-installer/)支持MSVC目标与NSIS、WebView2离线安装；[ClamAV扫描](https://docs.clamav.net/manual/Usage/Scanning.html)支持本地文件和多种封装 |
| 改造 | 复用同一产品源码、业务和数据库，适配Windows路径、凭据、软件选择/启动、构建PATH与安装配置；验证脚本和安装升级回归需补齐 |
| 新增必要保护 | 数据库迁移必须事务化；升级前自动留可恢复快照，拒绝旧应用写更新schema，失败不覆盖数据；测试普通/Demo隔离、重启和文件原件摘要 |
| 安全 | 虚拟磁盘和交换目录位于项目忽略目录；不挂载个人目录、宿主物理盘，不共享剪贴板，不关闭宿主/客户机安全防护。下载先比对官方摘要、验签与静态扫描，客户机Defender补验；扫描不等于无未知漏洞，未覆盖的嵌套格式须明示 |
| 许可 | UTM主项目Apache-2.0、内含QEMU等组件有独立许可；ClamAV GPL-2.0，只作为本机验收工具，不链接进MIT产品或随产品分发。Windows/MSVC/WebView2遵守各自微软许可，不伪造密钥/账号或购买付费方案 |
| 综合与验收 | 官方来源+已有框架适配可减少主机安装与重复数据；默认只建一个可回滚环境。未完成签名/病毒库/扫描或Windows运行时，均不能写成环境已安全可用。最终以实际二进制、安装/升级、业务与错误场景证据放行 |

已锁定来源：UTM DMG 250,021,057字节，官方SHA-256 `a8435c93cfb5f8bbfeea4b134cfad1ac66b67632b75e438c63b1a8ae043bef0e`；ClamAV PKG 166,772,489字节，官方SHA-256 `df7fa753e2f9f67f3bc99b2a40a3be7ef559088c68ad6bdf66b4b5764e965bd6`。元数据/下载位于 `reference-sandbox/windows-2026-09-26/`，下载后的实测结果另记验收证据，不预先声称通过。

微软来源：[Windows 11 ARM64官方下载与摘要](https://www.microsoft.com/en-us/software-download/windows11arm64)、[Windows 11 Enterprise评估说明](https://www.microsoft.com/en-us/evalcenter/evaluate-windows-11-enterprise)；[UTM Windows指南](https://docs.getutm.app/guides/windows/)说明新版支持TPM/Secure Boot。本轮不采用其旧版本绕过检查做法。

本轮实际使用的辅助工具均只服务于隔离验证，不进入产品依赖或交付包：

| 工具/版本 | 来源、用途与许可 | 本轮检查与限制 |
| --- | --- | --- |
| 7-Zip 26.03（macOS） | [官方下载页](https://www.7-zip.org/download.html)、[固定发布](https://github.com/ip7z/7zip/releases/tag/26.03)；静态展开微软引导包，保留LGPL/BSD及unRAR限制许可 | 官方资产SHA-256 `5ca87677072c59f5602e5c49baa27d4694bacd2259b4e507f0094249d4281480`及大小一致，归档路径检查与ClamAV扫描通过，仅在项目tools目录使用，不写系统安装位置、不进入产品 |
| wimlib 1.14.4 | [官方固定发布](https://wimlib.net/forums/viewtopic.php?t=737)、[API](https://wimlib.net/apidoc/)；清点和只读提取 WIM。工具 GPL-3.0+，库另有 LGPL 等授权说明 | 固定稳定版，不宣称最新版；源包 SHA-256 `3633db2b6c8b255eb86d3bf3df3059796bd1f08e50b8c9728c7eb66662e51300` 与发布一致，扫描通过、审阅后仅在项目内编译，不执行安装脚本，不写系统目录；自编清点工具按内容摘要去重，不运行 Windows 文件 |
| UTM Guest Tools 0.1.271 | [官方支持页](https://docs.getutm.app/guest-support/windows/)、[固定发布](https://github.com/utmapp/qemu/releases/tag/v10.0.2-utm)；ARM64 virtio 驱动和客户机工具，保留逐组件许可 | SHA-256 `65b6a69b392ee01dd314c10f3dad9ebbf9c4160be43f5f0dd6bb715944d9095b`；整包时限告警后改逐文件扫描通过。发现 Autounattend 会关闭 UAC、跳过部分安全初始化，排除该配置，不直接在安装期挂载原驱动 ISO |
| Node 24.21.0 / rustup 1.29.1 | [Node 固定目录与摘要](https://nodejs.org/dist/v24.21.0/)、[Rust 官方安装](https://rust-lang.org/tools/install/)、[固定 rustup](https://static.rust-lang.org/rustup/archive/1.29.1/aarch64-pc-windows-msvc/)；Windows ARM64 工具链，按各自 MIT/Apache 等许可使用 | 官方摘要一致、ClamAV 未告警；Windows 内按锁文件安装依赖，工具和下载缓存放客户机，不更改宿主开发环境；安装器可能联网下载组件，仍需逐批检查 |
| Visual Studio Build Tools | [微软安装器](https://aka.ms/vs/stable/vs_BuildTools.exe)、[组件说明](https://learn.microsoft.com/en-us/visualstudio/install/workload-component-id-vs-build-tools)；最小 MSVC+Windows SDK，微软许可 | 引导程序 SHA-256 与官方重定向资产名一致，扫描未告警，PE 内容/嵌入微软签名密码学校验通过；Windows 根信任/吊销检查及组件下载验证待做，不预先接受附加许可 |
| llama.cpp v0.5.0 / b11146 | [正式版本](https://github.com/ggml-org/llama.cpp/releases/tag/v0.5.0)指向[固定二进制](https://github.com/ggml-org/llama.cpp/releases/tag/b11146)，2026-09-23；MIT。Windows ARM64 CPU 归档12,034,624字节，避免 GPU 套件占用 | SHA-256 `1727d241f3bf6d27360e984e851cf013928fd655bf89f8628e70da027f377b7d` 与官方API一致，ZIP路径/链接检查与ClamAV通过；服务文档的工具执行与MCP能力默认关闭，验证仅使用本地权重、回环地址、offline与no-webui，不启用文件/命令工具；仍未在Windows运行 |

这些是已有工具的定版本验证用途，非新增产品功能。Star/响应速度未对每个编译辅助工具复评，不据此作安全保证。UTM/ClamAV 的官方签名与公证已核验；ClamAV 本地可搬移副本因调整 RPATH 重新作 ad-hoc 签名，原厂签名验证针对未经改动的来源文件，不能把调整后的副本声称为 Cisco 签名。

安全告警处理参考 [ClamAV 官方说明](https://docs.clamav.net/faq/faq-scan-alerts.html)：保留告警、读取规则、核对来源/内容/签名，不自动删除或加白名单。续验按[微软 PE/catalog 签名说明](https://learn.microsoft.com/en-us/windows/win32/secbp/understanding-pe-signatures)比对11份目录签名文件与5份PE/Appx，验证全部CMS和RFC3161时间戳；根证书下载自[微软 PKI 仓库](https://www.microsoft.com/pkiops/docs/repository.htm)，核对其公开指纹，仅作离线验证，不导入系统信任。证书及UTM配置说明共8份文件扫描未检出，4项内存篡改负例被拒绝。证据支持误报判断，仍不替代厂商确认、Windows原生信任策略及吊销检查；原始16项告警保留。具体状态见 [Windows 验证说明](docs/WINDOWS_VALIDATION.md)。未上传系统文件、项目资料或个人数据到外部扫描网站。

## 2026-09-26：真实本地模型验收工具（T-042，非成品依赖）

用户明确允许下载本地小模型验证。采用官方 [Ollama 0.34.4](https://github.com/ollama/ollama/releases/tag/v0.34.4)（2026-09-23 发布，MIT），隔离查看发行元数据、LICENSE 与该 tag 的 `envconfig/config.go` 后运行；macOS 归档 160,042,307 字节，SHA-256 `e9c8fddaab5f48f47f2c4ae3d23d0732f5182417125353faeed2188e34a22799` 与官方一致。参考副本位于本项目 `reference-sandbox/acceptance-2026-09-26/`，运行时/权重与日志位于忽略的验收目录，不进入应用、锁文件或正式交付。

模型选 [Qwen3.5:0.8b](https://ollama.com/library/qwen3.5:0.8b)（873M、Q8_0、约 1 GB，含图片输入）和 [Qwen2.5:0.5b](https://ollama.com/library/qwen2.5:0.5b)（494M、Q4_K_M、398 MB）；模型页均列 Apache-2.0，CLI 拉取完成 SHA-256 验证。Qwen2.5 另通过官方 [create API](https://docs.ollama.com/api/create) 建立温度 0、最多 768 token 的本机验收配置，复用权重，不改工作台默认配置或硬编码厂商参数。

安全边界参考官方 [FAQ](https://docs.ollama.com/faq)、[OpenAI 兼容接口](https://docs.ollama.com/api/openai-compatibility)及[思考控制](https://docs.ollama.com/capabilities/thinking)：仅绑定 `127.0.0.1:11439`，设置 `OLLAMA_NO_CLOUD=1` 与 `OLLAMA_NOHISTORY=1`，模型目录显式放在本项目验收目录。下载通过用户已有代理，不改变系统代理；推理只提交合成资料。运行时可能创建自己的本地主机身份文件，但不读取用户项目密钥；没有安装自启或改系统信任。此为来源/配置初查与本机运行验证，不是完整运行时安全审计。

选择理由是资源小、官方提供 macOS 运行时与标准兼容接口，能覆盖文本/图片；不是按 Star 推荐企业生产模型。维护证据以具体发行版本为准，本轮未统计仓库 Star、Issue 响应率或比较服务端集群部署。已有 UI/API/许可证文档与实际适配器均可复用，不移植其代码进工作台；备选仅作交叉验收，不替换产品架构。

质量结果：真实请求及流式返回通过，但小模型出现遗漏条件、虚构参数/来源及答非所问。**仅用于联通与流程验证，未通过模拟 IC 专业内容验收**；不能由此宣称公司岗位收益或推荐小模型承担设计评审。详细证据与未完成项见 [T-042 验收报告](COMPLETION_REPORT.md)。

## 2026-09-25：v1.0.1 提效复审与首轮方案（T-040 调研；T-041 已获实施确认）

**实施补充：** 用户随后确认“先实施第一轮四项升级优化”，对应 R-032。下文保留调研时的不足、备选和拟议指标；源码行号指上述调研基线，不能当作修改后的定位。T-041 已落实四项功能，具体测试与剩余业务验收见 [完成度报告](COMPLETION_REPORT.md)。

本轮只借鉴 Zotero 的引用回溯、AnythingLLM 的资料范围及 Cherry Studio 的助手预设等产品思路，没有复制这些项目代码或安装其依赖。继续复用既有 PDF.js（Apache-2.0）、React、Rust/SQLite；许可证说明和锁文件保持不变。中文/型号查找采用绑定参数的字面子串检索与相关片段截取，覆盖全部已索引文本；这不是分词优化或向量索引，十万级文档性能及真实业务命中率尚未测试。Docling/OCR、CSV 对比、单位运算、任意脚本及服务器扩展仍为待评估项。

用户要求：先搜寻成熟、持续维护的类似项目，再提出不足与优化建议；工作台要让模拟 IC 设计公司的工作人员提效，默认简单、进阶可定制，确认后才优化，GitHub 暂不更新。本轮仅检查现有源码、公开仓库及官方文档，未修改产品代码、安装运行候选、接入公司数据或发布新版本。所有下述效果目标是拟议验收标准，不是已测业务收益。

本机源码基线 `17532defb5059e90978708c748e8b6ec31529c1e`，正式应用仍为 v1.0.1。9 月 21 日的发行验收证明当时已测范围通过，不能替代本次对跨模块业务流程、查找质量和岗位收益的复审。

### 建议先确认的结论

推荐 **B：在现有工作台上做有边界的二次开发**。优先打通“找到设计资料 → 查看证据 → 形成评审笔记 → 留下待办”，复用现有 SQLite、PDF.js、项目与笔记能力；借鉴 Zotero 的引用方式、AnythingLLM 的资料范围、Cherry Studio 的助手预设，避免把几个完整产品拼进一个客户端。Docling 留作扫描件和复杂表格的可选增强，不作为基础安装必需服务。

| 路线 | 可取之处 | 对本项目的代价 | 决策 |
| --- | --- | --- | --- |
| A 直接使用 Zotero + AnythingLLM，或 Cherry Studio | 不自建成熟阅读、资料问答和助手功能；可作为批准后的对照试用 | 多个应用之间仍需切换；项目/待办/笔记不能自然贯通；需另行配置内网、隐私及版本管理 | 如果用户最终只要资料管理与问答，这是合理备选；本次不默认替换 |
| B 改进现有 v1.0.1 | 保留本地轻客户端、现有数据和已确认功能；将新增投入集中在实际断点 | 需完善上下文、索引、引用与草稿；仍要维护自有代码 | **首选**；上游代码复用与仅参考交互分别标明 |
| C 从零开发新的工作台 | 可以完全重订信息架构和数据模型 | 要重新实现已经具备的存储、权限、备份和导出，迁移与回归成本高，尚无推倒重来的依据 | 本轮不选；先通过 B 验证业务收益 |

### 现有实现的不足与影响

下列“代码确认”表示已看到实现限制；“隔离复现”使用内存 SQLite 合成数据，未读写用户数据库。本轮未重新启动应用做完整 UI 验收。

| 优先级 | 发现与证据 | 对日常工作的影响 | 建议 |
| --- | --- | --- | --- |
| P0 | [App.tsx](app/desktop/src/App.tsx) 第 557、675–695 行切页直接替换组件；[KnowledgeWorkspace](app/desktop/src/KnowledgeWorkspace.tsx) 第 13–19 行只拦截笔记内部切换，草稿是组件状态；对话、邮件、Datasheet 也有局部状态 | 编辑中去查资料可能丢未保存内容；Datasheet 原件/分析需重新载入。代码路径可确认风险，尚未本轮 UI 复测 | 跨页保留草稿与阅读位置；重启恢复提示；区分“保存草稿”和“正式笔记”，保存失败保持内容并可重试 |
| P0 | [storage.rs](app/desktop/src-tauri/src/storage.rs) 第 189–195 行 `unicode61`；[projects.rs](app/desktop/src-tauri/src/projects.rs) 第 494–508 行只按空白分词。隔离复现：“带隙基准启动电路设计记录 PSRR”搜“启动”及“启动*”均为 0 条，搜完整中文串或 PSRR 为 1 条 | 中文设计笔记中有内容却查不到；此前关键词回归不足以覆盖此情形 | 补中英混合分词/片段索引与精确型号检索；明确处理短词、µ/μ/u、符号和单位；用实际脱敏查询评估 |
| P0 | [projects.rs](app/desktop/src-tauri/src/projects.rs) 第 547–571 行候选只有最近 40 份文档前 1,200 字符；[ProjectWorkspace](app/desktop/src/ProjectWorkspace.tsx) 第 248–258 行再截前 28,000 字符 | 所谓语义检索不是面向全部资料的语义索引；旧文件和文末结论可能根本没交给模型 | 按页/段取与问题相关的证据，显示实际检索范围、缺失/失败文件；先做好可测的本地检索，再考虑可选向量检索 |
| P1 | [PdfReader](app/desktop/src/PdfReader.tsx) 当前只渲染 canvas，只有翻页与缩放；[DatasheetWorkspace](app/desktop/src/DatasheetWorkspace.tsx) 第 28–33 行只取前 28,000 字符，页码主要靠提示词生成 | 长 Datasheet 查参数、选原文、从摘要跳回出处仍费时；不能保证后半部电气条件被涵盖 | 搜索与选文、页码跳转、阅读位置；按页/段引用对象保存文件版本和原文，不只输出一个“[第 N 页]”字符串 |
| P1 | [App.tsx](app/desktop/src/App.tsx) 第 513–529 行浮空助手上下文为固定模块说明，不含用户选中资料/记录 | “授权当前页面”之后仍不能解释正在看的具体设计内容 | 改成“分析选中内容”，显示本次片段与来源清单；选择项目不自动发送项目全部资料；每次内容授权可撤销 |
| P1 | [NoteInput/Note](app/desktop/src-tauri/src/notes.rs) 只有标题、正文、时间，无项目与引用字段；首页以入口和系统状态为主 | 评审依据、项目进度、个人笔记分散；同一事实反复复制 | 持久的项目关联与证据笔记；首页显示继续上次工作、今日待办和最近资料；一般员工不需要关注底层数据位置 |
| P1 | [preferences.ts](app/desktop/src/preferences.ts) 主要是头像、名称和主题；[SkillsWorkspace](app/desktop/src/SkillsWorkspace.tsx) 以导入/启停为主 | 新人面对 11 个入口和技术术语；高手缺少可视化创建模板和字段能力 | 默认五个业务入口，旧功能在二级菜单可达；将用户界面的 Skills 解释为“工作模板”，提供复制修改及导入导出 |
| P2 | [document.rs](app/desktop/src-tauri/src/document.rs) 限 25 MiB，含文本层 PDF 为主；[calculator.ts](app/desktop/src/calculator.ts) 仅数值/函数，没有工程单位量纲体系 | 扫描资料、复杂表格与含单位计算仍需外部工具 | 先明确反馈和外部打开；复杂解析、单位计算、CSV 结果对比作为后续独立评估，不偷偷放大首轮 |

### 成熟候选：维护和社区证据

检索词包括 `local desktop AI knowledge assistant`、`PDF annotations notes`、`offline document parsing tables`、`analog IC assistant`，及对应仓库名、Release、privacy、license。只采用官方仓库/文档和 GitHub API 的事实，不把营销描述当实测结果。八个重点候选都未归档，且有近期实质修复/合并活动；这支持“仍维护”，不保证每天提交或公司部署成熟度。

以下 Star/Fork/开放 Issue/PR 为 **2026-09-25 12:04 UTC（北京时间 20:04）** 的快照；日期列使用 **UTC**，Issue 与 PR 分开计数。抽查默认分支最近 8 条提交、最近 2 条合并 PR、最近更新的 2 个 Issue，不能据此计算全项目响应率。固定提交链接是本轮文件检查快照，可能领先正式 Release；上线前应按所选正式 tag 重新核对资产和依赖。

| 项目 | Star / Fork | 开放 Issue / PR | 最近正式版 | 实质代码样本 | 维护者活动样本 |
| --- | --- | --- | --- | --- | --- |
| [cherry-studio](https://github.com/CherryHQ/cherry-studio) | 52,139 / 5,009 | 1088 / 554 | [v2.1.3](https://github.com/CherryHQ/cherry-studio/releases/tag/v2.1.3) · 2026-09-24 | [09d4ea5e](https://github.com/CherryHQ/cherry-studio/commit/09d4ea5e2f6756a31377a388446d66253f87cd1b) · 2026-09-25 | [合并 #21031](https://github.com/CherryHQ/cherry-studio/pull/21031) · 2026-09-25，DeJeune |
| [anything-llm](https://github.com/Mintplex-Labs/anything-llm) | 66,453 / 7,400 | 305 / 37 | [v1.16.2](https://github.com/Mintplex-Labs/anything-llm/releases/tag/v1.16.2) · 2026-09-22 | [58ae6fee](https://github.com/Mintplex-Labs/anything-llm/commit/58ae6fee08e10e649ca13d0cfc55598d90422cf8) · 2026-09-24 | [合并 #6470](https://github.com/Mintplex-Labs/anything-llm/pull/6470) · 2026-09-24，timothycarambat |
| [zotero](https://github.com/zotero/zotero) | 15,391 / 1,133 | 1412 / 191 | [10.0.4（Mac）](https://www.zotero.org/support/changelog) · 2026-09-22；其他桌面 10.0.3 · 09-17；GitHub 无 Release | [3ff64b65](https://github.com/zotero/zotero/commit/3ff64b658d6df9956e1eb05648fc858213cc1418) · 2026-09-24 | [合并 #6051](https://github.com/zotero/zotero/pull/6051) · 2026-09-21，dstillman |
| [joplin](https://github.com/laurent22/joplin) | 56,480 / 6,304 | 630 / 9 | [v3.7.18](https://github.com/laurent22/joplin/releases/tag/v3.7.18) · 2026-09-11 | [d9f0b420](https://github.com/laurent22/joplin/commit/d9f0b4206a34112895b0034592be5c7ea05bfc47) · 2026-09-25 | [合并 #16576](https://github.com/laurent22/joplin/pull/16576) · 2026-09-25，laurent22 |
| [docling](https://github.com/docling-project/docling) | 67,936 / 4,926 | 807 / 124 | [v2.130.0](https://github.com/docling-project/docling/releases/tag/v2.130.0) · 2026-09-22 | [2d5c590c](https://github.com/docling-project/docling/commit/2d5c590c34b6378fd8a47c65b534b280aa40c93c) · 2026-09-25 | [合并 #4371](https://github.com/docling-project/docling/pull/4371) · 2026-09-25，ceberam |
| [dify](https://github.com/langgenius/dify) | 157,172 / 24,778 | 409 / 684 | [1.17.1](https://github.com/langgenius/dify/releases/tag/1.17.1) · 2026-09-10 | [d86435ee](https://github.com/langgenius/dify/commit/d86435ee1e69b5badef832baf785863ea99bfbd7) · 2026-09-25 | [合并 #42750](https://github.com/langgenius/dify/pull/42750) · 2026-09-25，crazywoola |
| [open-webui](https://github.com/open-webui/open-webui) | 153,134 / 22,398 | 236 / 82 | [v0.11.4](https://github.com/open-webui/open-webui/releases/tag/v0.11.4) · 2026-09-21 | [46ea826e](https://github.com/open-webui/open-webui/commit/46ea826e012a63a60c5955b2c69c74a0a0eb9c56) · 2026-09-21 | [合并 #31342](https://github.com/open-webui/open-webui/pull/31342) · 2026-09-25，tjbck |
| [ragflow](https://github.com/infiniflow/ragflow) | 91,288 / 10,823 | 1164 / 350 | [v0.27.2](https://github.com/infiniflow/ragflow/releases/tag/v0.27.2) · 2026-09-10 | [313ca90f](https://github.com/infiniflow/ragflow/commit/313ca90f6abd7682fe8523e16fd67b3653a3fa84) · 2026-09-24 | [合并 #20165](https://github.com/infiniflow/ragflow/pull/20165) · 2026-09-24，yuzhichang |

Zotero 官方主要在论坛受理支持，不因 GitHub Issue 多或没有 GitHub Release 就判为失维护。[维护者对 #294 的回应](https://github.com/zotero/zotero/issues/294#issuecomment-5770451238)在 09-22；[Joplin #16576](https://github.com/laurent22/joplin/pull/16576#issuecomment-5743376926)在 09-19 有负责人回应；[Open WebUI #31403](https://github.com/open-webui/open-webui/issues/31403#issuecomment-5831508763)在 09-25 有成员回应。其余样本有近期合并，但本轮抽查不足以评价 Issue 响应速度。

### 同一需求下的部署、复用与缺口（十项标准第 3–7、10 项）

以下均为文档评估，未安装实测。普通员工应只接收应用和公司已批准的配置；开发运行时不应转嫁给每个员工。

| 候选 | 普通用户部署 / 环境 | 直接可用的能力 | 本项目可借鉴或复用 | 仍缺什么 / 综合结论 |
| --- | --- | --- | --- | --- |
| Cherry Studio | 易：Win/Mac/Linux 桌面包；对话需本地/内网或第三方模型。源码开发依赖 Electron/Node，当前检查分支 Node ≥24.12；公司管理功能部分属于商业版 | 自定义助手、多模型对话、资料输入、全局搜索 | 助手预设、常用入口和可选设置的交互；保留本项目安全边界，当前只参考设计 | 不直接提供本项目设计证据→评审→项目待办闭环；社区版不能冒充有完整企业管理。**助手体验参考首选** |
| AnythingLLM | 易至中：桌面安装包或 IT 自托管；资料检索涉及 embedding/向量库和所选 LLM，首次模型准备需安排 | 工作空间文档、会话附件、文档检索、引用、Query/Chat 模式 | 文档作用范围、检索片段/重排设置与引用接口；MIT 模块可在具体审查后适配 | 不是现有 Tauri 的即插即用库；要解决索引同步、模型资源、出处精度和项目关联。**文档问答参考首选，独立试用备选** |
| Zotero | 易：桌面包；本地资料管理无需账号或 LLM；同步可选 | 文献/资料库、PDF 批注、笔记、引用回原文 | [批注加入笔记且回到原页](https://www.zotero.org/support/pdf_reader)，用本地 PDF.js 实现相应证据体验 | 无原生 IC 指标/PVT/评审待办闭环；不直接复制 AGPL 源码。**阅读和证据组织参考首选** |
| Joplin | 易：桌面包，本地笔记不要求模型；可选同步/插件增加配置；公司服务器授权单独判断 | Markdown/富文本、笔记组织、搜索、历史恢复、插件 | [笔记版本恢复](https://joplinapp.org/help/apps/note_history/)、默认易用与进阶扩展分离的交互；保留现有 Markdown/SQLite | 需项目与引用结构、设计模板；已有插件不能当安全隔离的 prompt Skill。**草稿与知识积累参考** |
| Docling | 对员工难、对 IT 中：Python ≥3.10、解析依赖与预下载模型；可离线运行，硬件与模型包体积需实测 | 文档结构、表格、OCR、多格式转结构数据 | MIT 解析能力可作为可选本地进程或经批准内网服务，保留原文定位 | 单独不是工作台；需安装/升级、资源上限、队列、超时与错误回退；工程符号、合并表格准确率未测。**后续解析增强候选** |
| Dify | 对员工难：IT 部署 Docker Compose、数据库/Redis/插件服务；可配置本地/内网模型 | 应用与工作流、知识检索、统一 AI 服务 | 只借鉴模板变量与可复用步骤；以后通过受控 API 接入 IT 已部署流程 | 运维与工作流学习负担较大，[部署文档](https://docs.dify.ai/en/self-host/deploy/quick-start/docker-compose)列有多项服务。**本轮不作为客户端基础** |
| Open WebUI | 中：通常由 IT 部署 Python/Docker 服务，配置模型及可选本地 embedding | 多用户模型入口、知识、提示词、工具扩展 | 以后已有统一 AI 门户时可评估受控接口；借鉴普通/高级配置分层 | 不等于本地工作台；权限/扩展/模型下载与品牌条款需处理。**现成公司 AI 门户备选** |
| RAGFlow | 难：Docker 及解析/检索等服务；[仓库前提](https://github.com/infiniflow/ragflow#-prerequisites)列 CPU ≥4 核、RAM ≥16 GB、磁盘 ≥50 GB；另需模型 | 文档解析、分块检索、可追踪引用、知识库 | 以后公司集中知识库和大批复杂文档服务可评估 | 资源、部署与维护明显重于现有客户端；不能由高 Star 推断更适合个人桌面。**服务端阶段候选** |

与 IC 更直接相关的结果也做了排除：[analog-agents](https://github.com/Arcadia-1/analog-agents)含模拟设计 Skill/EDA 桥接，但本次页面仅约 73 Star、65 次提交，长期维护、可靠性和完整许可未完成核实；不满足本轮成熟基础优先条件，不导入执行。[Xschem](https://github.com/StefanSchippers/xschem)属于原理图/网表工具，解决另一类问题；本项目仍不扩展成 EDA。以上不是宣称不存在适合 IC 的成熟工具，而是本次检索未找到可直接替换本工作台且同时满足全部约束的成品。

### 许可、外联和执行风险（十项标准第 8–9 项）

本轮对重点仓库的许可证、部分配置/隐私声明做了固定提交抽样。未做全依赖漏洞审计、二进制验签、全链路流量检查，也未全面检查 Cookie/SSH key/环境变量/所有插件权限；未核实项不能写成安全通过。没有运行安装脚本，没有使用真实秘密或业务资料。

| 项目 | 许可与复用边界 | 已看文件与风险结论 |
| --- | --- | --- |
| cherry-studio | [LICENSE](https://github.com/CherryHQ/cherry-studio/blob/09d4ea5e2f6756a31377a388446d66253f87cd1b/LICENSE)：AGPL-3.0；不能把复用代码当作无条件 MIT。允许的使用、修改和分发依具体许可条件处理。 | [PRIVACY.md](https://github.com/CherryHQ/cherry-studio/blob/09d4ea5e2f6756a31377a388446d66253f87cd1b/PRIVACY.md)：当前隐私声明中两类匿名运行统计默认开启，可在隐私设置关闭；自配模型直连所选服务，内置服务存在转发；package.json 的 prepare/postinstall 执行本地构建和 Git hook 设置。只阅读，不安装；关闭统计不等于阻断其他服务外联。 |
| anything-llm | [LICENSE](https://github.com/Mintplex-Labs/anything-llm/blob/58ae6fee08e10e649ca13d0cfc55598d90422cf8/LICENSE)：MIT，复用保留许可与版权声明；依赖/模型单独核对。 | [README.md](https://github.com/Mintplex-Labs/anything-llm/blob/58ae6fee08e10e649ca13d0cfc55598d90422cf8/README.md)：声明使用 PostHog 匿名遥测，DISABLE_TELEMETRY 或设置可关；仍可能访问模型服务、CDN 与 GitHub 文件。Agents/自定义工具执行面另审；不能把默认本地存储写成默认完全断网。 |
| zotero | [COPYING](https://github.com/zotero/zotero/blob/cc951f9916c49dae7ea74c8b0029c698aa293ca0/COPYING)：AGPLv3，商标和第三方组件独立；本轮仅参考交互。 | [README.md](https://github.com/zotero/zotero/blob/cc951f9916c49dae7ea74c8b0029c698aa293ca0/README.md)：官方隐私页说明账号/同步可选，自动元数据、PDF 获取、翻译器/样式更新有外联；需按选项关闭并实测。并未审计所有插件。 |
| joplin | [LICENSE](https://github.com/laurent22/joplin/blob/d9f0b4206a34112895b0034592be5c7ea05bfc47/LICENSE)：客户端默认 AGPL-3.0-or-later，子目录有独立 LICENSE；packages/server/LICENSE.md 是单独 Personal Use License，不能直接用于本公司部署推断。 | [packages/app-desktop/package.json](https://github.com/laurent22/joplin/blob/d9f0b4206a34112895b0034592be5c7ea05bfc47/packages/app-desktop/package.json)：客户端包声明与根许可核对；同步目标、插件和更新请求仍需部署前专项检查。客户端开源不等于公司服务端可按同样许可免费部署。 |
| docling | [LICENSE](https://github.com/docling-project/docling/blob/2d5c590c34b6378fd8a47c65b534b280aa40c93c/LICENSE)：MIT；OCR/模型权重及依赖单独许可，本次尚未逐一核实。 | [docs/usage/advanced_options.md](https://github.com/docling-project/docling/blob/2d5c590c34b6378fd8a47c65b534b280aa40c93c/docs/usage/advanced_options.md)：首次使用可自动下载模型，应预置；enable_remote_services 明确控制文档外送服务，但不控制模型下载。复杂资料解析资源与准确率需样本验证。 |
| dify | [LICENSE](https://github.com/langgenius/dify/blob/d86435ee1e69b5badef832baf785863ea99bfbd7/LICENSE)：Dify 自定义许可（基于 Apache-2.0 加条件），涉及多租户、品牌；本轮不作具体商用法律结论。 | [docker/docker-compose.yaml](https://github.com/langgenius/dify/blob/d86435ee1e69b5badef832baf785863ea99bfbd7/docker/docker-compose.yaml)：抽样看到插件/沙箱服务、Sentry/Amplitude 等可配置项、内置示例鉴权值；部署须换独立密钥并审查实际外联、执行隔离，不能照搬样例上线。 |
| open-webui | [LICENSE](https://github.com/open-webui/open-webui/blob/8bd8b4fac5e059578ac0c74b3c18d11139f88b7d/LICENSE)：Open WebUI License 含品牌限制及例外，历史代码不同许可；不能一概称为 MIT。 | [backend/open_webui/env.py](https://github.com/open-webui/open-webui/blob/8bd8b4fac5e059578ac0c74b3c18d11139f88b7d/backend/open_webui/env.py)：OFFLINE_MODE 默认 false；代码中启用后设置 HF_HUB_OFFLINE 并关闭更新检查。此开关不是网络防火墙，所配外部模型/工具仍可能联网；完整权限面未审计。 |
| ragflow | [LICENSE](https://github.com/infiniflow/ragflow/blob/313ca90f6abd7682fe8523e16fd67b3653a3fa84/LICENSE)：Apache-2.0，需保留许可/NOTICE，依赖/模型另核实。 | [docker/docker-compose.yml](https://github.com/infiniflow/ragflow/blob/313ca90f6abd7682fe8523e16fd67b3653a3fa84/docker/docker-compose.yml)：配置暴露 Web/API 服务并挂载数据/配置；连接器、模型源、运行脚本/代码执行器需 IT 控制。本轮未完整检查全部子服务及模型供应链。 |

补充依据：[Zotero 隐私](https://www.zotero.org/support/privacy)、[AnythingLLM 隐私设置](https://docs.anythingllm.com/features/privacy-and-data-handling)、[文档作用范围](https://docs.anythingllm.com/chatting-with-documents/introduction)、[Open WebUI 官方仓库收录的离线教程](https://github.com/open-webui/docs/blob/main/docs/tutorials/maintenance/offline-mode.mdx)（教程注明社区贡献）。Zotero/Joplin/Cherry Studio 的 AGPL 并不意味着不能在公司使用；这里区别“使用现成软件”和“复制代码进现有 MIT 分发物”，不替代对具体部署方式的许可核对。

### 简单默认界面与进阶定制

默认只保留五个主入口：**今日、项目、资料、助手、工具**。资料内部包含 Datasheet/笔记，工具包含计算器/转换/软件/邮箱等既有功能；日历在“今日”可达。常用模块可固定，用户能恢复全部入口；导航收纳不得导致既有功能丢失。每页围绕一个主要动作，不展示数据库、向量库、模型采样参数等非必要实现细节。

| 使用层级 | 可见能力 | 具体例子 |
| --- | --- | --- |
| 默认用户 | 选项目、找资料、读原文、保存笔记、添加待办；少量现成工作模板 | 直接选“设计评审记录”，填入本次材料和待确认问题 |
| 熟练用户 | 固定常用入口、保存搜索条件、复制并修改工作模板 | 常用“LDO 资料复核”“设计评审”“周报”，可修改关注字段和输出格式 |
| 高级用户 | 可折叠的模板变量、字段/顺序、提示词、默认模型和本地模板导入导出 | 将“电源/温度/负载条件、min/typ/max、来源”定义为必填；团队通过文件分发规范模板 |

五入口已按 R-032 确认并实施，替代 R-030 的 11 个一级入口约束；全部旧功能仍可进入，邮件与日历最终归入“今日”。高级定制采用受限配置，预览后使用、校验变量/格式/大小；内置模板保留，个人副本不覆盖默认定义。导入模板不会获得执行程序或访问文件的权限。任意脚本、MCP、自动 EDA、自动发送邮件、后台抓取资料不在首轮范围。

### 拟议最小范围：优先做可靠的设计资料闭环

**首轮 P0+P1（推荐确认项）：** 草稿保护；中文/型号检索；五入口与继续上次工作；PDF 搜索/选文/引用跳回原页；从引用保存为项目评审笔记；少量可复制编辑的工作模板。首页/笔记/待办复用既有实现，不另造复杂项目系统。

核心输入为用户授权的本地文本 PDF、TXT/Markdown 和现有 DOCX 文本提取、所属项目、明确问题；输出为有来源的评审笔记及可由用户确认创建的待办。AI 可选，未连接模型时查找、阅读、引用和笔记仍可完成。第一阶段不引入额外服务器或全量向量库。

例如：查找某 LDO 的启动限制 → 打开相关段落及测试条件 → 选原文加入评审记录 → 保留“参数、min/typ/max、单位、电源/温度/负载条件、文档版本/页码、待确认项” → 人工确认后形成笔记和待办。批注/草稿保存在工作台，不写回源 PDF。模型只能整理已选证据，缺失写“未确认”，不能推断未测 PVT 覆盖。

技术安排：继续用 SQLite/现有文件指纹保存授权引用与草稿，PDF.js 增加所需阅读层；引用保存文件 ID/版本、页码、片段，文件变化后提示重新核对。检索先做分段索引及中英混合查询验收；中文分词/短词回退采用哪种实现要在确认后的小样本评估中决定，不承诺换一个 tokenizer 即可解决。现有模型调用加入“本次资料清单”，跨项目/换页不继承外发授权。模板为数据配置，沿用 prompt-only 权限。

后续候选按实际收益再选，不默认全做：

1. **文档增强：** Docling 离线 OCR 与复杂表格，先用扫描件/合并表格/工程符号样本对照现有提取，确认准确率、资源与分发成本后再接入。
2. **结果对比：** 导入设计人员导出的 CSV/表格，按指标、单位及同一 PVT 条件对齐，展示差异并生成评审初稿。指标缺失/条件不同标明不可直接比较；不接 EDA、不解析专有数据库、不自动给工程签核结论。此项需要另评估表格组件与典型样本。
3. **日常表达：** 从用户选择的进度/笔记整理周报、从会议文本提炼待办；人工确认去重后写入，不自动录音或发邮件。
4. **工程小工具：** 在现有计算器加入常用工程前缀、单位核对和可保存公式模板，独立验证角度、dB 类型及量纲。
5. **公司统一服务：** 仅在出现真实多人权限/共享知识需求和 IT 运维条件时评估 Dify/RAGFlow/Open WebUI，不在每个员工电脑安装完整后端。

### 怎样证明能提效

先用同一套脱敏/合成任务测 v1.0.1 基线，再用新方案交叉顺序复测；至少包含设计、版图/验证或测试协作、项目协调角色及不同熟练程度。报告样本量、中位数/失败率与人工改正耗时，不只统计 AI 生成速度。以下目标待用户确认：

| 任务 | 拟议验收目标 |
| --- | --- |
| 找到有证据的答案 | 至少 30 个固定查询，覆盖中文词、英文缩写、型号、旧文件、文末内容和资料缺失；前 5 条命中正确来源 ≥90%，缺证据明确提示；规模和机器规格随报告记录 |
| 阅读到形成评审记录 | 同一资料任务，从定位原文到保存 5 条有来源的要点，总耗时中位数较 v1.0.1 降低 ≥30%，且人工纠错耗时不增加 |
| 工程可追溯 | 测试集每条数值结论均含单位、适用条件、文档版本/页码；字段缺失明确标记，引用可回到正确原页，条件冲突不可合并为同一值 |
| 草稿可靠性 | 跨页、普通退出/重启、异常退出恢复、保存失败/目录不可用均不静默丢失已确认保存点内容；附件和权限状态不因恢复而自动外发 |
| 新人和进阶用户 | 新人经不超过 5 分钟介绍可独立完成查找→引用→笔记；高级用户无需写代码可复制修改模板、调整字段并恢复默认 |
| 本地边界与迁移 | 原件哈希不变、两版数据隔离、v1.0.1 数据和备份可兼容恢复；断网核心流程可用，未授权资料不进入模型请求 |

这些是首轮放行门槛；不承诺“提高 30%”已经成立。如果定位/纠错耗时没有改善，即使功能和自动测试通过，也不能记作提效验收完成。真实公司模型、邮箱与目标操作系统仍按 T-034；本次调研未改变其状态。

### 本轮交付状态

T-040 调研完成后已获首轮四项实施确认，T-041 在独立分支修改本地源码并交付隔离预览。GitHub、正式标签与 v1.0.1 正式包不变。候选参考副本在收尾时按规则清理，仅保留来源、版本及调研元数据，不作为依赖或分发内容。技术验收与业务提效指标分开：本机测试通过不能证明已达到上述 30% 耗时改善或新人学习目标，真实岗位对照仍待公司批准的样本和人员。

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

上述结构在本文件中按实际候选补充即可，不另建重复模板文件。2026-09-19 的规范整理本身未重做调研；本次 2026-09-25 的重新核对和建议以文首 T-040 节为准，未复核的历史条目仍按原日期理解。

## 项目清单

### 本轮界面参考：DeepSeek Harness（2026-09-19）

- 来源：https://github.com/deepseek-ai/deepseek-harness；用户已有本地部署，主包 @deepseek-ai/dsh 0.1.5-rc.1，界面 renderer 包 0.1.5-rc.2。读取部署说明、包清单、界面静态资源及 LICENSE，不运行参考代码。
- 借鉴中性色背景、紧凑导航/工具栏、分层面板和 12–14px 控件文字；仅参考视觉组织，不复制源码、样式、图标或品牌，不引入新运行依赖。实际界面源码继续由本项目维护。
- 已查看 renderer 包许可证为 MIT；若未来复用代码须保留版权许可并复核其他组件。本次无代码复用。未读取部署配置、访问 token、模型密钥或会话，不调用外部模型；静态 UI 查看不是整个 Harness 的安全审计。
- 此次不是新项目选型：直接替换现有产品不能保证既有功能等价，从零重建无必要；采用在当前平台上做表现层改进的方案。用户已明确授权“功能不变”的优化，不增加 MVP 功能或替换后端。
- 维护日期、Star/Fork、Issue/PR 及线上发布状态本轮未重新核实，不作为选型依据；所述版本仅代表用户本地已安装包。保留既有其他项目参考记录。

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

## v1.0.1 既有采用结论（不包含待确认建议）

- 界面采用统一侧栏导航、按需详情与共享 AI 入口；项目、文档、邮箱和 Skills 分别保留权限边界。参考产品的交互不等于复制其代码。
- 桌面架构已确定为 Tauri/React/Rust/SQLite；Electron 仅作对照，Docling/MarkItDown 未作为运行依赖。文档使用受限 Rust 解析、随包 PDF.js 及结构化 DOCX 导出，不再保留“技术待定”的早期计划。
- GPL/AGPL 等参考项目仅吸收公开产品思路；实际采用依赖的版本、许可和安全结论以本表、锁文件及随包许可证资料为准。
- 最初的产品调研只读公开资料；后续实际采用依赖及发布工具的检查分别记录在对应行。该版参考下载副本和临时发布工具已清理，必要依赖资源与许可证继续保留。
- 表中 Star 为各行调研日期的历史近似值，不代表当前数量或安全背书。“可离线”不等于“默认无外联”；新依赖仍需逐项审计，不以此前调研结论代替检查。

## Windows 云端验证方案实施 · 2026-10-24

已按确认方案复用 [GitHub Windows Server 2022 托管镜像](https://github.com/actions/runner-images/blob/main/images/windows/Windows2022-Readme.md) 的 MSVC、Rust、Node 和 WebView2，以预装环境避开本地评估系统激活与工具目录校验阻塞。流水线仅给仓库读取权限，无个人凭据，锁定 checkout v4.2.2 与 upload-artifact v4.6.2 的官方提交；公开仓库标准 runner，无采购或自建系统镜像。

原生界面采用 [Microsoft WebView2 调试接口](https://learn.microsoft.com/en-us/microsoft-edge/webview2/how-to/debug-visual-studio-code) 的子进程环境参数，仅向本机回环地址开放一次测试端口；Node 内置 WebSocket 驱动已安装的优化版，无新增产品依赖/调试插件，无 IPC mock。关闭应用后释放连接。与 [Tauri 官方 Windows CI](https://v2.tauri.app/develop/tests/webdriver/ci/) 的原生运行边界一致，但本项目使用 CDP 直接驱动，避免下载完整浏览器与另一套驱动。它验证真实 Windows WebView2/Rust 组合，不替代 Win10 实机或普通用户/UAC 检查。

打包工具依据 [tauri-cli v2.11.4 的 NSIS 工具清单](https://github.com/tauri-apps/tauri/blob/tauri-cli-v2.11.4/crates/tauri-bundler/src/bundle/windows/nsis/mod.rs)：官方 nsis-3.11.zip SHA-256 `c7d27f780ddb6cffb4730138cd1591e841f4b7edb155856901cdf5f214394fa1`，nsis_tauri_utils v0.5.3 DLL `5ba143b5db4a87d32d6e7802e033330aae56cbceabe0d1e3ba41948385ad4709`，与 GitHub 资产摘要一致。先校验并扫描压缩包和展开内容，再填入临时 runner 缓存，防止打包器隐式下载后直接执行。未关闭系统保护、增加排除目录或导入测试根证书。

2026-10-24 补充构建路径净化参考：Rust 官方 [路径重映射](https://doc.rust-lang.org/rustc/remap-source-paths.html)、LLVM 官方 [clang-cl](https://clang.llvm.org/docs/UsersManual.html#clang-cl) 与 [file-prefix-map](https://clang.llvm.org/docs/ClangCommandLineReference.html#cmdoption-clang-ffile-prefix-map)、锁定的 aws-lc-sys 0.45.0 `builder/cc_builder.rs`。只参考编译选项：Rust 重映射不覆盖 C 的 `__FILE__`，上游在 cl 模式没有等价净化；Windows 交付构建使用预装 clang-cl 的 MSVC ABI 与 C/C++ 路径映射，并去掉调试信息。LLVM 为 Apache-2.0 WITH LLVM-exception；不下载参考实现、不更换密码算法、不降低交付隐私检查。实际 Windows 结果以流水线证据为准。

2026-10-24 扫描验收参考微软官方 [MpCmdRun 参数](https://learn.microsoft.com/en-us/defender-endpoint/command-line-arguments-microsoft-defender-antivirus)：采用自定义扫描 `-DisableRemediation`，忽略文件排除项并扫描压缩内容，等待进程结束并按退出码阻断使用；仅本次扫描不自动处理检出文件，不修改系统防护、排除项或信任。原先 Start-MpScan 的零检出记录不再作为最终完整扫描证据；以新命令日志和退出码为准。
