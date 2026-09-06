import ReactMarkdown, { defaultUrlTransform } from "react-markdown";
import rehypeSanitize from "rehype-sanitize";
import remarkGfm from "remark-gfm";

type MarkdownMessageProps = {
  allowDataImages?: boolean;
  text: string;
};

export default function MarkdownMessage({ allowDataImages = false, text }: MarkdownMessageProps) {
  return (
    <div className="markdown-message">
      <ReactMarkdown
        skipHtml
        remarkPlugins={[remarkGfm]}
        rehypePlugins={allowDataImages ? [] : [rehypeSanitize]}
        urlTransform={(url, key) => allowDataImages && key === "src" && /^data:image\/(png|jpeg|webp);base64,/i.test(url) ? url : defaultUrlTransform(url)}
        components={{
          a: ({ children, href }) => (
            <span className="markdown-link" title="链接已设为只读，不会自动访问">
              {children}
              {href ? <code>{href}</code> : null}
            </span>
          ),
          img: ({ alt, src }) => allowDataImages && src && /^data:image\/(png|jpeg|webp);base64,/i.test(src)
            ? <img className="note-image" src={src} alt={alt ?? "笔记图片"} />
            : <span className="markdown-blocked-image">图片引用已阻止{alt ? `：${alt}` : ""}</span>,
        }}
      >
        {text}
      </ReactMarkdown>
    </div>
  );
}
