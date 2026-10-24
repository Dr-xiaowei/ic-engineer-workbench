import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { cpSync, existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const appRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const projectRoot = resolve(appRoot, "../..");
const args = process.argv.slice(2).filter((value) => value !== "--");
const options = { arch: "x64", version: "1.1.0-rc.1", webview: "offline" };
for (let i = 0; i < args.length; i += 2) {
  const key = args[i]?.replace(/^--/, "");
  if (!Object.hasOwn(options, key) || !args[i + 1]) throw new Error("参数仅支持 --arch x64|arm64 --version x.y.z[-rc.n] --webview offline|installed");
  options[key] = args[i + 1];
}
if (!["x64", "arm64"].includes(options.arch) || !["offline", "installed"].includes(options.webview)
  || !/^\d+\.\d+\.\d+(?:-rc\.\d+)?$/.test(options.version)) throw new Error("Windows 构建参数无效。");
if (process.platform !== "win32") throw new Error("请在隔离 Windows MSVC 开发环境执行；Mac 构建不记作 Windows 验收。");

const target = options.arch === "x64" ? "x86_64-pc-windows-msvc" : "aarch64-pc-windows-msvc";
const output = join(projectRoot, "artifacts/windows-preview", options.version, `${options.arch}-${options.webview}`);
if (existsSync(output)) throw new Error(`拒绝覆盖已有候选或未完成记录：${output}`);
mkdirSync(output, { recursive: true });
const manifest = { version: options.version, architecture: options.arch, target, webview: options.webview,
  channel: "preview", status: "building", platform: process.platform, files: [] };
const save = () => writeFileSync(join(output, "manifest.json"), JSON.stringify(manifest, null, 2) + "\n");
save();
const run = (script, parameters, env = process.env) => {
  const result = spawnSync(process.execPath, [script, ...parameters], { cwd: appRoot, env, stdio: "inherit" });
  if (result.error || result.status !== 0) throw new Error(result.error?.message ?? `${script} 退出码 ${result.status}`);
};

try {
  for (const mode of ["normal", "demo"]) {
    const identifier = `com.icengineer.workbench.windowspreview${mode === "demo" ? ".demo" : ""}`;
    const name = `芯智工作台 Windows 预览${mode === "demo" ? " Demo" : ""}`;
    const base = JSON.parse(readFileSync(join(appRoot, "src-tauri/tauri.conf.json"), "utf8"));
    const override = {
      version: options.version, identifier, productName: name,
      app: { windows: base.app.windows.map((window) => ({ ...window, title: name })) },
      bundle: { windows: { webviewInstallMode: options.webview === "offline"
        ? { type: "offlineInstaller", silent: true } : { type: "skip" } } },
    };
    // CI reuses compiled dependencies across the two app identifiers. Tauri's
    // config/environment inputs still rebuild the application for each mode.
    const targetDir = process.env.CARGO_TARGET_DIR || resolve(projectRoot, "artifacts/windows-validation/build", options.arch, mode);
    const parameters = ["--target", target, "--config", "src-tauri/tauri.windows.conf.json"];
    if (mode === "demo") parameters.push("--config", "src-tauri/tauri.demo.conf.json");
    parameters.push("--config", JSON.stringify(override));
    const env = { ...process.env, CARGO_TARGET_DIR: targetDir, ICWB_APP_MODE: mode };
    run(join(appRoot, "scripts/build-release.mjs"), parameters, env);
    const binary = join(targetDir, target, "release/ic-engineer-workbench.exe");
    // Inspect the uncompressed program. Scanning only a compressed installer misses embedded paths.
    run(join(appRoot, "scripts/verify-release-artifact.mjs"), [binary]);
    const bundles = join(targetDir, target, "release/bundle/nsis");
    const installers = readdirSync(bundles).filter((file) => file.endsWith(".exe") && file.includes(options.version));
    if (installers.length !== 1) throw new Error(`预期一个当前版本安装包，实际 ${installers.length}`);
    const directory = join(output, mode);
    mkdirSync(directory);
    const destination = join(directory, `${name}_${options.version}_${options.arch}_setup.exe`);
    cpSync(join(bundles, installers[0]), destination, { errorOnExist: true, force: false });
    const content = readFileSync(destination);
    manifest.files.push({ path: `${mode}/${destination.split(/[\\/]/).at(-1)}`, bytes: content.length,
      sha256: createHash("sha256").update(content).digest("hex"), identifier });
    save();
  }
  manifest.status = "built-not-accepted";
  save();
  writeFileSync(join(output, "SHA256SUMS.txt"), manifest.files.map((file) => `${file.sha256}  ${file.path}`).join("\n") + "\n");
  console.log(`Windows 双候选已构建，安装/升级/原生功能验收尚需单独完成：${output}`);
} catch (error) {
  manifest.status = "failed";
  save();
  throw error;
}
