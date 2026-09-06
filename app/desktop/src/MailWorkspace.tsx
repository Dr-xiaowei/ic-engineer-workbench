import { useEffect, useMemo, useState, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import MarkdownMessage from "./MarkdownMessage";
import PdfReader from "./PdfReader";

type MailAccount = {
  id: string; address: string; displayName: string; username: string;
  imapHost: string; imapPort: number; smtpHost: string; smtpPort: number; updatedAt: number;
};
type MailAttachment = { position: number; name: string; mimeType: string; sizeBytes: number };
type MailMessage = {
  accountId: string; folder: string; uid: number; messageId?: string; subject: string;
  sender: string; recipients: string; sentAt?: number; preview: string; bodyText: string;
  unread: boolean; attachments: MailAttachment[];
};
type DraftAccount = Omit<MailAccount, "id" | "updatedAt">;
type Composer = { to: string; cc: string; subject: string; body: string; attachmentPaths: string[] };
type AttachmentPreview = { kind: "pdf" | "markdown"; name: string; content: string };

const emptyDraft: DraftAccount = {
  address: "", displayName: "", username: "", imapHost: "", imapPort: 993,
  smtpHost: "", smtpPort: 465,
};
const emptyComposer: Composer = { to: "", cc: "", subject: "", body: "", attachmentPaths: [] };

function safeError(error: unknown, fallback: string) {
  return typeof error === "string" && error.length <= 240 ? error : fallback;
}
function addresses(value: string) {
  return value.split(/[;,\n]/).map((item) => item.trim()).filter(Boolean);
}
function formattedDate(timestamp?: number) {
  if (!timestamp) return "时间未知";
  return new Intl.DateTimeFormat("zh-CN", { dateStyle: "medium", timeStyle: "short" }).format(new Date(timestamp * 1000));
}
function formatBytes(value: number) {
  return value < 1024 * 1024 ? `${Math.ceil(value / 1024)} KiB` : `${(value / 1024 / 1024).toFixed(1)} MiB`;
}

export default function MailWorkspace() {
  const [accounts, setAccounts] = useState<MailAccount[]>([]);
  const [activeId, setActiveId] = useState("");
  const [draft, setDraft] = useState<DraftAccount>(emptyDraft);
  const [imapPassword, setImapPassword] = useState("");
  const [smtpPassword, setSmtpPassword] = useState("");
  const [notice, setNotice] = useState("");
  const [busy, setBusy] = useState(false);
  const [configured, setConfigured] = useState<Record<string, boolean>>({});
  const [connected, setConnected] = useState<Record<string, boolean>>({});
  const [showAccountForm, setShowAccountForm] = useState(true);
  const [folders, setFolders] = useState<string[]>(["INBOX"]);
  const [folder, setFolder] = useState("INBOX");
  const [messages, setMessages] = useState<MailMessage[]>([]);
  const [selectedUid, setSelectedUid] = useState<number>();
  const [query, setQuery] = useState("");
  const [composer, setComposer] = useState<Composer>();
  const [confirmSend, setConfirmSend] = useState(false);
  const [deleteArmed, setDeleteArmed] = useState(false);
  const [attachmentPreview, setAttachmentPreview] = useState<AttachmentPreview>();
  const active = accounts.find((account) => account.id === activeId) ?? accounts[0];
  const selected = useMemo(() => messages.find((message) => message.uid === selectedUid) ?? messages[0], [messages, selectedUid]);

  async function refresh(preferredId?: string) {
    try {
      const records = await invoke<MailAccount[]>("list_mail_accounts");
      setAccounts(records);
      const selectedAccount = preferredId && records.some((record) => record.id === preferredId) ? preferredId : records[0]?.id ?? "";
      setActiveId(selectedAccount);
      setShowAccountForm(!records.length);
      for (const record of records) {
        invoke<boolean>("has_mail_secrets", { accountId: record.id })
          .then((value) => setConfigured((current) => ({ ...current, [record.id]: value })))
          .catch(() => undefined);
      }
    } catch { setNotice("邮箱数据仅在桌面应用中可用。"); }
  }

  useEffect(() => { void refresh(); }, []);
  useEffect(() => {
    if (!active) { setMessages([]); return; }
    invoke<MailMessage[]>("list_local_mail", { accountId: active.id, folder })
      .then((records) => { setMessages(records); setSelectedUid(records[0]?.uid); })
      .catch(() => setMessages([]));
  }, [active?.id, folder]);

  async function saveAccount(event: FormEvent) {
    event.preventDefault(); setBusy(true);
    try {
      const account = await invoke<MailAccount>("save_mail_account", { account: { ...draft, id: null } });
      await invoke("save_mail_secrets", { accountId: account.id, imapPassword, smtpPassword });
      setConfigured((current) => ({ ...current, [account.id]: true }));
      await refresh(account.id); setDraft(emptyDraft); setImapPassword(""); setSmtpPassword(""); setShowAccountForm(false);
      setNotice("邮箱账户元数据已保存在 SQLite，密码只进入系统凭据库；尚未连接服务器。");
    } catch (error) { setNotice(safeError(error, "无法保存邮箱账户。")); }
    finally { setBusy(false); }
  }

  async function testConnection() {
    if (!active) return; setBusy(true);
    try {
      const result = await invoke<{ success: boolean; imapMessage: string; smtpMessage: string }>("test_mail_account", { accountId: active.id });
      setConnected((current) => ({ ...current, [active.id]: result.success }));
      setNotice(`${result.imapMessage} ${result.smtpMessage}`);
      if (result.success) {
        const records = await invoke<string[]>("list_mail_folders", { accountId: active.id });
        setFolders(records.includes("INBOX") ? records : ["INBOX", ...records]);
      }
    } catch (error) {
      setConnected((current) => ({ ...current, [active.id]: false }));
      setNotice(safeError(error, "邮箱连接检测失败。"));
    } finally { setBusy(false); }
  }

  async function syncFolder() {
    if (!active) return; setBusy(true);
    try {
      const records = await invoke<MailMessage[]>("sync_mail_folder", { accountId: active.id, folder });
      setMessages(records); setSelectedUid(records[0]?.uid);
      setNotice(`已通过只读 IMAP 同步 ${records.length} 封最近邮件，本地全文索引已更新。`);
    } catch (error) { setNotice(safeError(error, "邮件同步失败。")); }
    finally { setBusy(false); }
  }

  async function searchMail(event: FormEvent) {
    event.preventDefault(); if (!active) return;
    try {
      const records = query.trim()
        ? await invoke<MailMessage[]>("search_local_mail", { accountId: active.id, folder, query })
        : await invoke<MailMessage[]>("list_local_mail", { accountId: active.id, folder });
      setMessages(records); setSelectedUid(records[0]?.uid);
      setNotice(query.trim() ? `本地索引找到 ${records.length} 封邮件。` : "已恢复当前文件夹列表。");
    } catch (error) { setNotice(safeError(error, "本地邮件搜索失败。")); }
  }

  async function exportAttachment(attachment: MailAttachment) {
    if (!active || !selected) return;
    const path = await save({ defaultPath: attachment.name });
    if (!path) return;
    try {
      await invoke("export_mail_attachment", { accountId: active.id, folder, uid: selected.uid, position: attachment.position, path });
      setNotice(`附件已导出：${attachment.name}`);
    } catch (error) { setNotice(safeError(error, "附件导出失败。")); }
  }

  async function previewAttachment(attachment: MailAttachment) {
    if (!active || !selected) return;
    try { setAttachmentPreview(await invoke<AttachmentPreview>("preview_mail_attachment", { accountId: active.id, folder, uid: selected.uid, position: attachment.position })); }
    catch (error) { setNotice(safeError(error, "此附件无法安全预览，可选择导出。")); }
  }

  function compose(mode?: "reply" | "forward") {
    if (!selected || !mode) { setComposer(emptyComposer); return; }
    if (mode === "reply") setComposer({ ...emptyComposer, to: selected.sender, subject: `Re: ${selected.subject}`, body: `\n\n--- 原邮件 ---\n${selected.bodyText}` });
    else setComposer({ ...emptyComposer, subject: `Fwd: ${selected.subject}`, body: `\n\n--- 转发邮件 ---\n发件人：${selected.sender}\n收件人：${selected.recipients}\n${selected.bodyText}` });
  }

  async function chooseAttachments() {
    const paths = await open({ multiple: true, directory: false });
    if (!paths || !composer) return;
    const selectedPaths = Array.isArray(paths) ? paths : [paths];
    setComposer({ ...composer, attachmentPaths: [...new Set([...composer.attachmentPaths, ...selectedPaths])].slice(0, 20) });
  }

  async function sendConfirmed() {
    if (!active || !composer) return; setBusy(true);
    try {
      await invoke("send_mail", {
        accountId: active.id,
        draft: { to: addresses(composer.to), cc: addresses(composer.cc), subject: composer.subject, body: composer.body, attachmentPaths: composer.attachmentPaths },
        confirmed: true,
      });
      setComposer(undefined); setConfirmSend(false); setNotice("SMTP 已确认接收邮件。可同步已发送文件夹核对服务端副本。");
    } catch (error) { setNotice(safeError(error, "邮件发送失败。")); setConfirmSend(false); }
    finally { setBusy(false); }
  }

  async function deleteAccount() {
    if (!active) return;
    if (!deleteArmed) { setDeleteArmed(true); setNotice("再次点击“确认移除”才会删除本机账户缓存和系统凭据；服务器邮件不受影响。"); return; }
    try {
      await invoke("delete_mail_account", { accountId: active.id });
      setDeleteArmed(false); setFolders(["INBOX"]); setFolder("INBOX"); await refresh(); setNotice("本机邮箱账户、缓存和凭据已移除；未操作服务器邮件。");
    } catch (error) { setNotice(safeError(error, "无法移除邮箱账户。")); }
  }

  return (
    <section className="mail-workspace" aria-labelledby="mail-title">
      <header className="workspace-action-header">
        <div><p className="eyebrow">INTRANET MAIL CLIENT</p><h2 id="mail-title">本地邮箱</h2><p>邮件只读同步并缓存在本机；密码只存系统凭据库，发送前始终再次人工确认。</p></div>
        <div className="mail-header-actions">
          {active ? <button className="secondary-button" type="button" onClick={() => compose()}>写邮件</button> : null}
          {active ? <button className="primary-button" type="button" disabled={busy || !configured[active.id]} onClick={testConnection}>{busy ? "处理中…" : "检测 IMAP / SMTP"}</button> : null}
        </div>
      </header>
      {notice ? <p className="converter-notice" role="status">{notice}</p> : null}
      <div className="mail-layout">
        <aside className="mail-accounts">
          <div className="mail-aside-title"><p className="eyebrow">ACCOUNTS</p><button type="button" aria-label="添加邮箱账户" onClick={() => setShowAccountForm(true)}>＋</button></div>
          {accounts.map((account) => <button className={account.id === active?.id ? "active" : ""} key={account.id} type="button" onClick={() => { setActiveId(account.id); setShowAccountForm(false); setFolder("INBOX"); }}><span>{connected[account.id] ? "● 已验证" : configured[account.id] ? "○ 已配置" : "○ 缺少凭据"}</span><strong>{account.displayName}</strong><small>{account.address}</small></button>)}
          {!accounts.length ? <div className="conversion-empty">尚未添加邮箱</div> : null}
          {active && !showAccountForm ? <><p className="mail-folder-label">文件夹</p>{folders.map((item) => <button className={item === folder ? "active" : ""} key={item} type="button" onClick={() => setFolder(item)}><strong>{item}</strong></button>)}<button className="mail-delete" type="button" onClick={deleteAccount}>{deleteArmed ? "确认移除" : "移除本机账户"}</button></> : null}
        </aside>
        <main className="mail-main">
          {showAccountForm ? (
            <form className="mail-account-form" onSubmit={saveAccount}>
              <div className="mail-form-heading"><h3>添加内网邮箱账户</h3>{accounts.length ? <button type="button" onClick={() => setShowAccountForm(false)}>取消</button> : null}</div>
              <div className="mail-form-grid">
                <label>显示名称<input required value={draft.displayName} onChange={(e) => setDraft({ ...draft, displayName: e.target.value })} /></label>
                <label>邮箱地址<input required type="email" value={draft.address} onChange={(e) => setDraft({ ...draft, address: e.target.value })} /></label>
                <label>登录用户名<input required value={draft.username} onChange={(e) => setDraft({ ...draft, username: e.target.value })} /></label><span />
                <label>IMAP 内网主机<input required value={draft.imapHost} onChange={(e) => setDraft({ ...draft, imapHost: e.target.value })} placeholder="mail.internal" /></label>
                <label>IMAPS 端口<input required type="number" min="1" max="65535" value={draft.imapPort} onChange={(e) => setDraft({ ...draft, imapPort: Number(e.target.value) })} /></label>
                <label>SMTP 内网主机<input required value={draft.smtpHost} onChange={(e) => setDraft({ ...draft, smtpHost: e.target.value })} placeholder="mail.internal" /></label>
                <label>SMTP 端口<input required type="number" min="1" max="65535" value={draft.smtpPort} onChange={(e) => setDraft({ ...draft, smtpPort: Number(e.target.value) })} /></label>
                <label>IMAP 密码<input required type="password" autoComplete="off" value={imapPassword} onChange={(e) => setImapPassword(e.target.value)} /></label>
                <label>SMTP 密码<input required type="password" autoComplete="off" value={smtpPassword} onChange={(e) => setSmtpPassword(e.target.value)} /></label>
              </div>
              <p>只允许私有地址、本机或内网主机；IMAP 强制 TLS，SMTP 仅允许 SMTPS 或强制 STARTTLS。</p>
              <button className="primary-button" type="submit" disabled={busy}>{busy ? "保存中…" : "安全保存账户"}</button>
            </form>
          ) : active ? (
            <div className="mail-reader">
              <section className="mail-list-panel">
                <div className="mail-list-toolbar"><form onSubmit={searchMail}><input aria-label="搜索本地邮件" value={query} onChange={(e) => setQuery(e.target.value)} placeholder="搜索主题、发件人、正文"/><button type="submit">搜索</button></form><button type="button" disabled={busy || !configured[active.id]} onClick={syncFolder}>{busy ? "同步中…" : "同步"}</button></div>
                <div className="mail-message-list">{messages.map((message) => <button className={`${message.uid === selected?.uid ? "active" : ""} ${message.unread ? "unread" : ""}`} key={message.uid} type="button" onClick={() => setSelectedUid(message.uid)}><span>{message.sender || "未知发件人"}<time>{formattedDate(message.sentAt)}</time></span><strong>{message.subject}</strong><p>{message.preview || "无文本预览"}</p></button>)}{!messages.length ? <div className="mail-list-empty"><strong>{active.address}</strong><p>本地尚无邮件。连接后同步，或搜索已有缓存。</p></div> : null}</div>
              </section>
              <article className="mail-viewer">{selected ? <><header><p>{selected.sender}</p><h3>{selected.subject}</h3><small>收件人：{selected.recipients || active.address} · {formattedDate(selected.sentAt)}</small><div><button type="button" onClick={() => compose("reply")}>回复</button><button type="button" onClick={() => compose("forward")}>转发</button></div></header><pre>{selected.bodyText || "（无纯文本正文）"}</pre>{selected.attachments.length ? <section className="mail-attachments"><h4>附件</h4>{selected.attachments.map((attachment) => <div className="mail-attachment-row" key={attachment.position}><span><strong>{attachment.name}</strong><small>{attachment.mimeType} · {formatBytes(attachment.sizeBytes)}</small></span><button type="button" onClick={() => previewAttachment(attachment)}>预览</button><button type="button" onClick={() => exportAttachment(attachment)}>导出</button></div>)}</section> : null}</> : <div className="mail-empty"><span>✉</span><strong>{active.address}</strong><p>选择一封本地邮件查看纯文本正文。</p></div>}</article>
            </div>
          ) : null}
        </main>
      </div>
      {composer ? <div className="mail-modal-backdrop"><form className="mail-composer" onSubmit={(event) => { event.preventDefault(); setConfirmSend(true); }}><header><div><p className="eyebrow">COMPOSE</p><h3>写邮件</h3></div><button type="button" aria-label="关闭写邮件" onClick={() => setComposer(undefined)}>×</button></header><label>收件人<input required value={composer.to} onChange={(e) => setComposer({ ...composer, to: e.target.value })} placeholder="可用逗号或分号分隔" /></label><label>抄送<input value={composer.cc} onChange={(e) => setComposer({ ...composer, cc: e.target.value })} /></label><label>主题<input required maxLength={500} value={composer.subject} onChange={(e) => setComposer({ ...composer, subject: e.target.value })} /></label><label>正文<textarea required value={composer.body} onChange={(e) => setComposer({ ...composer, body: e.target.value })} /></label><div className="mail-compose-files"><button type="button" onClick={chooseAttachments}>选择本地附件</button>{composer.attachmentPaths.map((path) => <small key={path}>{path.split(/[\\/]/).pop()}</small>)}</div><footer><span>发送会连接 {active?.smtpHost}:{active?.smtpPort}</span><button className="primary-button" type="submit" disabled={busy}>检查并准备发送</button></footer></form></div> : null}
      {confirmSend && composer ? <div className="mail-modal-backdrop mail-confirm"><section role="dialog" aria-modal="true" aria-labelledby="send-confirm-title"><p className="eyebrow">HUMAN CONFIRMATION</p><h3 id="send-confirm-title">确认发送这封邮件？</h3><p>收件人：{addresses(composer.to).join("、")}<br/>主题：{composer.subject}<br/>附件：{composer.attachmentPaths.length} 个</p><div><button type="button" onClick={() => setConfirmSend(false)}>返回修改</button><button className="primary-button" type="button" disabled={busy} onClick={sendConfirmed}>{busy ? "发送中…" : "确认并发送"}</button></div></section></div> : null}
      {attachmentPreview ? <div className="mail-modal-backdrop"><section className="attachment-preview" role="dialog" aria-modal="true" aria-label="邮件附件预览"><header><div><p className="eyebrow">SAFE ATTACHMENT PREVIEW</p><h3>{attachmentPreview.name}</h3></div><button onClick={()=>setAttachmentPreview(undefined)}>×</button></header>{attachmentPreview.kind==="pdf"?<PdfReader title="邮件 PDF 附件" source={attachmentPreview.content}/>:<MarkdownMessage text={attachmentPreview.content}/>}</section></div>:null}
    </section>
  );
}
