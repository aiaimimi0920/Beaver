import fs from "node:fs/promises";
import { createReadStream, createWriteStream } from "node:fs";
import { pipeline } from "node:stream/promises";
import { createHash, randomUUID } from "node:crypto";
import path from "node:path";
import type { Snapshot, Change } from "../shared/types";

const ignored = new Set([
  ".git",
  ".godot",
  ".beaver",
  ".beaver-context",
  "node_modules",
  "target",
  "release",
  "exports",
]);
export async function safePath(
  root: string,
  relative: string,
): Promise<string> {
  if (
    !relative ||
    path.isAbsolute(relative) ||
    relative.includes("\\") ||
    relative.split("/").some((p) => !p || p === "." || p === "..") ||
    relative.includes(":")
  )
    throw new Error("非法项目相对路径");
  const base = await fs.realpath(root);
  let current = base;
  for (const part of relative.split("/")) {
    current = path.join(current, part);
    try {
      if ((await fs.lstat(current)).isSymbolicLink())
        throw new Error("不允许通过符号链接访问项目文件");
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code !== "ENOENT") throw error;
    }
  }
  return current;
}
export async function listFiles(root: string, dir = ""): Promise<string[]> {
  const out: string[] = [];
  for (const item of await fs.readdir(path.join(root, dir), {
    withFileTypes: true,
  })) {
    if (ignored.has(item.name) || item.name.startsWith(".beaver-write-"))
      continue;
    if (item.isSymbolicLink())
      throw new Error(
        `项目含符号链接，无法安全创建任务副本：${dir}/${item.name}`,
      );
    const relative = dir ? `${dir}/${item.name}` : item.name;
    if (item.isDirectory()) out.push(...(await listFiles(root, relative)));
    else if (item.isFile()) out.push(relative);
  }
  return out.sort();
}
export async function fileHash(file: string): Promise<string | undefined> {
  const hash = createHash("sha256");
  try {
    for await (const chunk of createReadStream(file))
      hash.update(chunk as Buffer);
    return hash.digest("hex");
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === "ENOENT") return undefined;
    throw error;
  }
}
export class Files {
  constructor(readonly root: string) {}
  blob(hash: string): string {
    if (!/^[a-f0-9]{64}$/.test(hash)) throw new Error("非法内容摘要");
    return path.join(this.root, "blobs", hash);
  }
  async capture(project: string): Promise<Snapshot> {
    await fs.mkdir(path.join(this.root, "blobs"), { recursive: true });
    const snapshot: Snapshot = {};
    for (const relative of await listFiles(project)) {
      const source = await safePath(project, relative);
      const temporary = path.join(this.root, "blobs", `.tmp-${randomUUID()}`);
      try {
        await pipeline(
          createReadStream(source),
          createWriteStream(temporary, { flags: "wx" }),
        );
        const hash = await fileHash(temporary);
        if (!hash) throw new Error("快照文件读取失败");
        if ((await fileHash(source)) !== hash)
          throw new Error(`文件在快照时发生变化，请重试：${relative}`);
        try {
          await fs.copyFile(temporary, this.blob(hash), 1);
        } catch (e) {
          if ((e as NodeJS.ErrnoException).code !== "EEXIST") throw e;
        }
        snapshot[relative] = hash;
      } finally {
        await fs.rm(temporary, { force: true });
      }
    }
    return snapshot;
  }
  async restoreCopy(snapshot: Snapshot, target: string): Promise<void> {
    await fs.mkdir(target, { recursive: true });
    for (const [relative, hash] of Object.entries(snapshot)) {
      const file = await safePath(target, relative);
      await fs.mkdir(path.dirname(file), { recursive: true });
      await fs.copyFile(this.blob(hash), file);
    }
  }
  changes(before: Snapshot, after: Snapshot): Change[] {
    return [...new Set([...Object.keys(before), ...Object.keys(after)])]
      .sort()
      .filter((p) => before[p] !== after[p])
      .map((p) => ({ path: p, before: before[p], after: after[p] }));
  }
  async conflicts(project: string, changes: Change[]): Promise<string[]> {
    const conflicts: string[] = [];
    for (const c of changes)
      if ((await fileHash(await safePath(project, c.path))) !== c.before)
        conflicts.push(c.path);
    return conflicts;
  }
  async apply(project: string, changes: Change[]): Promise<void> {
    const conflicts = await this.conflicts(project, changes);
    if (conflicts.length)
      throw new Error(`文件冲突，未覆盖：${conflicts.join(", ")}`);
    const applied: Change[] = [];
    try {
      for (const change of changes) {
        if (
          (await fileHash(await safePath(project, change.path))) !==
          change.before
        )
          throw new Error(`写入前文件发生变化：${change.path}`);
        await this.write(project, change.path, change.after);
        applied.push(change);
      }
    } catch (error) {
      for (const change of applied.reverse())
        if (
          (await fileHash(await safePath(project, change.path))) ===
          change.after
        )
          await this.write(project, change.path, change.before);
      throw error;
    }
  }
  private async write(
    project: string,
    relative: string,
    hash?: string,
  ): Promise<void> {
    const file = await safePath(project, relative);
    if (!hash) {
      await fs.rm(file, { force: true });
      return;
    }
    await fs.mkdir(path.dirname(file), { recursive: true });
    const temp = path.join(path.dirname(file), `.beaver-write-${randomUUID()}`);
    try {
      await fs.copyFile(this.blob(hash), temp);
      await fs.rename(temp, file);
    } finally {
      await fs.rm(temp, { force: true });
    }
  }
}
export class ProjectLocks {
  private tails = new Map<string, Promise<unknown>>();
  run<T>(id: string, work: () => Promise<T>): Promise<T> {
    const next = (this.tails.get(id) ?? Promise.resolve())
      .catch(() => undefined)
      .then(work);
    this.tails.set(id, next);
    void next
      .finally(() => {
        if (this.tails.get(id) === next) this.tails.delete(id);
      })
      .catch(() => undefined);
    return next;
  }
}
