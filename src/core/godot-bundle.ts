import fs from "node:fs/promises";
import path from "node:path";
import { templateVersion } from "./export-templates";
import { runCommand } from "./process";
import { safePath } from "./files";

export interface GodotBundle {
  directory: string;
  debug: string;
  release: string;
  version: string;
}
const names = {
  debug: "godot.windows.template_debug.x86_64.exe",
  release: "godot.windows.template_release.x86_64.exe",
};

export function requireBundleVersion(expected: string, actual: string): void {
  if (actual !== expected)
    throw new Error("本地导出模板与编辑器版本不一致，未回退其他版本");
}

export async function discoverGodotBundle(
  executable: string,
  signal: AbortSignal,
): Promise<GodotBundle | undefined> {
  if (
    !/^godot\.windows\.editor\.x86_64(?:\.console)?\.exe$/i.test(
      path.basename(executable),
    )
  )
    return undefined;
  const directory = await fs.realpath(path.dirname(executable));
  const debug = path.join(directory, names.debug);
  const release = path.join(directory, names.release);
  for (const file of [debug, release]) {
    const stat = await fs.lstat(file).catch(() => {
      throw new Error(`本地配套导出模板缺失，未回退其他版本：${file}`);
    });
    if (!stat.isFile() || stat.isSymbolicLink() || stat.size < 1024)
      throw new Error(`本地导出模板不是有效文件：${file}`);
  }
  async function probe(file: string) {
    const result = await runCommand(
      file,
      ["--version"],
      undefined,
      15000,
      signal,
    );
    if (result.code !== 0) throw new Error(`无法读取 Godot 版本：${file}`);
    templateVersion(result.output);
    return result.output.trim();
  }
  const version = await probe(executable);
  for (const file of [debug, release])
    requireBundleVersion(version, await probe(file));
  return { directory, debug, release, version };
}

export function bundleReceipt(bundle: GodotBundle, executable: string) {
  return {
    source: "editor-siblings",
    engine: executable,
    version: bundle.version,
    debugTemplate: bundle.debug,
    releaseTemplate: bundle.release,
  };
}

export async function bundlePreset(
  bundle: GodotBundle,
  config: string,
  preset: string,
  root: string,
): Promise<string> {
  let section = "";
  const selected: string[] = [];
  for (const line of config.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (trimmed.startsWith("[")) section = trimmed;
    const match = /^name\s*=\s*(.+)$/.exec(trimmed);
    if (
      /^\[preset\.\d+\]$/.test(section) &&
      match &&
      JSON.parse(match[1]!) === preset
    )
      selected.push(section.replace(/\]$/, ".options]"));
  }
  if (selected.length !== 1) throw new Error("Godot 导出预设不存在或名称重复");
  const options = selected[0]!;
  const output: string[] = [];
  let found = false;
  const injected = () => {
    for (const key of ["debug", "release"] as const)
      output.push(
        `custom_template/${key}=${JSON.stringify(bundle[key].replace(/\\/g, "/"))}`,
      );
  };
  for (const line of config.trimEnd().split(/\r?\n/)) {
    const trimmed = line.trim();
    if (trimmed.startsWith("[")) {
      section = trimmed;
      if (section === options) {
        found = true;
        output.push(line);
        injected();
        continue;
      }
    }
    if (section === options) {
      const match = /^custom_template\/(debug|release)\s*=\s*(.+)$/.exec(
        trimmed,
      );
      if (match) {
        const value: string = JSON.parse(match[2]!);
        if (value) {
          const file = value.startsWith("res://")
            ? await safePath(root, value.slice(6))
            : path.isAbsolute(value)
              ? value
              : await safePath(root, value);
          if (
            (await fs.realpath(file)) !==
            (await fs.realpath(bundle[match[1] as "debug" | "release"]))
          )
            throw new Error(
              "项目自定义模板与当前本地配套模板冲突；请显式调整预设，未覆盖原项目",
            );
        }
        continue;
      }
      const architecture = /^binary_format\/architecture\s*=\s*(.+)$/.exec(
        trimmed,
      );
      if (architecture && JSON.parse(architecture[1]!) !== "x86_64")
        throw new Error("本地配套模板仅支持 Windows x86_64");
    }
    output.push(line);
  }
  if (!found) {
    output.push(options);
    injected();
  }
  return output.join("\n") + "\n";
}
