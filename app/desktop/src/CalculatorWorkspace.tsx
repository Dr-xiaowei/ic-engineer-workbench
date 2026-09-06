import { useState, type FormEvent } from "react";
import { calculateExpression, type AngleMode } from "./calculator";

type HistoryItem = { expression: string; result: string };
const historyKey = "ic-workbench-calculator-history-v1";
function loadHistory(): HistoryItem[] { try { const value = JSON.parse(localStorage.getItem(historyKey) ?? "[]"); return Array.isArray(value) ? value.slice(0, 20) : []; } catch { return []; } }

export default function CalculatorWorkspace() {
  const [expression, setExpression] = useState("");
  const [angleMode, setAngleMode] = useState<AngleMode>("deg");
  const [result, setResult] = useState("");
  const [error, setError] = useState("");
  const [history, setHistory] = useState<HistoryItem[]>(loadHistory);
  function calculate(event?: FormEvent) {
    event?.preventDefault(); setError("");
    try {
      const value = calculateExpression(expression, angleMode);
      const text = Number.isInteger(value) ? String(value) : Number(value.toPrecision(14)).toString();
      setResult(text);
      const next = [{ expression, result: text }, ...history.filter((item) => item.expression !== expression)].slice(0, 20);
      setHistory(next); localStorage.setItem(historyKey, JSON.stringify(next));
    } catch (reason) { setResult(""); setError(reason instanceof Error ? reason.message : "无法计算表达式。"); }
  }
  const insert = (value: string) => setExpression((current) => `${current}${value}`.slice(0, 500));
  const backspace = () => setExpression((current) => Array.from(current).slice(0, -1).join(""));
  return <section className="tool-workspace calculator-workspace" aria-labelledby="calculator-title">
    <header className="workspace-action-header"><div><p className="eyebrow">SAFE LOCAL MATH</p><h2 id="calculator-title">科学计算器</h2><p>表达式只在当前设备解析，不使用 eval，也不会发送到模型或网络。</p></div><div className="segmented"><button className={angleMode === "deg" ? "active" : ""} onClick={() => setAngleMode("deg")}>DEG</button><button className={angleMode === "rad" ? "active" : ""} onClick={() => setAngleMode("rad")}>RAD</button></div></header>
    <div className="calculator-layout"><form className="calculator-panel" onSubmit={calculate}><label>表达式<textarea aria-label="科学计算表达式" placeholder="直接输入，例如：(1.8 * 2) + sqrt(4)" value={expression} onChange={(e) => setExpression(e.target.value)} maxLength={500} rows={3}/></label><output aria-live="polite">{error ? <span className="danger-text">{error}</span> : result}</output><div className="calculator-keys">{["(",")",",","⌫","sin(","cos(","tan(","sqrt(","asin(","acos(","atan(","abs(","ln(","log(","exp(","pow(","min(","max(","7","8","9","/","%","4","5","6","*","^","1","2","3","-","pi","0",".","+","e"] .map((key) => <button type="button" key={key} onClick={() => key === "⌫" ? backspace() : insert(key)}>{key}</button>)}</div><div className="calculator-actions"><button type="button" className="text-button" onClick={() => { setExpression(""); setResult(""); setError(""); }}>清空</button><button type="submit" className="primary-button" disabled={!expression.trim()}>计算</button></div></form><aside className="calculator-history"><header><strong>本地历史</strong><button type="button" onClick={() => { setHistory([]); localStorage.removeItem(historyKey); }}>清除</button></header>{history.map((item, index) => <button type="button" key={`${item.expression}-${index}`} onClick={() => setExpression(item.expression)}><span>{item.expression}</span><strong>= {item.result}</strong></button>)}{!history.length ? <p>尚无计算记录</p> : null}</aside></div>
  </section>;
}
