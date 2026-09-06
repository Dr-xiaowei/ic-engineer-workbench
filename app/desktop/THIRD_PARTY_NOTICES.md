# 第三方依赖与许可证说明

本文件记录首版源码直接依赖和随包资源。确切版本以 `pnpm-lock.yaml` 与 `src-tauri/Cargo.lock` 为准；构建交付前应基于这两个锁文件生成完整的传递依赖 SBOM，并在目标公司流程中执行漏洞库扫描。

## 界面层直接依赖

| 依赖 | 用途 | 许可证 |
| --- | --- | --- |
| React / React DOM | 本地客户端界面 | MIT |
| @tauri-apps/api / plugin-dialog | Tauri IPC 与系统文件对话框 | MIT / Apache-2.0 |
| react-markdown | 安全 Markdown 渲染 | MIT |
| remark-gfm | GFM 语法支持 | MIT |
| rehype-sanitize | 不可信 Markdown 清洗 | MIT |
| pdfjs-dist 6.3.289 | 离线 PDF 分页渲染与字体资源 | Apache-2.0；字体/CMap 含各自许可文件，原文随资源打包 |
| docx 9.7.1 | 笔记标题、表格、图片的 OOXML 导出 | MIT |
| Vite / TypeScript / Vitest / Testing Library | 构建、类型检查和测试，仅开发使用 | MIT（各包以锁文件元数据为准） |

## Rust 直接依赖

| 依赖 | 用途 | 许可证 |
| --- | --- | --- |
| tauri / tauri-plugin-dialog | 桌面壳与文件选择 | MIT / Apache-2.0 |
| reqwest / url | 受控内网模型 HTTP 与 URL 校验 | MIT / Apache-2.0 |
| keyring | 操作系统凭据库 | MIT / Apache-2.0 |
| rusqlite（bundled SQLite） | 本地数据库、FTS、在线备份 | MIT；SQLite 为 Public Domain |
| async-imap / tokio / tokio-native-tls / native-tls | IMAPS 客户端与 TLS | MIT 或 MIT / Apache-2.0（以锁定包元数据为准） |
| lettre | SMTPS/STARTTLS 邮件发送 | MIT |
| mail-parser | MIME 邮件解析 | MIT / Apache-2.0 |
| pdf-extract / lopdf / printpdf | PDF 文本抽取与可搜索 PDF 导出 | MIT |
| quick-xml / zip | DOCX XML 与压缩容器解析 | MIT；zip 为 MIT |
| serde / serde_json | 类型化数据序列化 | MIT / Apache-2.0 |
| sha2 / base64 | 内容标识与受限附件编码 | MIT / Apache-2.0 |
| futures-util / walkdir | 异步流与受限目录遍历 | MIT / Apache-2.0；walkdir 为 Unlicense / MIT |

## 随包资源

- `src-tauri/assets/fonts/NotoSansCJKsc-Regular.otf`：Noto Sans CJK SC，用于中文 PDF 嵌入，SIL Open Font License 1.1。
- `src-tauri/assets/fonts/OFL.txt`：上述字体许可证原文，必须与字体共同保留。

本项目未复制 AGPL/GPL 参考项目代码。Mailspring、Cherry Studio 与 AppFlowy 仅用于公开产品思路和界面模式分析。
