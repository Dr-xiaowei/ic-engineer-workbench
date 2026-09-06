import { cpSync, mkdirSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const target = resolve(root, "public/pdfjs");
mkdirSync(target, { recursive: true });
for (const entry of ["standard_fonts", "cmaps", "LICENSE"]) {
  cpSync(resolve(root, "node_modules/pdfjs-dist", entry), resolve(target, entry), { recursive: true });
}
