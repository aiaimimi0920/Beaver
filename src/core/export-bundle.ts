import fs from "node:fs/promises";
import path from "node:path";
import { z } from "zod";
import { fileHash, safePath } from "./files";

const manifestName = "export-manifest.json";
const recordSchema = z.object({
  path: z.string().min(1),
  bytes: z.number().int().nonnegative(),
  sha256: z.string().regex(/^[a-f0-9]{64}$/),
});
const manifestSchema = z.object({
  version: z.literal(2),
  entry: z.string().min(1),
  platform: z.enum(["Windows Desktop", "Linux", "macOS"]),
  files: z.array(recordSchema).min(1).max(100000),
});

async function bundleFiles(root: string, directory = ""): Promise<string[]> {
  const files: string[] = [];
  for (const item of await fs.readdir(path.join(root, directory), {
    withFileTypes: true,
  })) {
    const relative = directory ? `${directory}/${item.name}` : item.name;
    if (item.isSymbolicLink())
      throw new Error(`导出包不允许符号链接：${relative}`);
    if (item.isDirectory()) files.push(...(await bundleFiles(root, relative)));
    else if (
      item.isFile() &&
      relative !== manifestName &&
      relative !== "export.log"
    )
      files.push(relative);
    else if (!item.isFile())
      throw new Error(`导出包包含不支持的文件类型：${relative}`);
  }
  return files.sort();
}

export async function captureExportBundle(root: string) {
  const records: z.infer<typeof recordSchema>[] = [];
  for (const relative of await bundleFiles(root)) {
    const file = await safePath(root, relative);
    const stat = await fs.stat(file);
    const sha256 = await fileHash(file);
    if (!sha256) throw new Error(`导出文件读取期间消失：${relative}`);
    records.push({ path: relative, bytes: stat.size, sha256 });
  }
  return records;
}

export async function verifyExportBundle(directory: string) {
  const root = await fs.realpath(directory);
  const manifestFile = await safePath(root, manifestName);
  if ((await fs.stat(manifestFile)).size > 32 * 1024 * 1024)
    throw new Error("导出清单过大");
  const parsed = manifestSchema.safeParse(
    JSON.parse(await fs.readFile(manifestFile, "utf8")),
  );
  if (!parsed.success)
    throw new Error(
      "不是完整的 v2 导出清单；旧版导出请重新生成，不能仅凭入口文件判断完整性",
    );
  const manifest = parsed.data;
  const declared = new Set<string>();
  for (const record of manifest.files) {
    await safePath(root, record.path);
    const key =
      process.platform === "win32" ? record.path.toLowerCase() : record.path;
    if (declared.has(key)) throw new Error("导出清单包含重复路径");
    declared.add(key);
  }
  if (!manifest.files.some((file) => file.path === manifest.entry))
    throw new Error("入口程序未包含在完整性清单中");
  const actual = await captureExportBundle(root);
  if (actual.length !== manifest.files.length)
    throw new Error("导出文件数量发生变化，请重新生成或恢复完整目录");
  const expected = new Map(manifest.files.map((file) => [file.path, file]));
  for (const file of actual) {
    const old = expected.get(file.path);
    if (!old || old.sha256 !== file.sha256 || old.bytes !== file.bytes)
      throw new Error(`导出文件缺失或内容改变：${file.path}`);
  }
  const entry = await safePath(root, manifest.entry);
  const handle = await fs.open(entry, "r");
  try {
    const magic = Buffer.alloc(4);
    await handle.read(magic, 0, 4, 0);
    const valid =
      manifest.platform === "Windows Desktop"
        ? magic.subarray(0, 2).equals(Buffer.from("MZ"))
        : manifest.platform === "Linux"
          ? magic.equals(Buffer.from([0x7f, 0x45, 0x4c, 0x46]))
          : magic.equals(Buffer.from([0x50, 0x4b, 0x03, 0x04]));
    if (!valid || (await handle.stat()).size < 1024)
      throw new Error("入口文件不是有效的目标平台程序或归档");
  } finally {
    await handle.close();
  }
  return {
    path: root,
    entry: manifest.entry,
    files: actual.length,
    bytes: actual.reduce((sum, item) => sum + item.bytes, 0),
  };
}
