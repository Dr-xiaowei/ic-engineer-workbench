import { spawnSync } from "node:child_process";
import { realpathSync } from "node:fs";
import { homedir } from "node:os";
import { delimiter, dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const appRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const forwardedArguments = process.argv.slice(2).filter((argument) => argument !== "--");

if (forwardedArguments.includes("--debug")) {
  console.error("bundle:release 只允许优化构建；调试包不能作为交付候选。");
  process.exit(2);
}

const prefixes = [
  [resolve(appRoot), "/build/workspace/app/desktop"],
  [resolve(appRoot, "../.."), "/build/workspace"],
  [process.env.CARGO_TARGET_DIR, "/build/target"],
  [process.env.CARGO_HOME, "/build/cargo-home"],
  [homedir(), "/build/home"],
]
  .filter(([source]) => typeof source === "string" && source.length > 1)
  .sort(([left], [right]) => right.length - left.length);

const mappedPrefixes = [...new Map(prefixes.flatMap(([source, replacement]) => {
  const variants = new Set([source]);
  try { variants.add(realpathSync.native(source)); } catch { /* Optional not-yet-created target directory. */ }
  if (process.platform === "win32") for (const path of [...variants]) {
    if (/^[a-z]:\\/i.test(path)) variants.add(`\\\\?\\${path}`);
  }
  return [...variants].flatMap(path => [[path, replacement], [path.replaceAll("\\", "/"), replacement]]);
})).entries()];
const remapFlags = mappedPrefixes.map(
  ([source, replacement]) => `--remap-path-prefix=${source}=${replacement}`,
);
const inheritedFlags = (process.env.CARGO_ENCODED_RUSTFLAGS ?? "")
  .split("\u001f")
  .filter(Boolean);
const environment = {
  ...process.env,
  CARGO_ENCODED_RUSTFLAGS: [...inheritedFlags, ...remapFlags].join("\u001f"),
  PATH: `${dirname(process.execPath)}${delimiter}${process.env.PATH ?? ""}`,
};
delete environment.RUSTFLAGS;

if (process.platform === "win32") {
  // Rust remapping does not rewrite __FILE__ in native C dependencies (AWS-LC,
  // SQLite). clang-cl preserves the MSVC ABI and supports file-prefix mapping.
  const compiler = spawnSync("clang-cl", ["--version"], { encoding: "utf8" });
  if (compiler.error || compiler.status !== 0) throw new Error("Windows 交付构建需要已安装并加入 PATH 的 LLVM clang-cl，以移除 C/C++ 构建路径。");
  const quoteFlag = value => `"${value.replaceAll("\\", "\\\\").replaceAll('"', '\\"')}"`;
  const cFlags = mappedPrefixes.map(([source, replacement]) => quoteFlag(`/clang:-ffile-prefix-map=${source}=${replacement}`)).join(" ");
  environment.CC = "clang-cl";
  environment.CXX = "clang-cl";
  environment.CC_SHELL_ESCAPED_FLAGS = "1";
  environment.CFLAGS = [environment.CFLAGS, cFlags].filter(Boolean).join(" ");
  environment.CXXFLAGS = [environment.CXXFLAGS, cFlags].filter(Boolean).join(" ");
  environment.CARGO_PROFILE_RELEASE_STRIP = "debuginfo";
}

const executable = resolve(appRoot, "node_modules/@tauri-apps/cli/tauri.js");
const result = spawnSync(process.execPath, [executable, "build", ...forwardedArguments], {
  cwd: appRoot,
  env: environment,
  stdio: "inherit",
});

if (result.error) {
  console.error(`无法启动 Tauri 构建：${result.error.message}`);
  process.exit(1);
}
process.exit(result.status ?? 1);
