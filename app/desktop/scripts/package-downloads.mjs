import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { readFileSync, writeFileSync, existsSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../../..");
const version = JSON.parse(readFileSync(resolve(root, "app/desktop/package.json"))).version;
const release = resolve(root, `release/v${version}`);
if (process.platform !== "darwin" || process.arch !== "arm64") {
  throw new Error("本脚本仅打包已经构建的 macOS arm64 双应用");
}
const entries = [
  ["apps/normal/芯智工作台.app", `ic-workbench-v${version}-macos-arm64.zip`],
  ["apps/demo", `ic-workbench-demo-v${version}-macos-arm64.zip`],
];
const sums = [];
for (const [source, name] of entries) {
  const path = resolve(release, source);
  if (!existsSync(path)) throw new Error(`请先运行 pnpm build:clickable：缺少 ${source}`);
  execFileSync(process.execPath, [resolve(root, "app/desktop/scripts/verify-release-artifact.mjs"), path], { stdio: "inherit" });
  const archive = resolve(release, name);
  if (existsSync(archive)) throw new Error(`不覆盖已有下载包：${name}`);
  execFileSync("ditto", ["-c", "-k", "--keepParent", "--norsrc", "--noextattr", path, archive], {
    env: { ...process.env, COPYFILE_DISABLE: "1" }, stdio: "inherit",
  });
  const files = execFileSync("unzip", ["-Z1", archive], { encoding: "utf8" });
  if (/(^|\/)(?:\._|__MACOSX|\.DS_Store)/m.test(files)) throw new Error(`压缩包包含系统元数据：${name}`);
  sums.push(`${createHash("sha256").update(readFileSync(archive)).digest("hex")}  ${name}`);
}
writeFileSync(resolve(release, "SHA256SUMS.txt"), `${sums.join("\n")}\n`);
console.log("两套下载包与 SHA256SUMS.txt 已生成。");
