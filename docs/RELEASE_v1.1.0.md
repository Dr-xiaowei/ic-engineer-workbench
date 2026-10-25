# 芯智工作台 v1.1.0

面向模拟 IC 设计日常工作的本地工作台。2026-10-25 经用户确认将现有成果作为正式版本发布。本次不追加功能验证。

## 更新内容

- 自动延续未提交草稿、阅读位置和搜索状态，减少跨页切换后的重复操作。
- 将原文证据保存为项目评审笔记，经确认再生成待办，便于回查来源。
- 提供设计评审、Datasheet 核查、故障排查、项目周报四个可定制模板。
- 五组主导航和常用固定入口，保留高级配置和原有功能。
- 统一 PDF（有文本层）、Word DOCX、PowerPoint PPTX、TXT、Markdown 的基本内容互转。无需 AI 或 Office，另存新文件，不覆盖原件。
- 增加 Windows x64 普通版/Demo，修复 Windows 路径、系统凭据及发布包中的开发路径问题。

## 下载与安装

| 版本 | 下载文件 |
| --- | --- |
| macOS Apple Silicon 普通版 | `ic-workbench-v1.1.0-macos-arm64.zip` |
| macOS Apple Silicon Demo | `ic-workbench-demo-v1.1.0-macos-arm64.zip` |
| Windows x64 普通版 | `ic-workbench-v1.1.0-windows-x64-setup.exe` |
| Windows x64 Demo | `ic-workbench-demo-v1.1.0-windows-x64-setup.exe` |

下载后先将文件的 SHA-256 与 `SHA256SUMS.txt` 比较。macOS 解压后放入个人 Applications；Windows 运行当前用户安装程序，需已安装 [Microsoft WebView2 Evergreen Runtime](https://developer.microsoft.com/microsoft-edge/webview2/)。安装包不含 WebView2，以减小下载量；没有 Node、Rust、Office 或 AI 模型依赖。普通版默认离线，使用 AI 时自行配置本地或内网兼容服务。Demo 有独立合成数据和内置演示服务。

macOS 为 ad-hoc 签名，尚无 Developer ID 公证；Windows 未做 Authenticode 签名。Gatekeeper、SmartScreen 或公司终端策略可能提示，按组织批准的方式使用，不要全局关闭安全保护。

## 升级与个人数据

1. 在旧应用设置中导出工作台备份，并保留该文件与旧应用，关闭旧应用后安装新版。原工程、附件及映射文件继续留在原位置。
2. macOS 旧正式版升级：替换应用即可，普通版与 Demo 继续使用各自原有数据目录。原有 schema 9→10 迁移会先保存旧数据库，迁移逻辑本次未改动。
3. 开发预览转正式版：预览标识和目录与正式版不同，不会自动合并或删除。先导出预览备份，再在正式版设置中显式恢复；恢复会替换目标应用的当前工作台数据，若目标已有资料，先单独导出目标备份。
4. 备份不含密码或密钥，也不复制所有外部原件。恢复后网络关闭，需重新配置系统凭据，确认原文件路径，再按需开启内网访问。
5. 普通版与 Demo 数据互不合并。不要用旧应用打开已升级的数据库；需要回退时使用保留的旧备份。Windows 正式版身份从本次起固定，后续更新应延续该身份和迁移规则。

本次不删除个人数据、预览数据或历史 Release。已有候选流程验证过 schema 9 迁移、备份恢复及预览版本升级/卸载重装的数据保持；预览→正式通道迁移和本次重新打包未追加实机验收。

## 已有证据与限制

本次进行正式构建、安全扫描和文件完整性核对，没有重跑功能测试。Windows 既有候选 [运行 40087540364](https://github.com/Dr-xiaowei/ic-engineer-workbench/actions/runs/40087540364) 在 Server 2022 x64 通过 71 项前端、4 项协议、54 项 Rust 常规、6 项显式原生专项、3 项系统识别守卫和 36 项安装后界面/IPC 检查；包括双模式候选升级与卸载重装的数据保持。Mac 的已有本地小模型、凭据、TLS 邮件、文档和备份证据见完成度报告。

- 没有完成 Win10 工作站、普通用户/UAC/SmartScreen、全部交互选择器/DPI、Windows 真实模型或实际岗位效率的全面验收；不承诺全部系统和硬件无缺陷。
- 文档互转经 Markdown 重排，不保证图片、动画、复杂公式、合并单元格或原始版式保真。没有 OCR，不解析旧二进制 `.doc/.ppt`；工程数值与单位须对照原件。
- AI 输出需要人工复核；不替代 EDA 仿真、工程签核或公司真实环境验收。

详细范围见 [完成度报告](../COMPLETION_REPORT.md) 与 [Windows 验证说明](WINDOWS_VALIDATION.md)。
