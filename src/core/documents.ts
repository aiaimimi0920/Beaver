import fs from "node:fs/promises";
import { createHash } from "node:crypto";
import { safePath } from "./files";

export function documentPath(relative: string): string {
  if (
    !relative.endsWith(".md") ||
    relative
      .split("/")
      .some(
        (part) =>
          part.startsWith(".") ||
          /^(node_modules|target|release|exports)$/i.test(part),
      )
  )
    throw new Error("资料仅支持项目内的 Markdown 文件");
  return relative;
}

export async function readDocument(root: string, relative: string) {
  const file = await safePath(root, documentPath(relative));
  const handle = await fs.open(file, "r");
  try {
    if ((await handle.stat()).size > 512000) throw new Error("资料超过 500 KB");
    const bytes = await handle.readFile();
    if (bytes.length > 512000) throw new Error("资料超过 500 KB");
    return {
      path: relative,
      text: new TextDecoder("utf-8", { fatal: true }).decode(bytes),
      revision: createHash("sha256").update(bytes).digest("hex"),
    };
  } finally {
    await handle.close();
  }
}
