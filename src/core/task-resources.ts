import fs from "node:fs/promises";
import { fileHash, listFiles, safePath } from "./files";
import type { Task } from "../shared/types";
import { decodeResourceText } from "./text-encoding";

export async function taskResources(task: Task) {
  if (task.workspacePrepared === false) return [];
  const paths = new Map(task.references.map((ref) => [ref.path, "参考"]));
  for (const change of task.changes) paths.set(change.path, "修改");
  const current = new Set(await listFiles(task.workspace));
  for (const relative of Object.keys(task.baseline))
    if (!current.has(relative)) paths.set(relative, "删除");
  for (const relative of current) {
    if (paths.has(relative)) continue;
    if (
      (await fileHash(await safePath(task.workspace, relative))) !==
      task.baseline[relative]
    )
      paths.set(relative, "修改");
  }
  return Promise.all(
    [...paths].map(async ([relative, origin]) => {
      const file = await safePath(task.workspace, relative);
      const stat = await fs.stat(file).catch((error: unknown) => {
        if ((error as NodeJS.ErrnoException).code === "ENOENT")
          return undefined;
        throw error;
      });
      return { path: relative, origin, exists: !!stat?.isFile() };
    }),
  );
}

async function readBytes(task: Task, relative: string) {
  const file = await safePath(task.workspace, relative);
  const handle = await fs.open(file, "r");
  try {
    if ((await handle.stat()).size > 512000)
      throw new Error("文件超过 500 KB，请在工作副本中查看");
    const bytes = await handle.readFile();
    if (bytes.length > 512000) throw new Error("文件超过 500 KB");
    return bytes;
  } finally {
    await handle.close();
  }
}
export async function taskResourceText(task: Task, relative: string) {
  return decodeResourceText(await readBytes(task, relative));
}
export async function taskResourceBytes(task: Task, relative: string) {
  const bytes = await readBytes(task, relative);
  return {
    path: relative,
    bytes: bytes.length,
    base64: bytes.toString("base64"),
  };
}
