import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, lstatSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { delimiter, dirname, join, resolve } from "node:path";
import { release, machine, totalmem, cpus } from "node:os";
import { fileURLToPath } from "node:url";
import { classifyWindows } from "./windows-environment.mjs";

// Run inside a disposable Windows VM with synthetic fixtures. This is the
// automated portion of acceptance; it does not certify installer or GUI flows.
if (process.platform !== "win32") throw new Error("Native Windows is required for this acceptance run.");
const appRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const projectRoot = resolve(appRoot, "../..");
const options = { arch: "x64", backup: "", tls: "", scope: "win10" };
const args = process.argv.slice(2).filter((arg) => arg !== "--");
for (let i = 0; i < args.length; i += 2) {
  const key = args[i]?.replace(/^--/, "");
  if (!Object.hasOwn(options, key) || !args[i + 1]) throw new Error("Use --scope windows|win10 --arch x64|arm64 --backup <synthetic-schema9.icwb> --tls <synthetic-TLS-directory>.");
  options[key] = args[i + 1];
}
if (!["x64", "arm64"].includes(options.arch)) throw new Error("Invalid architecture.");
if (!options.backup || !options.tls) throw new Error("Explicit synthetic backup and TLS fixtures are required.");
const backup = resolve(options.backup);
const tls = resolve(options.tls);
for (const path of [backup, join(tls, "identity.p12"), join(tls, "cert.pem")]) {
  const stat = lstatSync(path);
  if (!stat.isFile() || stat.isSymbolicLink()) throw new Error("Fixture must be a regular file: " + path);
}
const digest = (path) => createHash("sha256").update(readFileSync(path)).digest("hex");
const originalBackupHash = digest(backup);
const target = options.arch === "x64" ? "x86_64-pc-windows-msvc" : "aarch64-pc-windows-msvc";
const stamp = new Date().toISOString().replaceAll(/[:.]/g, "-");
const output = join(projectRoot, "artifacts/windows-validation/runs", `${stamp}-${options.arch}`);
if (existsSync(output)) throw new Error("Refusing to overwrite a previous run.");
mkdirSync(output, { recursive: true });
const manifest = {
  status: "running", platform: process.platform, nodeArchitecture: process.arch, target,
  kernelVersion: release(), machine: machine(), logicalProcessors: cpus().length, memoryBytes: totalmem(),
  nodeVersion: process.version, startedAt: new Date().toISOString(),
  originalBackupSha256: originalBackupHash, checks: [],
  remaining: ["win10-workstation", "standard-user-UAC", "installer-update-uninstall", "native-GUI", "real-local-model", "performance", "Intel-AMD-hardware"],
};
const save = () => writeFileSync(join(output, "manifest.json"), JSON.stringify(manifest, null, 2) + "\n");
const env = {
  ...process.env,
  PATH: `${dirname(process.execPath)}${delimiter}${process.env.PATH ?? ""}`,
  CARGO_TARGET_DIR: process.env.CARGO_TARGET_DIR || join(projectRoot, "artifacts/windows-validation/test-build", options.arch),
  CARGO_NET_OFFLINE: "true",
  ICWB_QA_BACKUP: backup, ICWB_QA_TLS: tls,
  // The frontend creates fresh PDF/Word/PowerPoint/Markdown conversion inputs
  // in this run's directory; the native backend must export and read them back.
  ICWB_CONVERSION_FIXTURES: join(output, "document-conversion-fixtures"),
};
const run = (name, executable, parameters) => {
  const started = performance.now();
  const result = spawnSync(executable, parameters, { cwd: appRoot, env, encoding: "utf8", maxBuffer: 32 * 1024 * 1024, timeout: 45 * 60 * 1000 });
  writeFileSync(join(output, `${name}.log`), (result.stdout ?? "") + (result.stderr ?? ""));
  const check = { name, exitCode: result.status, elapsedMs: Math.round(performance.now() - started) };
  if (result.error) check.error = result.error.message;
  manifest.checks.push(check);
  save();
  if (result.error || result.status !== 0) {
    console.error((result.stdout ?? "").slice(-12000));
    console.error((result.stderr ?? "").slice(-4000));
    throw new Error(`${name} failed; inspect its log.`);
  }
  console.log(`${name}: passed (${check.elapsedMs} ms)`);
};
save();
try {
  // Windows 11 also reports NT 10.0. Record the native build and workstation
  // edition so a run on another OS cannot be presented as Windows 10 evidence.
  run("windows-environment", "powershell.exe", ["-NoProfile", "-NonInteractive", "-Command",
    "$ErrorActionPreference='Stop'; $o=Get-CimInstance Win32_OperatingSystem; " +
    "$o | Select-Object Caption,Version,BuildNumber,OSArchitecture,ProductType | ConvertTo-Json -Compress"]);
  const system = JSON.parse(readFileSync(join(output, "windows-environment.log"), "utf8").replace(/^\uFEFF/, "").trim());
  manifest.operatingSystem = system;
  save();
  manifest.environment = classifyWindows(system, options.scope);
  if (manifest.environment.win10Evidence) manifest.remaining = manifest.remaining.filter((item) => item !== "win10-workstation");
  save();
  if (process.arch !== options.arch) throw new Error("Use a Node runtime matching the requested Windows architecture.");
  run("frontend", process.execPath, ["node_modules/vitest/vitest.mjs", "run"]);
  run("demo-model-protocol", process.execPath, ["--test", "../../demo/mock-model-server.test.mjs"]);
  run("rust-version", "rustc.exe", ["-vV"]);
  const cargoArgs = ["test", "--locked", "--manifest-path", "src-tauri/Cargo.toml", "--target", target];
  run("rust-regression", "cargo.exe", cargoArgs);
  for (const name of [
    "actual_schema9_upgrade_preserves_every_existing_table_and_original_file",
    "restores_exported_synthetic_backup_in_isolation",
    "system_credentials_lifecycle_and_preview_isolation",
    "real_tls_imap_readonly_cache_and_smtp_delivery",
    "production_imap_connector_rejects_untrusted_certificate",
    "generated_frontend_formats_pass_backend_export_and_read_back",
  ]) run(name, "cargo.exe", [...cargoArgs, name, "--", "--ignored", "--nocapture"]);
  // Cargo exits successfully if a misspelled filter matches zero tests.
  for (const check of manifest.checks.filter((check) => !["windows-environment", "rust-version", "frontend", "demo-model-protocol", "rust-regression"].includes(check.name))) {
    if (!/test result: ok\. 1 passed; 0 failed;/.test(readFileSync(join(output, `${check.name}.log`), "utf8"))) {
      throw new Error(`${check.name} did not actually run exactly one test.`);
    }
  }
  if (digest(backup) !== originalBackupHash) throw new Error("Synthetic source backup changed.");
  manifest.status = "automated-checks-passed-native-acceptance-incomplete";
} catch (error) {
  manifest.status = "failed";
  manifest.error = error.message;
  throw error;
} finally {
  manifest.backupUnchanged = digest(backup) === originalBackupHash;
  manifest.completedAt = new Date().toISOString();
  save();
  console.log(`Evidence: ${output}`);
}
