// Exercise the installed release binary and its real Rust IPC through WebView2.
// No production test plugin, IPC mocks, model download or remote browser is used.
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { createServer } from "node:net";
import { join, resolve } from "node:path";
import { setTimeout as delay } from "node:timers/promises";

if (process.platform !== "win32" || process.env.GITHUB_ACTIONS !== "true") throw new Error("This destructive synthetic-data suite runs only in disposable Windows CI.");
const [exe, mode, phase, version, fixtureDir, evidenceDir] = process.argv.slice(2);
if (!exe || !["normal", "demo"].includes(mode) || !["initial", "upgrade", "reinstall"].includes(phase) || !fixtureDir || !evidenceDir) throw new Error("Expected executable mode phase version fixtures evidence.");
const output = resolve(evidenceDir);
mkdirSync(output, { recursive: true });
const report = { mode, phase, version, status: "running", checks: [], startedAt: new Date().toISOString(), scope: "Windows Server native UI and installed preview; not Win10/UAC acceptance" };
const save = () => writeFileSync(join(output, `${mode}-${phase}.json`), JSON.stringify(report, null, 2));
const hash = (p) => createHash("sha256").update(readFileSync(p)).digest("hex");
const listener = createServer();
await new Promise((ok, fail) => { listener.once("error", fail); listener.listen(0, "127.0.0.1", ok); });
const port = listener.address().port;
await new Promise((ok) => listener.close(ok));
const started = performance.now();
const app = spawn(exe, [], { env: { ...process.env, WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port} --remote-debugging-address=127.0.0.1` }, stdio: ["ignore", "pipe", "pipe"] });
let processError;
app.on("error", (error) => { processError = error; });
let processLog = "";
for (const stream of [app.stdout, app.stderr]) stream.on("data", (b) => { processLog = (processLog + b.toString()).slice(-100000); });
let socket;
let seq = 0;
const pending = new Map();
const exceptions = [];
async function until(fn, label, timeout = 20000) {
  const end = Date.now() + timeout;
  let last;
  while (Date.now() < end) {
    if (processError) throw processError;
    try { const value = await fn(); if (value) return value; } catch (error) { last = error; }
    await delay(150);
  }
  throw new Error(`Timeout: ${label}${last ? ` (${last.message})` : ""}`);
}
function cdp(method, params = {}) {
  const id = ++seq;
  return new Promise((ok, fail) => {
    const timer = setTimeout(() => { pending.delete(id); fail(new Error(`CDP timeout: ${method}`)); }, 15000);
    pending.set(id, { ok, fail, timer });
    socket.send(JSON.stringify({ id, method, params }));
  });
}
async function evaluate(expression) {
  const result = await cdp("Runtime.evaluate", { expression, awaitPromise: true, returnByValue: true });
  if (result.exceptionDetails) throw new Error(result.exceptionDetails.exception?.description || result.exceptionDetails.text);
  return result.result.value;
}
const invoke = (command, args = {}) => evaluate(`window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)},${JSON.stringify(args)})`);
const check = async (name, fn) => {
  const t = performance.now();
  await fn(); report.checks.push({ name, passed: true, elapsedMs: Math.round(performance.now() - t) }); save();
};
async function button(name, scope = "document") {
  return until(() => evaluate(`(() => { const root = ${scope}; const b = [...root.querySelectorAll('button')].find(e => (e.getAttribute('aria-label') || e.textContent).trim() === ${JSON.stringify(name)} && e.getClientRects().length); if (!b || b.disabled) return false; b.click(); return true; })()`), `button ${name}`);
}
async function fill(label, value) {
  await until(() => evaluate(`(() => { const e = document.querySelector('[aria-label=${JSON.stringify(label)}]'); return e && !e.disabled; })()`), label);
  await evaluate(`(() => { const e = document.querySelector('[aria-label=${JSON.stringify(label)}]'); const proto = e.tagName === 'TEXTAREA' ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype; Object.getOwnPropertyDescriptor(proto,'value').set.call(e,${JSON.stringify(value)}); e.dispatchEvent(new Event('input',{bubbles:true})); })()`);
}
async function navigate(group, page) {
  await button(group, `document.querySelector('nav[aria-label="主导航"]')`);
  await button(page, `document.querySelector('nav[aria-label="当前模块功能"]')`);
  await until(() => evaluate(`document.querySelector('h1')?.textContent === ${JSON.stringify(page)}`), page);
}
const title = `CI ${mode} 合成评审笔记`;
const body = "# 偏置复核\n\nIq=10 µA，VDD=1.8 V，TT，27°C。仅用于自动验证。";
try {
  const target = await until(async () => {
    const response = await fetch(`http://127.0.0.1:${port}/json/list`, { signal: AbortSignal.timeout(1500) });
    return (await response.json()).find((p) => p.type === "page" && /^https?:\/\/tauri\.localhost\//.test(p.url));
  }, "installed WebView2 page", 45000);
  socket = new WebSocket(target.webSocketDebuggerUrl);
  await new Promise((ok, fail) => { socket.addEventListener("open", ok, { once: true }); socket.addEventListener("error", fail, { once: true }); });
  socket.addEventListener("message", ({ data }) => {
    const msg = JSON.parse(data);
    if (msg.method === "Runtime.exceptionThrown") exceptions.push(msg.params.exceptionDetails);
    const wait = pending.get(msg.id);
    if (wait) { pending.delete(msg.id); clearTimeout(wait.timer); msg.error ? wait.fail(new Error(msg.error.message)) : wait.ok(msg.result); }
  });
  await cdp("Runtime.enable");
  await until(() => evaluate("!!window.__TAURI_INTERNALS__ && !!document.querySelector('h1')"), "application ready");
  report.startupMs = Math.round(performance.now() - started);
  await check("runtime identity and offline default", async () => {
    const status = await invoke("get_runtime_status");
    assert.equal(status.demoMode, mode === "demo");
    assert.equal(status.appVersion, version);
    assert.equal(await invoke("get_network_access"), false);
  });
  if (phase === "initial") {
    await check("all grouped destinations are reachable", async () => {
      for (const [group, pages] of [["今日", ["工作台", "日历与待办", "邮件助手"]], ["项目", ["项目管理"]], ["资料", ["Datasheet", "知识笔记"]], ["助手", ["智能对话", "工作模板"]], ["工具", ["格式转换", "科学计算器", "常用软件"]]]) {
        for (const page of pages) await navigate(group, page);
      }
    });
    await check("normal/demo seed isolation", async () => {
      const notes = await invoke("list_notes");
      if (mode === "normal") assert.equal(notes.length, 0);
      else assert(notes.some((n) => n.id === "note-demo-datasheet"));
      assert(!notes.some((n) => n.title.startsWith("CI ")));
    });
    await check("calculator operates in installed UI", async () => {
      await navigate("工具", "科学计算器");
      await fill("科学计算表达式", "1.8 * 2 + sqrt(4)");
      await button("计算");
      await until(() => evaluate("document.querySelector('.calculator-panel output')?.textContent === '5.6'"), "calculation result");
    });
    await check("four templates and advanced Skills entry", async () => {
      await navigate("助手", "工作模板");
      for (const name of ["设计评审", "Datasheet 核查", "故障排查", "项目周报"]) assert(await evaluate(`document.body.innerText.includes(${JSON.stringify(name)})`));
      await evaluate("document.querySelector('.advanced-options summary').click()");
      await until(() => evaluate("document.querySelector('.advanced-options').open && !!document.querySelector('.skills-workspace')"), "Skills workspace");
    });
    await check("note editor saves through real Rust IPC", async () => {
      await navigate("资料", "知识笔记");
      await button("新建笔记");
      await fill("笔记标题", title);
      await fill("笔记内容", body);
      await button("保存笔记");
      await until(async () => (await invoke("list_notes")).some((n) => n.title === title && n.body === body), "saved note");
    });
    await check("Chinese project indexing and model-number search", async () => {
      const project = await invoke("add_project", { path: join(resolve(fixtureDir), "中文 工程资料") });
      await invoke("index_project", { projectId: project.id });
      for (const query of ["偏置", "ZXW100"]) assert((await invoke("search_project", { projectId: project.id, query })).length > 0);
    });
    await check("native PDF conversion and source overwrite rejection", async () => {
      const source = join(resolve(fixtureDir), "中文 工程资料", "偏置 ZXW100.md");
      const before = hash(source);
      const document = await invoke("convert_document", { path: source });
      const destination = join(output, `${mode}-转换.pdf`);
      const args = { path: destination, format: "pdf", title: "合成偏置", body: document.markdown, formatted: null, sourcePaths: [source] };
      await invoke("export_converted_document", args);
      assert((await invoke("convert_document", { path: destination })).markdown.includes("ZXW100"));
      await assert.rejects(invoke("export_converted_document", args));
      await assert.rejects(invoke("export_converted_document", { ...args, path: source, format: "md" }));
      assert.equal(hash(source), before);
    });
    await check("backup restore preserves notes and returns offline", async () => {
      const path = join(output, `${mode}-synthetic.icwb`);
      await invoke("backup_local_data", { path, uiStateJson: "{}" });
      const before = hash(path);
      const temporary = await invoke("save_note", { input: { id: null, title: "CI temporary after backup", body: "discard on restore" } });
      await assert.rejects(invoke("restore_local_data", { path, confirmed: false }));
      await invoke("restore_local_data", { path, confirmed: true });
      const notes = await invoke("list_notes");
      assert(notes.some((n) => n.title === title)); assert(!notes.some((n) => n.id === temporary.id));
      assert.equal(await invoke("get_network_access"), false);
      assert.equal(hash(path), before);
    });
    await check("native calendar persistence", async () => {
      const now = Math.floor(Date.now() / 1000);
      const task = await invoke("save_calendar_task", { input: { id: null, title: `CI ${mode} review`, startAt: now, endAt: null, status: "todo", priority: "important", remindAt: null, projectId: null, notes: "synthetic" } });
      assert((await invoke("list_calendar_tasks", { rangeStart: now - 86400, rangeEnd: now + 86400 })).some((t) => t.id === task.id));
    });
    await check("draft autosaves before process exit", async () => {
      await navigate("资料", "知识笔记");
      await button("新建笔记");
      await fill("笔记标题", `${title} 草稿`);
      await fill("笔记内容", `${body}\n未提交草稿`);
      await until(async () => JSON.stringify(await invoke("load_workspace_state", { key: "note-draft" })).includes("未提交草稿"), "draft on disk");
    });
  } else {
    await check(`${phase} keeps saved notes, projects and draft`, async () => {
      assert((await invoke("list_notes")).some((n) => n.title === title && n.body === body));
      assert((await invoke("list_projects")).length > 0);
      assert(JSON.stringify(await invoke("load_workspace_state", { key: "note-draft" })).includes("未提交草稿"));
      await navigate("资料", "知识笔记");
      await until(() => evaluate(`document.querySelector('[aria-label="笔记标题"]')?.value === ${JSON.stringify(`${title} 草稿`)}`), "restored draft UI");
    });
  }
  await check("no uncaught renderer exceptions", async () => assert.deepEqual(exceptions, []));
  const screenshot = await cdp("Page.captureScreenshot", { format: "png" });
  writeFileSync(join(output, `${mode}-${phase}.png`), Buffer.from(screenshot.data, "base64"));
  report.status = "passed";
} catch (error) {
  report.status = "failed"; report.error = error.stack || String(error);
  if (socket?.readyState === WebSocket.OPEN) {
    try { const s = await cdp("Page.captureScreenshot", { format: "png" }); writeFileSync(join(output, `${mode}-${phase}-failed.png`), Buffer.from(s.data, "base64")); } catch {}
  }
  process.exitCode = 1;
} finally {
  if (socket?.readyState === WebSocket.OPEN) {
    try { await evaluate("void window.__TAURI_INTERNALS__.invoke('plugin:window|destroy',{label:'main'}); true"); } catch {}
    socket.close();
  }
  await delay(1000);
  if (app.exitCode === null) app.kill();
  for (const entry of pending.values()) clearTimeout(entry.timer);
  writeFileSync(join(output, `${mode}-${phase}-process.log`), processLog);
  report.completedAt = new Date().toISOString(); save();
  console.log(JSON.stringify(report));
}
