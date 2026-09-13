import fs from "node:fs/promises";
import path from "node:path";
import { randomUUID } from "node:crypto";
import type { Asset, Feature, Project, GameBrief } from "../shared/types";
import { gameBriefSchema, featureSchema } from "../shared/game-design";
import { Store } from "./store";
import { listFiles, safePath } from "./files";
import {
  blueprintSchema,
  defaultBlueprint,
  type ProjectBlueprint,
} from "../shared/project-blueprint";
import {
  changedOverviewFields,
  overviewFromPlan,
  overviewSchema,
  overviewBlueprint,
  projectOverview,
} from "../shared/project-overview";

export async function readProjectDesign(
  root: string,
): Promise<GameBrief | undefined> {
  try {
    const metadata = JSON.parse(
      await fs.readFile(await safePath(root, "beaver.project.json"), "utf8"),
    );
    if (metadata.schemaVersion !== 1) throw new Error("未知项目元数据版本");
    return gameBriefSchema.parse(metadata.design);
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === "ENOENT") return undefined;
    throw new Error("项目的 beaver.project.json 无效，请修正后重试");
  }
}

export class Projects {
  constructor(
    private store: Store,
    private resources: string,
  ) {}
  list(): Project[] {
    return this.store.list<Project>("project");
  }
  async import(directory: string): Promise<Project> {
    const root = await fs.realpath(directory);
    await fs.access(path.join(root, "project.godot"));
    const internal = await fs.realpath(this.store.root);
    if (root === internal || root.startsWith(internal + path.sep))
      throw new Error("不能把应用内部数据注册为游戏项目");
    const design = await readProjectDesign(root);
    // Read current metadata after file I/O so import cannot overwrite a newer plan.
    const existing = this.list().find((p) => p.path === root);
    const p: Project = existing
      ? { ...existing }
      : {
          id: randomUUID(),
          name: path.basename(root),
          path: root,
          createdAt: new Date().toISOString(),
        };
    p.design = design;
    this.store.put("project", p.id, p);
    return p;
  }
  async create(
    parent: string,
    name: string,
    template: string,
    design?: GameBrief,
    blueprint?: ProjectBlueprint,
  ): Promise<Project> {
    const brief = design ? gameBriefSchema.parse(design) : undefined;
    const plan = blueprint ? blueprintSchema.parse(blueprint) : undefined;
    if (
      !/^[^<>:"/\\|?*\x00-\x1f]{1,80}$/.test(name) ||
      name === "." ||
      name === ".." ||
      /[. ]$/.test(name)
    )
      throw new Error("项目名称包含非法文件名字符");
    if (!["blank", "nightbar"].includes(template))
      throw new Error("模板不存在");
    const destination = path.join(await fs.realpath(parent), name);
    const internal = await fs.realpath(this.store.root);
    if (destination === internal || destination.startsWith(internal + path.sep))
      throw new Error("不能在应用内部数据目录创建项目");
    await fs.access(
      path.join(this.resources, "templates", template, "project.godot"),
    );
    await fs.mkdir(destination);
    await fs.cp(path.join(this.resources, "templates", template), destination, {
      recursive: true,
    });
    if (brief)
      await fs.writeFile(
        path.join(destination, "beaver.project.json"),
        JSON.stringify({ schemaVersion: 1, design: brief }, null, 2) + "\n",
        { flag: "wx" },
      );
    const project = await this.import(destination);
    if (!plan) return project;
    const initialized = { ...project, blueprint: plan, blueprintRevision: 1 };
    this.store.put("project", project.id, initialized);
    return initialized;
  }
  saveBlueprint(id: string, input: unknown, expectedRevision: number): Project {
    const blueprint = blueprintSchema.parse(input);
    const project = this.get(id);
    if ((project.blueprintRevision ?? 0) !== expectedRevision)
      throw new Error("项目规划已有更新。草稿已保留，请重新载入后再编辑。");
    if (
      changedOverviewFields(
        projectOverview(project),
        overviewFromPlan(project.name, blueprint),
      ).length
    )
      throw new Error("基础设定已锁定，请在总览中开启编辑后保存");
    const updated = {
      ...project,
      blueprint,
      blueprintRevision: expectedRevision + 1,
    };
    this.store.put("project", id, updated);
    return updated;
  }
  saveOverview(id: string, input: unknown, expectedRevision: number): Project {
    const overview = overviewSchema.parse(input);
    const project = this.get(id);
    if ((project.blueprintRevision ?? 0) !== expectedRevision)
      throw new Error("项目设定已有更新，草稿已保留，请重新载入后再编辑");
    const blueprint = blueprintSchema.parse(
      overviewBlueprint(
        overview,
        project.blueprint ?? defaultBlueprint(project.design),
      ),
    );
    if (!changedOverviewFields(projectOverview(project), overview).length)
      return project;
    const updated = {
      ...project,
      name: overview.name,
      blueprint,
      blueprintRevision: expectedRevision + 1,
    };
    this.store.put("project", id, updated);
    return updated;
  }
  get(id: string): Project {
    const p = this.store.get<Project>("project", id);
    if (!p) throw new Error("项目不存在");
    return p;
  }
  async assets(id: string): Promise<Asset[]> {
    const root = this.get(id).path;
    const assets: Asset[] = [];
    for (const relative of await listFiles(root)) {
      const file = await safePath(root, relative);
      const stat = await fs.stat(file);
      const ext = path.extname(relative).toLowerCase();
      const kind: Asset["kind"] = /\.(png|jpe?g|webp|gif|bmp|svg)$/.test(ext)
        ? "image"
        : /\.(wav|ogg|mp3|flac|m4a)$/.test(ext)
          ? "audio"
          : /\.(glb|gltf|obj|blend)$/.test(ext)
            ? "model"
            : /\.(tscn|scn)$/.test(ext)
              ? "scene"
              : /\.(gd|json|txt|md|csv|po|pot|cfg|godot|tres)$/.test(ext)
                ? "text"
                : "other";
      assets.push({
        path: relative,
        bytes: stat.size,
        kind,
        modifiedAt: stat.mtime.toISOString(),
      });
    }
    return assets;
  }
  async text(id: string, relative: string): Promise<string> {
    const file = await safePath(this.get(id).path, relative);
    if ((await fs.stat(file)).size > 512000)
      throw new Error("文本过大，请使用外部工具查看");
    return fs.readFile(file, "utf8");
  }
  async importAssets(id: string, sources: string[]): Promise<void> {
    const root = this.get(id).path;
    for (const source of sources) {
      if (!(await fs.stat(source)).isFile()) continue;
      const name = path.basename(source).replace(/[^\p{L}\p{N}._ -]/gu, "_");
      const destination = await safePath(
        root,
        `references/${randomUUID().slice(0, 8)}-${name}`,
      );
      await fs.mkdir(path.dirname(destination), { recursive: true });
      await fs.copyFile(source, destination, 1);
    }
  }
  async features(): Promise<Feature[]> {
    const out: Feature[] = [];
    for (const id of await fs.readdir(path.join(this.resources, "features")))
      out.push(
        featureSchema.parse(
          JSON.parse(
            await fs.readFile(
              path.join(this.resources, "features", id, "feature.json"),
              "utf8",
            ),
          ),
        ),
      );
    return out;
  }
}
