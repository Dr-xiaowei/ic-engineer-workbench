import { useEffect, useMemo, useState, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import MarkdownMessage from "./MarkdownMessage";
import { loadModelEndpoints } from "./model-config";
import { useVerifiedEndpointIds } from "./model-session";

export type AssistantPageContext = {
  id: string;
  label: string;
  summary: string;
};

type ChatResult = {
  content: string;
  requestId?: string;
};

type FloatingAssistantProps = {
  context: AssistantPageContext;
  onOpenChat: () => void;
};

function safeErrorMessage(error: unknown): string {
  return typeof error === "string" && error.length <= 240
    ? error
    : "浮空助手请求失败，请检查模型连接后重试。";
}

export default function FloatingAssistant({ context, onOpenChat }: FloatingAssistantProps) {
  const [open, setOpen] = useState(false);
  const [authorizedPageId, setAuthorizedPageId] = useState("");
  const [question, setQuestion] = useState("");
  const [answer, setAnswer] = useState("");
  const [requestId, setRequestId] = useState("");
  const [error, setError] = useState("");
  const [sending, setSending] = useState(false);
  const verifiedEndpointIds = useVerifiedEndpointIds();
  const endpoint = useMemo(
    () =>
      loadModelEndpoints().find(
        (item) => item.enabled && verifiedEndpointIds.includes(item.id),
      ),
    [verifiedEndpointIds],
  );
  const authorized = authorizedPageId === context.id;

  useEffect(() => {
    setAuthorizedPageId("");
    setQuestion("");
    setAnswer("");
    setRequestId("");
    setError("");
  }, [context.id]);

  function closePanel() {
    setOpen(false);
    setAuthorizedPageId("");
    setQuestion("");
    setAnswer("");
    setRequestId("");
    setError("");
  }

  async function submitQuestion(event: FormEvent) {
    event.preventDefault();
    const prompt = question.trim();
    if (!authorized || !endpoint || !prompt || sending) return;
    setSending(true);
    setAnswer("");
    setRequestId("");
    setError("");
    try {
      const result = await invoke<ChatResult>("send_chat_completion", {
        endpointId: endpoint.id,
        baseUrl: endpoint.baseUrl,
        model: endpoint.model,
        messages: [
          {
            role: "system",
            content:
              "你是芯智工作台的页面助手。只根据用户本次明确授权的当前页面摘要回答；信息不足时直接说明，不推测其他页面、文件、剪贴板或系统内容。",
            attachments: [],
          },
          {
            role: "user",
            content: `当前页面：${context.label}\n授权页面摘要：${context.summary}\n\n问题：${prompt}`,
            attachments: [],
          },
        ],
      });
      setAnswer(result.content);
      setRequestId(result.requestId ?? "");
    } catch (requestError) {
      setError(safeErrorMessage(requestError));
    } finally {
      setSending(false);
    }
  }

  return (
    <>
      <button
        className="floating-assistant-trigger"
        type="button"
        aria-label="打开浮空助手"
        aria-expanded={open}
        onClick={() => setOpen(true)}
      >
        AI
      </button>
      {open ? (
        <aside className="floating-assistant" role="dialog" aria-labelledby="floating-assistant-title">
          <header>
            <div>
              <p className="eyebrow">EXPLICIT PAGE CONTEXT</p>
              <h2 id="floating-assistant-title">浮空助手</h2>
            </div>
            <button className="icon-button" type="button" aria-label="关闭浮空助手" onClick={closePanel}>
              ×
            </button>
          </header>

          <section className={`assistant-consent ${authorized ? "authorized" : ""}`}>
            <div>
              <strong>{context.label}</strong>
              <span>{authorized ? "已授权本页 · 关闭或切页即撤销" : "尚未授权页面内容"}</span>
            </div>
            {authorized ? (
              <button type="button" className="text-button" onClick={() => setAuthorizedPageId("")}>
                撤销
              </button>
            ) : (
              <button type="button" className="secondary-button" onClick={() => setAuthorizedPageId(context.id)}>
                授权当前页面
              </button>
            )}
          </section>

          {authorized ? <p className="assistant-context-preview">{context.summary}</p> : null}

          {!endpoint ? (
            <div className="assistant-model-warning">
              <p>需要先在智能对话中完成一次模型连接检测。本次页面内容尚未发送。</p>
              <button className="text-button" type="button" onClick={onOpenChat}>前往模型设置</button>
            </div>
          ) : null}

          <form onSubmit={submitQuestion}>
            <label htmlFor="floating-question">仅询问当前页面</label>
            <textarea
              id="floating-question"
              rows={4}
              maxLength={4000}
              value={question}
              disabled={!authorized || sending}
              placeholder={authorized ? "输入关于当前页面的问题…" : "授权后才能输入问题"}
              onChange={(event) => setQuestion(event.target.value)}
            />
            <button className="primary-button" type="submit" disabled={!authorized || !endpoint || !question.trim() || sending}>
              {sending ? "分析中…" : "发送本页问题"}
            </button>
          </form>

          {error ? <p className="assistant-error">{error}</p> : null}
          {answer ? (
            <section className="assistant-answer" aria-label="浮空助手回答">
              <MarkdownMessage text={answer} />
              {requestId ? <small>请求 ID：{requestId}</small> : null}
            </section>
          ) : null}

          <footer>不读取屏幕、剪贴板、DOM、其他应用或未授权文件。</footer>
        </aside>
      ) : null}
    </>
  );
}
