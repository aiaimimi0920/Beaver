import fs from "node:fs/promises";
import { fileHash, listFiles, safePath } from "./files";
import { decodeResourceText } from "./text-encoding";
import type { Snapshot } from "../shared/types";
export async function normalizeSources(root: string, baseline: Snapshot) {
  const normalized: string[] = [];
  for (const relative of await listFiles(root)) {
    if (!/\.(gd|tscn|tres|godot|cfg|md|json|toml|ya?ml)$/i.test(relative))
      continue;
    const file = await safePath(root, relative);
    if ((await fileHash(file)) === baseline[relative]) continue;
    const bytes = await fs.readFile(file);
    let text: string;
    try {
      text = decodeResourceText(bytes);
    } catch {
      throw new Error(`源文件编码无效：${relative}`);
    }
    if (!bytes.equals(Buffer.from(text, "utf8"))) {
      await fs.writeFile(file, text, "utf8");
      normalized.push(relative);
    }
  }
  return normalized;
}
