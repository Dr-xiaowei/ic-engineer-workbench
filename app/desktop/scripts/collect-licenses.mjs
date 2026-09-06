import { readdirSync, readFileSync, statSync, writeFileSync, mkdirSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const records = new Map();
function collect(name, version, license, directory) {
  const key = `${name}@${version}`;
  if (records.has(key)) return;
  const texts = [];
  for (const file of readdirSync(directory)) {
    if (!/^(licen[sc]e|copying|notice|copyright)([._-]|$)/i.test(file)) continue;
    const path = join(directory,file);
    if (statSync(path).isFile() && statSync(path).size < 300_000) texts.push(readFileSync(path,"utf8"));
  }
  records.set(key, { name, version, license: typeof license === "string" ? license : JSON.stringify(license ?? "SEE PACKAGE"), texts });
}
const metadata = spawnSync("cargo", ["metadata","--locked","--offline","--format-version","1","--filter-platform",process.arch === "arm64" ? "aarch64-apple-darwin" : "x86_64-apple-darwin"], { cwd: join(root,"src-tauri"), encoding:"utf8", maxBuffer:20*1024*1024 });
if (metadata.status !== 0) throw Error(metadata.stderr);
const cargo = JSON.parse(metadata.stdout);
const used = new Set(cargo.resolve.nodes.map(node=>node.id));
for (const pkg of cargo.packages) if (pkg.source && used.has(pkg.id)) collect(pkg.name,pkg.version,pkg.license,dirname(pkg.manifest_path));
const store = join(root,"node_modules/.pnpm");
for (const entry of readdirSync(store)) {
  const directory = join(store,entry,"node_modules");
  try {
    for (const name of readdirSync(directory)) {
      if (name.startsWith(".")) continue;
      const children = name.startsWith("@") ? readdirSync(join(directory,name)).map(child=>join(name,child)) : [name];
      for (const child of children) {
        try { const pkg = JSON.parse(readFileSync(join(directory,child,"package.json"),"utf8")); collect(pkg.name,pkg.version,pkg.license,join(directory,child)); } catch { /* not a package */ }
      }
    }
  } catch { /* pnpm metadata files */ }
}
const sorted = [...records.values()].sort((a,b)=>a.name.localeCompare(b.name)||a.version.localeCompare(b.version));
const target = join(root,"src-tauri/assets/legal");mkdirSync(target,{recursive:true});
writeFileSync(join(target,"THIRD_PARTY_LICENSES.txt"),"Third-party dependency inventory and supplied license notices\nVersions correspond to the v1.0.0 lockfiles. Includes build/target dependencies as a conservative superset.\n\n"+sorted.map(p=>`${p.name} ${p.version}\nLicense: ${p.license}\n${p.texts.join("\n")}\n`).join("\n--------------------\n\n"));
console.log(`Collected ${sorted.length} package notices`);
