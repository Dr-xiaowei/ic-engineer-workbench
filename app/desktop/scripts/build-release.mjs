import { spawnSync } from "node:child_process";
import { homedir } from "node:os";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const appRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const forwardedArguments = process.argv.slice(2).filter((argument) => argument !== "--");

if (forwardedArguments.includes("--debug")) {
  console.error("bundle:release 只允许优化构建；调试包不能作为交付候选。");
  process.exit(2);
}

const prefixes = [
  [resolve(appRoot), "/build/workspace/app/desktop"],
  [process.env.CARGO_TARGET_DIR, "/build/target"],
  [process.env.CARGO_HOME, "/build/cargo-home"],
  [homedir(), "/build/home"],
]
  .filter(([source]) => typeof source === "string" && source.length > 1)
  .sort(([left], [right]) => right.length - left.length);

const remapFlags = [...new Map(prefixes).entries()].map(
  ([source, replacement]) => `--remap-path-prefix=${source}=${replacement}`,
);
const inheritedFlags = (process.env.CARGO_ENCODED_RUSTFLAGS ?? "")
  .split("\u001f")
  .filter(Boolean);
const environment = {
  ...process.env,
  CARGO_ENCODED_RUSTFLAGS: [...inheritedFlags, ...remapFlags].join("\u001f"),
  PATH: `${dirname(process.execPath)}:${process.env.PATH ?? ""}`,
};
delete environment.RUSTFLAGS;

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
