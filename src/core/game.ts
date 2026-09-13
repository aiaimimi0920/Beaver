import fs from "node:fs/promises";
import path from "node:path";
import os from "node:os";
import {
  downloadTemplates,
  installTemplateArchive,
  templateDirectory,
  templatesReady,
  templateVersion,
} from "./export-templates";
import { spawn, type ChildProcessWithoutNullStreams } from "node:child_process";
import { captureExportBundle, verifyExportBundle } from "./export-bundle";
import type { Project } from "../shared/types";
import { findTool, runCommand, terminate } from "./process";
import { Preferences } from "./settings";

export class Games {
  private running = new Set<ChildProcessWithoutNullStreams>();
  private exports = new Set<AbortController>();
  private closing = false;
  private templateSetup?: AbortController;
  constructor(private prefs: Preferences) {}
  async prepareTemplates(
    archive?: string,
  ): Promise<{ directory: string; version: string; reused: boolean }> {
    if (this.closing) throw new Error("应用正在退出");
    if (this.templateSetup) throw new Error("导出模板正在准备中");
    if (process.platform !== "win32")
      throw new Error("模板准备当前仅支持 Windows x86_64 导出");
    const controller = new AbortController();
    this.templateSetup = controller;
    let temporary: string | undefined;
    try {
      const executable = await findTool("godot", this.prefs.read().tools.godot);
      const result = await runCommand(
        executable,
        ["--version"],
        undefined,
        15000,
        controller.signal,
      );
      if (result.code !== 0) throw new Error("无法读取 Godot 版本");
      const version = templateVersion(result.output);
      const directory = await templateDirectory(executable, version);
      if (await templatesReady(directory))
        return { directory, version, reused: true };
      if (
        await fs.lstat(directory).then(
          () => true,
          () => false,
        )
      )
        throw new Error("已有模板目录不完整，未覆盖；请先检查并保留原有文件");
      if (!archive) {
        temporary = await fs.mkdtemp(
          path.join(os.tmpdir(), "beaver-templates-"),
        );
        archive = path.join(temporary, "templates.tpz");
        await downloadTemplates(result.output, archive, controller.signal);
      }
      await installTemplateArchive(
        archive,
        directory,
        version,
        controller.signal,
      );
      return { directory, version, reused: false };
    } finally {
      try {
        if (temporary) await fs.rm(temporary, { recursive: true, force: true });
      } finally {
        this.templateSetup = undefined;
      }
    }
  }
  cancelTemplates(): void {
    this.templateSetup?.abort();
  }
  private async importProject(
    godot: string,
    root: string,
    log: string,
    signal: AbortSignal,
  ): Promise<void> {
    let imported: Awaited<ReturnType<typeof runCommand>>;
    try {
      const help = await runCommand(godot, ["--help"], root, 15000, signal);
      if (help.code !== 0 || !help.output.includes("--import"))
        throw new Error(
          "当前 Godot 不支持可靠的无界面导入（--import），请在工具设置中选择支持该参数的 Godot 4 编辑器后重试。",
        );
      imported = await runCommand(
        godot,
        ["--headless", "--path", root, "--import"],
        root,
        180000,
        signal,
      );
    } catch (error) {
      await fs.writeFile(log, String(error), "utf8");
      throw new Error(`导入未完成；日志：${log}；${String(error)}`);
    }
    await fs.writeFile(log, imported.output, "utf8");
    if (imported.code !== 0 || imported.output.includes("ERROR:"))
      throw new Error(
        `导入失败；日志：${log}\n${imported.output.slice(-2000)}`,
      );
  }
  async play(project: Project): Promise<void> {
    if (this.closing) throw new Error("应用正在退出");
    const godot = await findTool("godot", this.prefs.read().tools.godot);
    const controller = new AbortController();
    const logDirectory = await fs.mkdtemp(
      path.join(os.tmpdir(), "beaver-play-"),
    );
    if (this.closing) throw new Error("应用正在退出");
    this.exports.add(controller);
    try {
      await this.importProject(
        godot,
        project.path,
        path.join(logDirectory, "import.log"),
        controller.signal,
      );
    } finally {
      this.exports.delete(controller);
    }
    if (this.closing) throw new Error("应用正在退出");
    const child = spawn(godot, ["--path", project.path], {
      windowsHide: false,
      stdio: "pipe",
      detached: process.platform !== "win32",
    });
    this.running.add(child);
    let startupLog = "";
    const collect = (data: Buffer) => {
      startupLog = (startupLog + data.toString()).slice(-8000);
    };
    child.stdout.on("data", collect);
    child.stderr.on("data", collect);
    child.once("exit", () => this.running.delete(child));
    child.once("error", () => this.running.delete(child));
    await new Promise<void>((resolve, reject) => {
      let timer: NodeJS.Timeout | undefined;
      const cleanup = () => {
        clearTimeout(timer);
        child.removeListener("exit", exited);
        child.removeListener("error", failed);
      };
      const failed = (error: Error) => {
        cleanup();
        reject(error);
      };
      const exited = (code: number | null) =>
        failed(
          new Error(`游戏启动后退出（${code}），请检查项目。\n${startupLog}`),
        );
      child.once("spawn", () => {
        timer = setTimeout(() => {
          cleanup();
          resolve();
        }, 900);
      });
      child.once("exit", exited);
      child.once("error", failed);
    });
  }
  async export(
    project: Project,
    destination: string,
    preset: string,
  ): Promise<{ path: string; log: string }> {
    if (this.closing) throw new Error("应用正在退出");
    const godot = await findTool("godot", this.prefs.read().tools.godot);
    if (!preset.trim()) throw new Error("请选择 Godot 导出预设");
    const parent = await fs.realpath(destination);
    const projectRoot = await fs.realpath(project.path);
    const relative = path.relative(projectRoot, parent);
    if (
      !relative ||
      (!path.isAbsolute(relative) &&
        relative !== ".." &&
        !relative.startsWith(".." + path.sep))
    )
      throw new Error("导出目录必须位于项目外，避免把构建产物重新导入资源");
    const config = await fs.readFile(
      path.join(project.path, "export_presets.cfg"),
      "utf8",
    );
    const target = exportTarget(config, preset);
    const folder = await fs.mkdtemp(path.join(parent, "Beaver-game-"));
    const executable = path.join(folder, target.entry);
    if (this.closing) throw new Error("应用正在退出");
    const controller = new AbortController();
    this.exports.add(controller);
    try {
      await this.importProject(
        godot,
        project.path,
        path.join(folder, "import.log"),
        controller.signal,
      );
      const result = await runCommand(
        godot,
        [
          "--headless",
          "--path",
          project.path,
          "--export-release",
          preset,
          executable,
        ],
        project.path,
        180000,
        controller.signal,
      );
      await fs.writeFile(path.join(folder, "export.log"), result.output);
      if (result.code !== 0 || result.output.includes("ERROR:"))
        throw new Error(
          `导出失败，未生成可运行交付。检查导出模板与预设。\n${result.output.slice(-6000)}`,
        );
      const files = await captureExportBundle(folder);
      const entry = files.find((file) => file.path === target.entry);
      if (!entry || entry.bytes < 1024) throw new Error("导出产物无效");
      await fs.writeFile(
        path.join(folder, "export-manifest.json"),
        JSON.stringify(
          {
            version: 2,
            project: project.name,
            preset,
            platform: target.platform,
            entry: path.basename(executable),
            sha256: entry.sha256,
            files,
            exportedAt: new Date().toISOString(),
            runtimeVerified: false,
          },
          null,
          2,
        ),
      );
      await verifyExportBundle(folder);
      return { path: folder, log: result.output };
    } finally {
      this.exports.delete(controller);
    }
  }
  async presets(project: Project): Promise<string[]> {
    try {
      return [
        ...(
          await fs.readFile(
            path.join(project.path, "export_presets.cfg"),
            "utf8",
          )
        ).matchAll(/^name="([^"]+)"/gm),
      ].map((m) => m[1]!);
    } catch {
      return [];
    }
  }
  async shutdown(): Promise<void> {
    this.closing = true;
    this.cancelTemplates();
    for (const controller of this.exports) controller.abort();
    await Promise.all([...this.running].map((p) => terminate(p)));
  }
}

export function exportTarget(
  config: string,
  preset: string,
): { platform: string; entry: string } {
  for (const section of config.split(/(?=^\[preset\.\d+\])/m)) {
    const header = section.split(/^\[/m).find((s) => s.startsWith("preset."));
    if (!header) continue;
    const name = /^name="([^"]+)"/m.exec(header)?.[1];
    const platform = /^platform="([^"]+)"/m.exec(header)?.[1];
    if (name !== preset) continue;
    const entry =
      platform === "Windows Desktop"
        ? "Game.exe"
        : platform === "Linux"
          ? "Game.x86_64"
          : platform === "macOS"
            ? "Game.zip"
            : undefined;
    if (!entry || !platform)
      throw new Error(
        "首版导出支持 Windows、Linux、macOS 桌面预设；其他渠道发行暂未开放",
      );
    return { platform, entry };
  }
  throw new Error("Godot 导出预设不存在");
}
