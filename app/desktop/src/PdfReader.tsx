import { useEffect, useRef, useState, type CSSProperties } from "react";
import { getDocument, GlobalWorkerOptions, TextLayer, type PDFDocumentProxy, type RenderTask } from "pdfjs-dist";
import workerUrl from "pdfjs-dist/build/pdf.worker.min.mjs?url";
GlobalWorkerOptions.workerSrc = workerUrl;

type Props = { source: string; title: string; page?: number; zoom?: number; onPageChange?: (page: number) => void; onZoomChange?: (zoom: number) => void; onQuote?: (text: string, page: number) => void };
/** The canvas and selectable text share one viewport. No PDF actions or links execute. */
export default function PdfReader(props: Props) {
  const { source, title, onQuote } = props;
  const canvas = useRef<HTMLCanvasElement>(null), textContainer = useRef<HTMLDivElement>(null), scroll = useRef<HTMLDivElement>(null);
  const [document, setDocument] = useState<PDFDocumentProxy>();
  const [localPage, setLocalPage] = useState(1), [localZoom, setLocalZoom] = useState(1);
  const page = props.page ?? localPage, zoom = props.zoom ?? localZoom;
  const setPage = (p: number) => { const next = Math.max(1, Math.min(document?.numPages ?? 1, p)); setLocalPage(next); props.onPageChange?.(next); };
  const setZoom = (z: number) => { setLocalZoom(z); props.onZoomChange?.(z); };
  const [error, setError] = useState(""), [busy, setBusy] = useState(false), [selection, setSelection] = useState("");
  const [width, setWidth] = useState(600), [size, setSize] = useState({ width: 600, height: 780, scale: 1 });
  useEffect(() => {
    if (!scroll.current || typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(entries => setWidth(Math.max(100, entries[0].contentRect.width - 16)));
    observer.observe(scroll.current); return () => observer.disconnect();
  }, []);
  useEffect(() => {
    setDocument(undefined); setLocalPage(1); setError(""); setSelection("");
    if (!source.startsWith("data:application/pdf;base64,") || source.length > 48 * 1024 * 1024) { setError("PDF 数据无效或超过阅读上限。"); return; }
    let disposed = false;
    const task = getDocument({ data: Uint8Array.from(atob(source.split(",")[1]), c => c.charCodeAt(0)), useWasm: false, useSystemFonts: false, disableFontFace: true, useWorkerFetch: false, standardFontDataUrl: new URL("/pdfjs/standard_fonts/", window.location.href).href, cMapUrl: new URL("/pdfjs/cmaps/", window.location.href).href, cMapPacked: true });
    void task.promise.then(value => { if (!disposed) setDocument(value); }).catch(() => { if (!disposed) setError("无法读取此 PDF，请使用按页文本视图，或检查文件是否损坏、加密。"); });
    return () => { disposed = true; void task.destroy(); };
  }, [source]);
  useEffect(() => {
    if (!document || !canvas.current || !textContainer.current) return;
    let cancelled = false, render: RenderTask | undefined, layer: TextLayer | undefined;
    setBusy(true); setSelection(""); setError("");
    const container = textContainer.current; container.replaceChildren();
    void document.getPage(Math.min(page, document.numPages)).then(async value => {
      if (cancelled || !canvas.current) return;
      const natural = value.getViewport({ scale: 1 });
      const scale = Math.min(width / natural.width, 1) * zoom;
      const viewport = value.getViewport({ scale });
      const pixels = value.getViewport({ scale: Math.min(scale * Math.min(window.devicePixelRatio || 1, 2), 4096 / natural.width, 4096 / natural.height) });
      setSize({ width: viewport.width, height: viewport.height, scale });
      canvas.current.width = pixels.width; canvas.current.height = pixels.height;
      render = value.render({ canvas: canvas.current, viewport: pixels, intent: "print" });
      const content = await value.getTextContent();
      if (cancelled) return;
      layer = new TextLayer({ textContentSource: content, container, viewport });
      await Promise.all([render.promise, layer.render()]);
    }).then(() => { if (!cancelled) setBusy(false); }).catch(() => { if (!cancelled) { setBusy(false); setError("此页渲染或选文不可用，请使用按页文本视图。"); } });
    return () => { cancelled = true; render?.cancel(); layer?.cancel(); container.replaceChildren(); };
  }, [document, page, zoom, width]);
  function captureSelection() { const chosen = window.getSelection(); if (chosen?.rangeCount && textContainer.current?.contains(chosen.getRangeAt(0).commonAncestorContainer)) setSelection(chosen.toString().slice(0, 8000)); }
  return <section className="pdf-reader" aria-label={title}>
    <div className="pdf-toolbar"><button disabled={!document || page <= 1} onClick={() => setPage(page - 1)}>上一页</button><label>页码 <input type="number" aria-label="跳转到页码" min={1} max={document?.numPages ?? 1} value={page} onChange={e => setPage(Number(e.target.value) || 1)} /></label><span>/ {document?.numPages ?? "—"}</span><button disabled={!document || page >= document.numPages} onClick={() => setPage(page + 1)}>下一页</button><button disabled={zoom <= .5} onClick={() => setZoom(Math.max(.5, zoom - .25))}>缩小</button><span>{Math.round(zoom * 100)}%</span><button disabled={zoom >= 2} onClick={() => setZoom(Math.min(2, zoom + .25))}>放大</button>{busy ? <span role="status">正在渲染…</span> : null}</div>
    {error ? <p role="alert">{error}</p> : null}
    {onQuote ? <div className="pdf-quote-bar"><span>{selection ? `已选择 ${selection.length} 字` : "选中原文后，可保留为引用；选文不准确时使用按页文本。"}</span><button disabled={!selection.trim()} onClick={() => { onQuote(selection, page); setSelection(""); }}>保留原文引用</button></div> : null}
    <div className="pdf-canvas-scroll" ref={scroll}><div className="pdf-page-surface" style={{ width: size.width, height: size.height, "--total-scale-factor": size.scale } as CSSProperties}><canvas ref={canvas} aria-label={`${title} 第 ${page} 页`} /><div className="pdf-text-layer" ref={textContainer} onMouseDown={() => setSelection("")} onMouseUp={captureSelection} onKeyUp={captureSelection} /></div></div>
  </section>;
}
