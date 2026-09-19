import { spawnSync } from "node:child_process";
import { cpSync, existsSync, lstatSync, mkdirSync, readFileSync, readdirSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

if (process.platform !== "darwin") {
  console.error("当前脚本生成可双击的 macOS .app；其他平台需在对应目标系统构建。 ");
  process.exit(2);
}

const appRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const workspaceRoot = resolve(appRoot, "../..");
const version = JSON.parse(readFileSync(resolve(appRoot, "package.json"), "utf8")).version;
if (!/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/.test(version)) {
  console.error("应用版本格式无效，无法确定安全的输出目录。");
  process.exit(2);
}
const tauriVersion = JSON.parse(readFileSync(resolve(appRoot, "src-tauri/tauri.conf.json"), "utf8")).version;
if (version !== tauriVersion) {
  console.error("package.json 与 Tauri 配置版本不一致，请先对齐版本。");
  process.exit(2);
}
const outputRoot = resolve(workspaceRoot, `release/v${version}/apps`);
if (existsSync(outputRoot)) {
  console.error(`拒绝覆盖已有应用产物：${outputRoot}。请为新迭代设置独立版本；日常检查使用调试构建。`);
  process.exit(2);
}
const buildRoot = resolve(tmpdir(), "ic-workbench-clickable-build");

const variants = [
  { mode: "normal", name: "芯智工作台", config: undefined },
  { mode: "demo", name: "芯智工作台 Demo", config: "src-tauri/tauri.demo.conf.json" },
];

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    cwd: appRoot,
    stdio: "inherit",
    ...options,
  });
  if (result.error || result.status !== 0) {
    console.error(`${command} 执行失败${result.error ? `：${result.error.message}` : ""}`);
    process.exit(result.status || 1);
  }
}

function removeAppleDouble(path) {
  if (!lstatSync(path).isDirectory()) return;
  for (const entry of readdirSync(path)) {
    const child = resolve(path, entry);
    if (entry.startsWith("._")) {
      rmSync(child, { force: true, recursive: true });
    } else if (lstatSync(child).isDirectory()) {
      removeAppleDouble(child);
    }
  }
}

mkdirSync(outputRoot, { recursive: true });

for (const variant of variants) {
  const targetDir = resolve(buildRoot, variant.mode);
  const args = ["scripts/build-release.mjs", "--bundles", "app"];
  if (variant.config) args.push("--config", variant.config);
  run(process.execPath, args, {
    env: {
      ...process.env,
      CARGO_TARGET_DIR: targetDir,
      ICWB_APP_MODE: variant.mode,
    },
  });

  const source = resolve(targetDir, "release/bundle/macos", `${variant.name}.app`);
  const destinationDirectory = resolve(outputRoot, variant.mode);
  const destination = resolve(destinationDirectory, `${variant.name}.app`);
  removeAppleDouble(source);
  run("codesign", ["--force", "--deep", "--sign", "-", source]);
  run("codesign", ["--verify", "--deep", "--strict", source]);
  rmSync(destinationDirectory, { force: true, recursive: true });
  mkdirSync(destinationDirectory, { recursive: true });
  cpSync(source, destination, { recursive: true });
  if (variant.mode === "demo") {
    const demoMaterials = resolve(destinationDirectory, "演示资料");
    mkdirSync(demoMaterials, { recursive: true });
    for (const entry of ["README.md", "ACCEPTANCE_CHECKLIST.md", "sample-project", "sample-skill", "samples"]) {
      cpSync(resolve(workspaceRoot, "demo", entry), resolve(demoMaterials, entry), { recursive: true });
    }
  }
  removeAppleDouble(destinationDirectory);
  run("codesign", ["--verify", "--deep", "--strict", destination]);
  run(process.execPath, ["scripts/verify-release-artifact.mjs", destinationDirectory]);
}

removeAppleDouble(outputRoot);
run(process.execPath, ["scripts/verify-release-artifact.mjs", outputRoot]);
console.log(`可双击应用已生成：${outputRoot}`);
