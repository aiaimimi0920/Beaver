import { dialog, shell, desktopCapturer } from "electron";
import { z } from "zod";
import fs from "node:fs/promises";
import {
  taskResources,
  taskResourceText,
  taskResourceBytes,
} from "../core/task-resources";
import { verifyExportBundle } from "../core/export-bundle";
import path from "node:path";
import { randomUUID } from "node:crypto";
import { Store } from "../core/store";
import { Tasks } from "../core/tasks";
import { Projects } from "../core/projects";
import { Preferences } from "../core/settings";
import { Games } from "../core/game";
import { findTool, runCommand } from "../core/process";
import { safePath } from "../core/files";
import type { Task, ToolStatus, Settings } from "../shared/types";
import { readLocalCodex } from "../core/local-codex";
import { readDocument } from "../core/documents";
import { directionSchema } from "../shared/task-board";
import { gameBriefSchema } from "../shared/game-design";
import { blueprintSchema } from "../shared/project-blueprint";
import { overviewSchema } from "../shared/project-overview";
import { ToolSetup } from "../core/tool-setup";
import { managedCodex } from "../core/managed-codex";
import type { ToolName, ToolSetupState } from "../shared/tool-setup";
import { matchesToolVersion } from "../shared/tool-setup";

const idSchema = z.object({ id: z.string().uuid() });
const assetSchema = z.object({
  id: z.string().uuid(),
  path: z.string().min(1).max(2000),
});
const taskInput = z.object({
  askRatio: z
    .union([
      z.literal(0),
      z.literal(10),
      z.literal(30),
      z.literal(70),
      z.literal(100),
    ])
    .nullable()
    .optional(),
  direction: directionSchema.optional(),
  projectId: z.string().uuid(),
  prompt: z.string().min(1).max(50000),
  title: z.string().max(120).optional(),
  stopConditions: z.string().max(5000).optional(),
  references: z
    .array(
      z.object({
        path: z.string().min(1).max(2000),
        note: z.string().max(5000),
        region: z
          .object({
            x: z.number().min(0).max(1),
            y: z.number().min(0).max(1),
            w: z.number().min(0).max(1),
            h: z.number().min(0).max(1),
          })
          .optional(),
      }),
    )
    .max(50)
    .optional(),
  maxMinutes: z.number().int().min(0).max(1440).optional(),
  capability: z.enum(["code", "review"]).optional(),
  decompose: z.boolean().optional(),
  autoAccept: z.boolean().optional(),
});
export class Api {
  private cancellation = new AbortController();
  private setup: ToolSetup;
  private configuredSetup?: Settings["tools"];
  shutdown(): void {
    this.setup.cancel();
    this.cancellation.abort();
  }
  constructor(
    private store: Store,
    private projects: Projects,
    private tasks: Tasks,
    private prefs: Preferences,
    private games: Games,
  ) {
    this.setup = new ToolSetup(
      {
        detect: (name, signal) =>
          this.detectOne(
            name,
            signal,
            true,
            this.configuredSetup?.[name] ?? this.prefs.read().tools[name],
          ),
        install: async (name, signal) => {
          await this.install(name, signal);
        },
        accept: (tool) => {
          const settings = this.prefs.read();
          settings.tools[tool.name] = tool.path;
          this.prefs.save(settings, {});
        },
        save: (state) => this.store.put("toolSetup", "main", state),
        redact: (error) => this.prefs.redact(error),
      },
      this.store.get<ToolSetupState>("toolSetup", "main"),
    );
  }
  async call(method: string, input: unknown): Promise<unknown> {
    switch (method) {
      case "task.approval": {
        const p = idSchema.extend({ autoAccept: z.boolean() }).parse(input);
        return this.tasks.approval(p.id, p.autoAccept);
      }
      case "task.autonomy": {
        const p = idSchema
          .extend({
            askRatio: z
              .union([
                z.literal(0),
                z.literal(10),
                z.literal(30),
                z.literal(70),
                z.literal(100),
              ])
              .nullable(),
          })
          .parse(input);
        return this.tasks.setAutonomy(p.id, p.askRatio);
      }
      case "state":
        return {
          projects: this.projects.list(),
          tasks: this.store.list<Task>("task").map((t) => ({
            ...t,
            effectiveAskRatio: t.askRatio ?? this.prefs.read().askRatio ?? 100,
            delivery: this.projects.list().find((p) => p.id === t.projectId)
              ?.delivery,
          })),
          settings: this.prefs.read(),
          features: await this.projects.features(),
        };
      case "chooseDirectory": {
        const result = await dialog.showOpenDialog({
          properties: ["openDirectory", "createDirectory"],
        });
        return result.canceled ? null : result.filePaths[0];
      }
      case "chooseTool": {
        const result = await dialog.showOpenDialog({
          properties: ["openFile"],
        });
        return result.canceled ? null : result.filePaths[0];
      }
      case "project.import":
        return this.projects.import(
          z.object({ path: z.string().min(1) }).parse(input).path,
        );
      case "project.create": {
        const p = z
          .object({
            parent: z.string().min(1),
            name: z.string().min(1),
            template: z.enum(["blank", "nightbar"]),
            design: gameBriefSchema.optional(),
            blueprint: blueprintSchema.optional(),
          })
          .parse(input);
        return this.projects.create(
          p.parent,
          p.name,
          p.template,
          p.design,
          p.blueprint,
        );
      }
      case "project.blueprint.save": {
        const p = z
          .object({
            id: z.string().uuid(),
            blueprint: blueprintSchema,
            expectedRevision: z.number().int().min(0),
          })
          .parse(input);
        return this.projects.saveBlueprint(
          p.id,
          p.blueprint,
          p.expectedRevision,
        );
      }
      case "project.overview.save": {
        const p = z
          .object({
            id: z.string().uuid(),
            overview: overviewSchema,
            expectedRevision: z.number().int().min(0),
            allowRiskyChanges: z.literal(true),
          })
          .strict()
          .parse(input);
        return this.projects.saveOverview(p.id, p.overview, p.expectedRevision);
      }
      case "project.reveal":
        return shell.openPath(this.projects.get(idSchema.parse(input).id).path);
      case "assets":
        return this.projects.assets(idSchema.parse(input).id);
      case "asset.text": {
        const p = assetSchema.parse(input);
        return this.projects.text(p.id, p.path);
      }
      case "asset.reveal": {
        const p = assetSchema.parse(input);
        shell.showItemInFolder(
          await safePath(this.projects.get(p.id).path, p.path),
        );
        return null;
      }
      case "asset.import": {
        const { id } = idSchema.parse(input);
        const result = await dialog.showOpenDialog({
          properties: ["openFile", "multiSelections"],
        });
        if (!result.canceled)
          await this.projects.importAssets(id, result.filePaths);
        return null;
      }
      case "task.create": {
        const p = taskInput.parse(input);
        for (const ref of p.references ?? [])
          await safePath(this.projects.get(p.projectId).path, ref.path);
        return this.tasks.create(p);
      }
      case "document.read": {
        const p = assetSchema.parse(input);
        return readDocument(this.projects.get(p.id).path, p.path);
      }
      case "document.save": {
        const p = assetSchema
          .extend({
            text: z.string().max(512000),
            revision: z
              .string()
              .regex(/^[a-f0-9]{64}$/)
              .nullable(),
          })
          .parse(input);
        return this.tasks.saveDocument(p.id, p.path, p.text, p.revision);
      }
      case "task.events":
        return this.store.events(idSchema.parse(input).id);
      case "task.resources":
        return taskResources(this.tasks.get(idSchema.parse(input).id));
      case "task.resourceText": {
        const p = assetSchema.parse(input);
        return taskResourceText(this.tasks.get(p.id), p.path);
      }
      case "task.resourceBytes": {
        const p = assetSchema.parse(input);
        return taskResourceBytes(this.tasks.get(p.id), p.path);
      }
      case "task.followup": {
        const p = idSchema
          .extend({ text: z.string().trim().min(1).max(10000) })
          .parse(input);
        return this.tasks.followup(p.id, p.text);
      }
      case "task.delegate": {
        const p = idSchema
          .extend({ text: z.string().trim().min(1).max(10000) })
          .parse(input);
        return this.tasks.delegate(p.id, p.text);
      }
      case "task.direction": {
        const p = idSchema.extend({ direction: directionSchema }).parse(input);
        return this.tasks.setDirection(p.id, p.direction);
      }
      case "task.interrupt":
        return this.tasks.interrupt(idSchema.parse(input).id);
      case "task.continue": {
        const p = z
          .object({
            id: z.string().uuid(),
            text: z.string().max(50000),
            freshContext: z.boolean().optional(),
          })
          .parse(input);
        if (p.freshContext)
          throw new Error("新会话恢复需要使用 Beaver 原生版本。");
        return this.tasks.continue(p.id, p.text);
      }
      case "task.accept":
        return this.tasks.accept(idSchema.parse(input).id);
      case "task.answer": {
        const p = z
          .object({
            id: z.string().uuid(),
            questionId: z.string().uuid(),
            answers: z.unknown(),
            automatic: z.array(z.string()).max(3).optional(),
          })
          .parse(input);
        return this.tasks.answer(p.id, p.questionId, p.answers, p.automatic);
      }
      case "task.rollback": {
        const p = z
          .object({
            id: z.string().uuid(),
            keep: z.array(z.string()).max(5000),
          })
          .parse(input);
        return this.tasks.rollback(p.id, p.keep);
      }
      case "task.dialogueRollback": {
        const p = z
          .object({ id: z.string().uuid(), text: z.string().min(1).max(10000) })
          .parse(input);
        return this.tasks.dialogueRollback(p.id, p.text);
      }
      case "task.reveal":
        return shell.openPath(
          this.tasks.get(idSchema.parse(input).id).workspace,
        );
      case "feature.add": {
        const p = z
          .object({
            projectId: z.string().uuid(),
            featureId: z.string().regex(/^[a-z0-9-]+$/),
          })
          .parse(input);
        return this.tasks.feature(p.projectId, p.featureId);
      }
      case "settings.save": {
        if (this.setup.read().status === "running")
          throw new Error("请等待工具准备结束后再保存设置");
        const p = z
          .object({
            settings: z.unknown(),
            keys: z.record(z.string(), z.string().max(10000)),
          })
          .parse(input);
        return this.prefs.save(p.settings, p.keys);
      }
      case "settings.clearKey":
        this.prefs.clearKey(z.object({ slot: z.string() }).parse(input).slot);
        return null;
      case "settings.importLocalCodex": {
        if (this.setup.read().status === "running")
          throw new Error("请等待工具准备结束后再导入配置");
        const p = z
          .object({
            settings: z.unknown(),
            keys: z.record(z.string(), z.string().max(10000)),
          })
          .parse(input);
        return this.prefs.importLocalDefaults(
          await readLocalCodex(),
          p.settings,
          p.keys,
        );
      }
      case "tools.detect": {
        const p = z
          .object({
            tools: z
              .object({
                codex: z.string().max(2000),
                godot: z.string().max(2000),
                blender: z.string().max(2000),
                node: z.string().max(2000),
              })
              .optional(),
          })
          .parse(input ?? {});
        return this.detect(p.tools);
      }
      case "tools.setupStatus":
        return this.setup.read();
      case "tools.cancelSetup":
        this.setup.cancel();
        return null;
      case "tools.setup": {
        if (this.setup.read().status === "running")
          throw new Error("已有环境准备正在进行");
        const p = z
          .object({
            tools: z
              .object({
                codex: z.string().max(2000),
                godot: z.string().max(2000),
                blender: z.string().max(2000),
                node: z.string().max(2000),
              })
              .optional(),
          })
          .parse(input ?? {});
        this.configuredSetup = p.tools;
        try {
          return await this.setup.prepare();
        } finally {
          this.configuredSetup = undefined;
        }
      }
      case "tools.install": {
        const name = z
          .object({ name: z.enum(["codex", "godot", "blender", "node"]) })
          .parse(input).name;
        const result = await this.setup.prepare([name]);
        if (result.status !== "completed")
          throw new Error(result.error ?? "环境准备未完成");
        return result;
      }
      case "game.play":
        return this.games.play(this.tasks.project(idSchema.parse(input).id));
      case "game.presets":
        return this.games.presets(this.projects.get(idSchema.parse(input).id));
      case "game.prepareTemplates":
        return this.games.prepareTemplates();
      case "game.cancelTemplates":
        return this.games.cancelTemplates();
      case "game.importTemplates": {
        const selected = await dialog.showOpenDialog({
          title: "导入可信的 Godot Windows x86_64 导出模板",
          properties: ["openFile"],
          filters: [{ name: "Godot 导出模板", extensions: ["tpz"] }],
        });
        if (selected.canceled || !selected.filePaths[0]) return null;
        return this.games.prepareTemplates(selected.filePaths[0]);
      }
      case "game.export": {
        const p = z
          .object({
            id: z.string().uuid(),
            destination: z.string(),
            preset: z.string().max(200),
          })
          .parse(input);
        return this.tasks.locks.run(p.id, async () => {
          const project = this.tasks.project(p.id);
          try {
            const result = await this.games.export(
              project,
              p.destination,
              p.preset,
            );
            project.delivery = {
              status: "verified",
              scope: "export-snapshot",
              checkedAt: new Date().toISOString(),
              path: result.path,
              runtimeVerified: false,
            };
            this.store.put("project", project.id, project);
            return result;
          } catch (error) {
            project.delivery = {
              status: "failed",
              scope: "export-snapshot",
              checkedAt: new Date().toISOString(),
              message: String(error),
              runtimeVerified: false,
            };
            this.store.put("project", project.id, project);
            throw error;
          }
        });
      }
      case "game.verifyExport": {
        const { path: folder } = z
          .object({ path: z.string().min(1).max(2000) })
          .parse(input);
        const project = this.projects
          .list()
          .find((p) => p.delivery?.path === folder);
        try {
          const result = await verifyExportBundle(folder);
          if (project) {
            project.delivery = {
              status: "verified",
              scope: "export-snapshot",
              checkedAt: new Date().toISOString(),
              path: folder,
              runtimeVerified: false,
            };
            this.store.put("project", project.id, project);
          }
          return result;
        } catch (error) {
          if (project) {
            project.delivery = {
              status: "failed",
              scope: "export-snapshot",
              checkedAt: new Date().toISOString(),
              path: folder,
              message: String(error),
              runtimeVerified: false,
            };
            this.store.put("project", project.id, project);
          }
          throw error;
        }
      }
      case "screenshots":
        return (
          await desktopCapturer.getSources({
            types: ["window"],
            thumbnailSize: { width: 640, height: 360 },
          })
        ).map((s) => ({
          id: s.id,
          name: s.name,
          image: s.thumbnail.toDataURL(),
        }));
      case "screenshot.capture": {
        const p = z
          .object({ id: z.string().uuid(), source: z.string() })
          .parse(input);
        const sources = await desktopCapturer.getSources({
          types: ["window"],
          thumbnailSize: { width: 1920, height: 1080 },
        });
        const source = sources.find((s) => s.id === p.source);
        if (!source) throw new Error("目标窗口已关闭");
        const relative = `references/capture-${randomUUID()}.png`;
        const dest = await safePath(this.projects.get(p.id).path, relative);
        await fs.mkdir(path.dirname(dest), { recursive: true });
        await fs.writeFile(dest, source.thumbnail.toPNG());
        return relative;
      }
      default:
        throw new Error("未知应用操作");
    }
  }
  private async detect(
    configured = this.prefs.read().tools,
  ): Promise<ToolStatus[]> {
    return Promise.all(
      (["codex", "godot", "blender", "node"] as const).map((name) =>
        this.detectOne(name, this.cancellation.signal, false, configured[name]),
      ),
    );
  }
  private async detectOne(
    name: ToolName,
    signal: AbortSignal,
    fallback = false,
    configured = this.prefs.read().tools[name],
  ): Promise<ToolStatus> {
    try {
      const managed =
        name === "codex"
          ? await managedCodex(path.join(this.store.root, "tools"))
          : undefined;
      let file: string;
      try {
        file = await findTool(name, configured || managed);
      } catch (error) {
        if (!fallback || !configured) throw error;
        file = await findTool(name, managed);
      }
      const result = await runCommand(
        file,
        ["--version"],
        undefined,
        15000,
        signal,
      );
      const version = this.prefs.redact(
        result.output.split("\n")[0]?.trim() ?? "",
      );
      const available = result.code === 0 && matchesToolVersion(name, version);
      return {
        name,
        path: file,
        available,
        version: available ? version : `版本验证失败：${version || "无输出"}`,
      };
    } catch (error) {
      return {
        name,
        path: "",
        available: false,
        version: this.prefs.redact(String(error)),
      };
    }
  }
  private async install(
    name: "codex" | "godot" | "blender" | "node",
    signal = this.cancellation.signal,
  ): Promise<string> {
    if (process.platform !== "win32")
      throw new Error(
        "当前自动安装仅验证 Windows。请安装工具后在设置中选择其可执行文件。",
      );
    if (name === "codex") {
      const prefix = path.join(this.store.root, "tools");
      const node = await findTool("node", this.prefs.read().tools.node);
      const npm = path.join(path.dirname(node), "npm.cmd");
      await fs.access(npm);
      const script = `& '${npm.replace(/'/g, "''")}' install --prefix '${prefix.replace(/'/g, "''")}' @openai/codex; exit $LASTEXITCODE`;
      const result = await runCommand(
        "powershell.exe",
        [
          "-NoProfile",
          "-NonInteractive",
          "-EncodedCommand",
          Buffer.from(script, "utf16le").toString("base64"),
        ],
        undefined,
        300000,
        signal,
      );
      if (result.code !== 0) throw new Error(result.output);
      const binary = await managedCodex(prefix);
      if (!binary)
        throw new Error("Codex 安装包中未找到当前架构的程序，未写入无效配置");
      return result.output;
    }
    const ids = {
      godot: "GodotEngine.GodotEngine",
      blender: "BlenderFoundation.Blender",
      node: "OpenJS.NodeJS.LTS",
    };
    const result = await runCommand(
      "winget.exe",
      [
        "install",
        "--id",
        ids[name],
        "--exact",
        "--accept-package-agreements",
        "--accept-source-agreements",
      ],
      undefined,
      600000,
      signal,
    );
    if (result.code !== 0) throw new Error(result.output);
    return result.output;
  }
}
