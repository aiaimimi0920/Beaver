import fs from "node:fs/promises";
import { createWriteStream } from "node:fs";
import { createHash } from "node:crypto";
import path from "node:path";
import { Readable, Transform } from "node:stream";
import { pipeline } from "node:stream/promises";
import { runCommand } from "./process";

const required = ["windows_release_x86_64.exe", "windows_debug_x86_64.exe"];
const limit = 2 * 1024 ** 3;

export function templateVersion(output: string): string {
  const match =
    /^(4\.\d+(?:\.\d+)?)\.(stable|rc\d+|beta\d+|dev\d+)(?:\.|$)/.exec(
      output.trim(),
    );
  if (!match) throw new Error("无法识别 Godot 4 导出模板版本");
  return `${match[1]}.${match[2]}`;
}

export function templateRelease(output: string) {
  const version = templateVersion(output);
  if (!/^4\.\d+(?:\.\d+)?\.stable\.official\./.test(output.trim()))
    throw new Error(
      "自动下载仅支持官方稳定版 Godot；自定义引擎请导入匹配的模板包",
    );
  const tag = version.replace(/\.stable$/, "-stable");
  const filename = `Godot_v${tag}_export_templates.tpz`;
  return {
    version,
    filename,
    base: `https://github.com/godotengine/godot-builds/releases/download/${tag}`,
  };
}

export function templateChecksum(sums: string, filename: string): string {
  const matches = sums
    .split(/\r?\n/)
    .map((line) => /^([a-f0-9]{128})\s+\*?(.+)$/i.exec(line.trim()))
    .filter((match) => match?.[2] === filename);
  if (matches.length !== 1)
    throw new Error("官方校验清单中没有唯一匹配的模板包");
  return matches[0]![1]!.toLowerCase();
}

export async function templateDirectory(
  executable: string,
  version: string,
  appData = process.env.APPDATA,
): Promise<string> {
  if (templateVersion(version) !== version) throw new Error("无效模板目录版本");
  const folder = path.dirname(executable);
  for (const marker of ["._sc_", "_sc_"]) {
    if (
      await fs.stat(path.join(folder, marker)).then(
        (s) => s.isFile(),
        () => false,
      )
    )
      return path.join(folder, "editor_data", "export_templates", version);
  }
  if (!appData || !path.isAbsolute(appData))
    throw new Error("无法确定 Godot 数据目录");
  return path.join(appData, "Godot", "export_templates", version);
}

export async function templatesReady(directory: string): Promise<boolean> {
  for (const file of required) {
    try {
      const stat = await fs.lstat(path.join(directory, file));
      if (!stat.isFile() || stat.isSymbolicLink() || stat.size < 1024)
        return false;
      const handle = await fs.open(path.join(directory, file), "r");
      try {
        const bytes = Buffer.alloc(2);
        await handle.read(bytes, 0, 2, 0);
        if (bytes.toString() !== "MZ") return false;
      } finally {
        await handle.close();
      }
    } catch {
      return false;
    }
  }
  return true;
}

export async function downloadTemplates(
  output: string,
  destination: string,
  signal: AbortSignal,
): Promise<void> {
  const release = templateRelease(output);
  const combined = AbortSignal.any([
    signal,
    AbortSignal.timeout(20 * 60 * 1000),
  ]);
  const sums = await fetch(`${release.base}/SHA512-SUMS.txt`, {
    signal: combined,
  });
  if (!sums.ok)
    throw new Error(`获取官方模板校验清单失败：HTTP ${sums.status}`);
  const checksum = templateChecksum(
    await boundedText(sums, 1024 * 1024),
    release.filename,
  );
  const response = await fetch(`${release.base}/${release.filename}`, {
    signal: combined,
  });
  if (!response.ok || !response.body)
    throw new Error(`下载模板失败：HTTP ${response.status}`);
  let bytes = 0;
  const hash = createHash("sha512");
  const measure = new Transform({
    transform(chunk: Buffer, _encoding, callback) {
      bytes += chunk.length;
      if (bytes > limit) {
        callback(new Error("模板包超过 2 GiB 限制"));
        return;
      }
      hash.update(chunk);
      callback(null, chunk);
    },
  });
  await pipeline(
    Readable.from(responseChunks(response)),
    measure,
    createWriteStream(destination, { flags: "wx" }),
    { signal: combined },
  );
  if (hash.digest("hex") !== checksum)
    throw new Error("模板包 SHA-512 校验失败，未安装");
}

async function boundedText(response: Response, max: number): Promise<string> {
  if (!response.body) throw new Error("校验清单为空");
  const chunks: Uint8Array[] = [];
  let size = 0;
  for await (const chunk of responseChunks(response)) {
    size += chunk.length;
    if (size > max) throw new Error("校验清单过大");
    chunks.push(chunk);
  }
  return Buffer.concat(chunks).toString("utf8");
}

async function* responseChunks(response: Response): AsyncGenerator<Uint8Array> {
  const reader = response.body!.getReader();
  try {
    while (true) {
      const { done, value } = await reader.read();
      if (done) return;
      yield value;
    }
  } finally {
    await reader.cancel().catch(() => {});
    reader.releaseLock();
  }
}

export async function installTemplateArchive(
  archive: string,
  directory: string,
  version: string,
  signal: AbortSignal,
): Promise<void> {
  if (process.platform !== "win32")
    throw new Error("模板自动安装当前仅支持 Windows");
  if (templateVersion(version) !== version) throw new Error("无效模板版本");
  if (
    await fs.lstat(directory).then(
      () => true,
      () => false,
    )
  )
    throw new Error("模板目录已经存在，未覆盖；请保留并检查已有模板");
  const parent = path.dirname(directory);
  await fs.mkdir(parent, { recursive: true });
  const stage = await fs.mkdtemp(path.join(parent, ".beaver-templates-"));
  const literal = (value: string) => `'${value.replace(/'/g, "''")}'`;
  const script = `$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem
$zip = [IO.Compression.ZipFile]::OpenRead(${literal(archive)})
try {
  $versionEntries = @($zip.Entries | Where-Object { $_.FullName -ceq 'templates/version.txt' })
  if ($versionEntries.Count -ne 1 -or $versionEntries[0].Length -gt 128) { throw 'Invalid template version entry' }
  $reader = [IO.StreamReader]::new($versionEntries[0].Open())
  try { $version = $reader.ReadToEnd().Trim() } finally { $reader.Dispose() }
  if ($version -cne ${literal(version)}) { throw 'Template version does not match engine' }
  foreach ($name in @('windows_release_x86_64.exe', 'windows_debug_x86_64.exe')) {
    $entries = @($zip.Entries | Where-Object { $_.FullName -ceq ('templates/' + $name) })
    if ($entries.Count -ne 1 -or $entries[0].Length -lt 1024 -or $entries[0].Length -gt 536870912) { throw 'Invalid Windows template entry' }
    [IO.Compression.ZipFileExtensions]::ExtractToFile($entries[0], [IO.Path]::Combine(${literal(stage)}, $name), $false)
  }
  [IO.File]::WriteAllText([IO.Path]::Combine(${literal(stage)}, 'version.txt'), $version, [Text.UTF8Encoding]::new($false))
} finally { $zip.Dispose() }
`;
  try {
    const result = await runCommand(
      path.join(
        process.env.SystemRoot ?? "C:/Windows",
        "System32/WindowsPowerShell/v1.0/powershell.exe",
      ),
      [
        "-NoProfile",
        "-NonInteractive",
        "-EncodedCommand",
        Buffer.from(script, "utf16le").toString("base64"),
      ],
      undefined,
      180000,
      signal,
    );
    if (result.code !== 0)
      throw new Error(`模板包安装失败：${result.output.slice(-2000)}`);
    if (!(await templatesReady(stage))) throw new Error("模板程序验证失败");
    signal.throwIfAborted();
    await fs.rename(stage, directory);
  } finally {
    // Only remove the exact staging directory created by this invocation.
    await fs.rm(stage, { recursive: true, force: true });
  }
}
