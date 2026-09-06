import { lstatSync, readFileSync, readdirSync } from "node:fs";
import { homedir } from "node:os";
import { basename, dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const appRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const requestedPath = process.argv.slice(2).find((argument) => argument !== "--");

if (!requestedPath) {
  console.error("用法：pnpm verify:artifact -- <应用包或二进制路径>");
  process.exit(2);
}

const artifactPath = resolve(requestedPath);
const forbiddenNames = [
  /^\._/,
  /^\.env(?:\.|$)/i,
  /\.(?:key|pem|p12|pfx)$/i,
];
const forbiddenContent = [...new Set([homedir(), appRoot])]
  .filter((value) => value.length > 1)
  .map((value) => Buffer.from(value));
const failures = [];
let fileCount = 0;

function inspect(path) {
  const metadata = lstatSync(path);
  if (metadata.isSymbolicLink()) {
    failures.push(`${path}: 不允许未经检查的符号链接`);
    return;
  }
  if (metadata.isDirectory()) {
    for (const entry of readdirSync(path)) inspect(resolve(path, entry));
    return;
  }
  if (!metadata.isFile()) return;

  fileCount += 1;
  const name = basename(path);
  if (forbiddenNames.some((pattern) => pattern.test(name))) {
    failures.push(`${path}: 禁止的文件名`);
  }

  const content = readFileSync(path);
  for (const marker of forbiddenContent) {
    if (content.includes(marker)) {
      failures.push(`${path}: 含开发用户名或工作区绝对路径`);
      break;
    }
  }
}

try {
  inspect(artifactPath);
} catch (error) {
  console.error(`无法检查产物：${error instanceof Error ? error.message : String(error)}`);
  process.exit(1);
}

if (failures.length > 0) {
  console.error("交付产物检查失败：");
  for (const failure of failures) console.error(`- ${failure}`);
  process.exit(1);
}

console.log(`交付产物检查通过：${artifactPath}（${fileCount} 个文件）`);
