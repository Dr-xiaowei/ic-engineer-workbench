import { useEffect, useRef, useState } from "react";
import { getDocument, GlobalWorkerOptions, type PDFDocumentProxy, type RenderTask } from "pdfjs-dist";
import workerUrl from "pdfjs-dist/build/pdf.worker.min.mjs?url";

GlobalWorkerOptions.workerSrc = workerUrl;

/** Bundled renderer; document bytes are supplied by the authorized Rust loader. */
export default function PdfReader({ source, title }: { source: string; title: string }) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const [document, setDocument] = useState<PDFDocumentProxy>();
  const [page, setPage] = useState(1);
  const [zoom, setZoom] = useState(1);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    setDocument(undefined); setPage(1); setError("");
    if (!source.startsWith("data:application/pdf;base64,") || source.length > 48 * 1024 * 1024) {
      setError("PDF 数据无效或超过阅读上限。"); return;
    }
    let disposed = false;
    const task = getDocument({ data: Uint8Array.from(atob(source.split(",")[1]), c => c.charCodeAt(0)), useWasm: false, useSystemFonts: false, disableFontFace: true, useWorkerFetch: false, standardFontDataUrl: new URL("/pdfjs/standard_fonts/", window.location.href).href, cMapUrl: new URL("/pdfjs/cmaps/", window.location.href).href, cMapPacked: true });
    void task.promise.then(value => { if (!disposed) setDocument(value); }).catch(() => { if (!disposed) setError("无法读取此 PDF，请使用按页文本视图，或检查文件是否损坏、加密。"); });
    return () => { disposed = true; void task.destroy(); };
  }, [source]);
  useEffect(() => {
    if (!document || !canvas.current) return;
    let cancelled = false;
    let render: RenderTask | undefined;
    setBusy(true);
    void document.getPage(page).then(value => {
      if (cancelled || !canvas.current) return;
      const natural = value.getViewport({ scale: 1 });
      const scale = Math.min(zoom * Math.min(window.devicePixelRatio || 1, 2), 4096 / natural.width, 4096 / natural.height);
      const viewport = value.getViewport({ scale });
      const target = canvas.current;
      target.width = viewport.width; target.height = viewport.height;
      // Static reader: avoid RAF suspension when the native window is backgrounded.
      render = value.render({ canvas: target, viewport, intent: "print" });
      return render.promise;
    }).then(() => { if (!cancelled) setBusy(false); }).catch(() => { if (!cancelled) { setBusy(false); setError("此页无法渲染，请尝试其他页或按页文本视图。"); } });
    return () => { cancelled = true; render?.cancel(); };
  }, [document, page, zoom]);
  return <section className="pdf-reader" aria-label={title}><div className="pdf-toolbar"><button disabled={!document || page <= 1} onClick={() => setPage(page - 1)}>上一页</button><span>{page} / {document?.numPages ?? "—"}</span><button disabled={!document || page >= document.numPages} onClick={() => setPage(page + 1)}>下一页</button><button disabled={zoom <= .5} onClick={() => setZoom(Math.max(.5, zoom - .25))}>缩小</button><span>{Math.round(zoom * 100)}%</span><button disabled={zoom >= 2} onClick={() => setZoom(Math.min(2, zoom + .25))}>放大</button>{busy ? <span role="status">正在渲染…</span> : null}</div>{error ? <p role="alert">{error}</p> : null}<div className="pdf-canvas-scroll"><canvas ref={canvas} style={{width:`${zoom*100}%`}} aria-label={`${title} 第 ${page} 页`} /></div></section>;
}
