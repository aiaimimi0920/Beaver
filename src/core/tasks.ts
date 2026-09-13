import fs from "node:fs/promises";
import path from "node:path";
import { randomUUID } from "node:crypto";
import type {
  Project,
  Task,
  Reference,
  FeatureAdoption,
} from "../shared/types";
import { Store } from "./store";
import { Files, ProjectLocks, safePath } from "./files";
import { documentPath } from "./documents";
import { Preferences } from "./settings";
import { LocalExecutor } from "./executor";
import { Journal, type FileOperation } from "./journal";
import { readProjectDesign } from "./projects";
import { featureSchema } from "../shared/game-design";
import { enginePackageId } from "../shared/engine-packages";
import { validateAnswers } from "../shared/clarifications";
import type { CodexRpc } from "./rpc";
import { directionSchema, type TaskDirection } from "../shared/task-board";
import { blueprintSchema } from "../shared/project-blueprint";
import { formatTaskBrief } from "../shared/task-brief";
import { planEligible, preparePlannedTask, reconcilePlans } from "./task-plans";
import { taskPlanSchema } from "../shared/task-plan";

export class Tasks {
  private executor: LocalExecutor;
  private closing = false;
  private starting = new Set<string>();
  private preparing = new Set<string>();
  private answering = new Set<string>();
  private journal: Journal;
  readonly ready: Promise<void>;
  constructor(
    private store: Store,
    readonly files: Files,
    readonly locks: ProjectLocks,
    private prefs: Preferences,
    private resources: string,
    mcpScript: string,
    private notify: () => void,
    rpcFactory?: () => CodexRpc,
  ) {
    this.journal = new Journal(store, files);
    this.ready = this.journal.recover().then(() => store.recover());
    this.executor = new LocalExecutor(
      store.root,
      resources,
      mcpScript,
      prefs,
      (id, kind, text) => {
        store.event(id, kind, text);
        notify();
      },
      (task) => this.save(task),
      (task, status, error) => this.finish(task, status, error),
      rpcFactory,
    );
  }
  get(id: string): Task {
    const task = this.store.get<Task>("task", id);
    if (!task) throw new Error("任务不存在");
    return task;
  }
  setAutonomy(id: string, askRatio: unknown): Task {
    const value = askRatioSchema.nullable().parse(askRatio);
    const task = this.get(id);
    task.askRatio = value;
    this.executor.setAskRatio(id, value);
    this.save(task);
    return {
      ...task,
      effectiveAskRatio: value ?? this.prefs.read().askRatio ?? 100,
    };
  }
  project(id: string): Project {
    const p = this.store.get<Project>("project", id);
    if (!p) throw new Error("项目不存在");
    if (
      this.store
        .list<FileOperation>("operation")
        .some(
          (op) =>
            op.projectId === id && ["applying", "aborting"].includes(op.state),
        )
    )
      throw new Error(
        "项目存在未完成的文件恢复，请恢复原路径/权限后重启 Beaver，避免进一步修改。",
      );
    return p;
  }
  private save(task: Task): void {
    task.autoAccept =
      this.store.get<Task>("task", task.id)?.autoAccept ?? task.autoAccept;
    task.direction =
      this.store.get<Task>("task", task.id)?.direction ?? task.direction;
    task.updatedAt = new Date().toISOString();
    this.store.put("task", task.id, task);
    this.notify();
  }
  async create(
    input: {
      askRatio?: AskRatio | null;
      projectId: string;
      prompt: string;
      title?: string;
      stopConditions?: string;
      references?: Reference[];
      maxMinutes?: number;
      capability?: "code" | "review";
      direction?: TaskDirection;
      decompose?: boolean;
      autoAccept?: boolean;
      dependsOn?: string[];
    },
    context?: (task: Task) => Promise<void>,
    options: { deferPump?: boolean } = {},
  ): Promise<Task> {
    await this.ready;
    if (this.closing) throw new Error("正在退出，无法创建任务");
    const project = this.project(input.projectId);
    const id = randomUUID();
    return this.locks.run(project.id, async () => {
      const current = this.project(project.id);
      const projectContext = current.blueprint
        ? {
            name: current.name,
            revision: current.blueprintRevision ?? 0,
            blueprint: blueprintSchema.parse(current.blueprint),
          }
        : undefined;
      const baseline = await this.files.capture(project.path);
      const workspace = path.join(this.store.root, "workspaces", id);
      await this.files.restoreCopy(baseline, workspace);
      // Read the captured file, not stale catalog state or a concurrently edited source.
      const design = await readProjectDesign(workspace);
      const task: Task = {
        askRatio: input.askRatio ?? null,
        id,
        projectId: project.id,
        title: input.title || input.prompt.slice(0, 60),
        prompt: input.prompt,
        design,
        projectContext,
        stopConditions: input.stopConditions ?? "",
        references: input.references ?? [],
        maxMinutes: input.maxMinutes ?? 0,
        capability: input.capability ?? "code",
        decompose:
          input.capability !== "review" &&
          (input.decompose ??
            (!context && (!input.direction || input.direction === "general"))),
        autoAccept: input.autoAccept ?? true,
        subtaskIds: [],
        dependsOn: input.dependsOn ?? [],
        direction: input.direction,
        status: "queued",
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
        workspace,
        baseline,
        changes: [],
        conflicts: [],
      };
      if (projectContext) {
        const dir = path.join(workspace, ".beaver-context/project");
        await fs.mkdir(dir, { recursive: true });
        await fs.writeFile(
          path.join(dir, "blueprint.json"),
          JSON.stringify(projectContext, null, 2),
          "utf8",
        );
        await fs.writeFile(
          path.join(dir, "brief.md"),
          formatTaskBrief(task),
          "utf8",
        );
      }
      if (context) await context(task);
      this.save(task);
      this.store.event(id, "user", input.prompt);
      if (!options.deferPump) this.pump();
      return task;
    });
  }
  private pump(): void {
    if (this.closing) return;
    reconcilePlans(this.store);
    const tasks = this.store.list<Task>("task");
    let free =
      this.prefs.read().maxParallel -
      tasks.filter((t) => t.status === "running").length;
    for (const task of tasks.reverse())
      if (
        free > 0 &&
        task.status === "queued" &&
        !this.starting.has(task.id) &&
        planEligible(task, tasks)
      ) {
        free--;
        this.starting.add(task.id);
        this.preparing.add(task.id);
        task.status = "running";
        task.error = undefined;
        this.save(task);
        void this.locks
          .run(task.projectId, async () => {
            this.project(task.projectId);
            await preparePlannedTask(this.store, this.files, task);
            if (this.get(task.id).status === "interrupted")
              task.status = "interrupted";
            this.save(task);
          })
          .then(async () => {
            this.preparing.delete(task.id);
            if (this.closing || this.get(task.id).status !== "running") return;
            if (task.decompose && task.plan)
              await this.finish(task, "completed");
            else await this.executor.start(task);
          })
          .catch((error: unknown) => {
            task.status = "failed";
            task.error = String(error);
            this.save(task);
          })
          .finally(() => {
            this.starting.delete(task.id);
            this.preparing.delete(task.id);
          });
      }
  }
  private async finish(
    task: Task,
    status: string,
    error?: string,
  ): Promise<void> {
    try {
      if (status === "completed" && task.capability !== "review") {
        const normalized = await normalizeSources(
          task.workspace,
          task.baseline,
        );
        if (normalized.length)
          this.store.event(
            task.id,
            "encoding",
            `已规范化为 UTF-8 无 BOM：${normalized.join(", ")}`,
          );
      }
      task.changes = this.files.changes(
        task.baseline,
        await this.files.capture(task.workspace),
      );
      task.error = error;
      if (task.clarifications?.some((q) => !q.answers)) {
        task.status = "awaitingInput";
      } else if (status === "completed" && !this.closing) {
        if (task.decompose) taskPlanSchema.parse(task.plan);
        if (task.capability === "review" && task.changes.length) {
          task.status = "conflict";
          task.error = "审查任务修改了文件，未合入原项目。";
        } else
          await this.locks.run(task.projectId, async () => {
            if (this.closing) {
              task.status = "interrupted";
              return;
            }
            const p = this.project(task.projectId);
            task.conflicts = await this.files.conflicts(p.path, task.changes);
            if (task.conflicts.length) task.status = "conflict";
            else {
              task.status = task.decompose ? "waitingChildren" : "completed";
              await this.journal.apply(p, task.changes, task, "merge");
            }
          });
      } else
        task.status =
          this.closing || status === "interrupted" ? "interrupted" : "failed";
    } catch (e) {
      task.status = task.clarifications?.some((q) => !q.answers)
        ? "awaitingInput"
        : "failed";
      task.error = String(e);
    }
    if (
      task.status === "completed" &&
      this.get(task.id).autoAccept &&
      task.relation === "child"
    ) {
      task.accepted = true;
      task.approvalSource = "automatic";
      this.store.event(task.id, "approval", "按子任务自动审批策略认可。");
    }
    this.save(task);
    this.store.event(
      task.id,
      "status",
      `${task.status}${task.error ? ": " + task.error : ""}`,
    );
    this.pump();
  }
  async interrupt(id: string): Promise<void> {
    const t = this.get(id);
    if (t.status === "waitingChildren") {
      t.planPaused = true;
      this.save(t);
      for (const childId of t.subtaskIds ?? []) await this.interrupt(childId);
      return;
    }
    if (t.status === "queued" || this.preparing.has(id)) {
      t.status = "interrupted";
      this.save(t);
    } else if (t.status === "running") await this.executor.interrupt(id);
  }
  setDirection(id: string, direction: TaskDirection): void {
    const task = this.get(id);
    task.direction = directionSchema.parse(direction);
    task.updatedAt = new Date().toISOString();
    this.store.put("task", id, task);
    this.notify();
  }
  async continue(id: string, text: string): Promise<void> {
    const task = this.get(id);
    this.project(task.projectId);
    if (task.status === "waitingChildren") {
      if (text) throw new Error("请打开具体子任务补充要求");
      task.planPaused = false;
      this.save(task);
      for (const childId of task.subtaskIds ?? []) {
        const child = this.get(childId);
        if (child.status === "interrupted") {
          child.status = "queued";
          this.save(child);
        }
      }
      this.pump();
      return;
    }
    if (task.status === "awaitingInput")
      throw new Error("请先回答待补充的问题");
    if (task.status === "running") {
      this.store.event(id, "user", text);
      try {
        await this.executor.steer(id, text);
      } catch (error) {
        this.store.event(
          id,
          "system",
          "补充要求已记录，但未确认被运行中的 Codex 接受。请在继续任务时确认。",
        );
        throw error;
      }
      return;
    }
    if (["completed", "rolledBack", "conflict"].includes(task.status))
      throw new Error("已交付或冲突任务请创建后续任务，保留独立回退边界");
    if (task.status === "queued") {
      task.prompt += "\n\n补充要求：" + text;
      this.save(task);
      this.store.event(id, "user", text);
      return;
    }
    task.prompt +=
      "\n\n继续要求：" +
      (text || "继续未完成的目标，先检查已有进度，避免重复已完成的工作。");
    task.status = "queued";
    this.save(task);
    this.pump();
  }
  async answer(
    id: string,
    questionId: string,
    input: unknown,
    automatic: string[] = [],
  ): Promise<void> {
    await this.ready;
    if (this.closing) throw new Error("正在退出，请下次启动后回答");
    if (this.answering.has(id)) throw new Error("正在提交回答");
    this.answering.add(id);
    try {
      const current = this.get(id);
      if (current.status !== "awaitingInput")
        throw new Error("任务当前没有等待回答");
      const pending = current.clarifications?.find((q) => q.id === questionId);
      if (!pending) throw new Error("问题不存在或已过期");
      const checked = validateAnswers(pending, input);
      if (new Set(automatic).size !== automatic.length)
        throw new Error("自动回答标识重复");
      for (const key of automatic) {
        const q = pending.questions.find((q) => q.id === key);
        if (
          !q ||
          automaticChoice(
            current.askRatio ?? this.prefs.read().askRatio ?? 100,
            q,
          ) !== checked[key]
        )
          throw new Error("自动回答与策略或推荐不匹配");
      }
      const projectId = current.projectId;
      await this.executor.interrupt(id);
      await this.locks.run(projectId, async () => {
        if (this.closing) throw new Error("正在退出，请下次启动后回答");
        const task = this.get(id);
        this.project(projectId);
        if (task.status !== "awaitingInput")
          throw new Error("任务当前没有等待回答");
        const item = task.clarifications?.find((q) => q.id === questionId);
        if (!item) throw new Error("问题不存在或已过期");
        const answers = validateAnswers(item, input);
        const ratio = task.askRatio ?? this.prefs.read().askRatio ?? 100;
        if (new Set(automatic).size !== automatic.length)
          throw new Error("自动回答标识重复");
        for (const key of automatic) {
          const question = item.questions.find((q) => q.id === key);
          if (!question || automaticChoice(ratio, question) !== answers[key])
            throw new Error("自动回答与策略或推荐不匹配");
        }
        item.autoAnswers ??= {};
        item.automaticQuestions ??= [];
        for (const key of automatic) {
          item.autoAnswers[key] = answers[key]!;
          item.automaticQuestions.push(
            item.questions.find((q) => q.id === key)!,
          );
        }
        item.answers = answers;
        item.answerSources = Object.fromEntries(
          Object.keys(answers).map((id) => [
            id,
            automatic.includes(id) ? "automatic" : "user",
          ]),
        );
        if (automatic.length) item.appliedAskRatio = ratio;
        item.answeredAt = new Date().toISOString();
        const text = item.questions
          .map(
            (q) =>
              `${q.question}\n${automatic.includes(q.id) ? "按当前设置自动采用" : "用户回答"}：${answers[q.id]}`,
          )
          .join("\n\n");
        task.prompt +=
          "\n\n用户已确认的补充（先检查工作副本，继续未完成目标）：\n" + text;
        task.status = "queued";
        task.error = undefined;
        this.store.transaction(() => {
          this.save(task);
          this.store.event(id, automatic.length ? "decision" : "user", text);
        });
      });
      this.pump();
    } finally {
      this.answering.delete(id);
    }
  }
  async saveDocument(
    projectId: string,
    relative: string,
    text: string,
    revision: string | null,
  ): Promise<void> {
    await this.ready;
    documentPath(relative);
    if (Buffer.byteLength(text, "utf8") > 512000)
      throw new Error("资料超过 500 KB");
    await this.locks.run(projectId, async () => {
      if (this.closing) throw new Error("正在退出，无法保存");
      const project = this.project(projectId);
      await safePath(project.path, relative);
      const baseline = await this.files.capture(project.path);
      if ((baseline[relative] ?? null) !== revision)
        throw new Error(
          "资料已被其他任务或编辑器修改；草稿已保留，请重新读取后整合",
        );
      const id = randomUUID();
      const workspace = path.join(this.store.root, "workspaces", id);
      await this.files.restoreCopy(baseline, workspace);
      const file = await safePath(workspace, relative);
      await fs.mkdir(path.dirname(file), { recursive: true });
      await fs.writeFile(file, text.replace(/^\uFEFF/, ""), "utf8");
      const task: Task = {
        id,
        projectId,
        title: "编辑资料 · " + relative,
        prompt: "人工编辑资料",
        stopConditions: "",
        status: "interrupted",
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
        workspace,
        baseline,
        references: [],
        conflicts: [],
        maxMinutes: 0,
        capability: "code",
        changes: this.files.changes(
          baseline,
          await this.files.capture(workspace),
        ),
        report: "人工修改等待合入。",
      };
      this.save(task);
      task.status = "completed";
      task.report = "人工修改已保存。";
      try {
        await this.journal.apply(project, task.changes, task, "merge");
      } catch (error) {
        task.status = "conflict";
        task.report = "人工修改未完成合入，工作副本已保留。";
        task.error = String(error);
        this.save(task);
        throw error;
      }
      this.notify();
    });
  }
  async rollback(id: string, keep: string[]): Promise<void> {
    const task = this.get(id);
    if (task.status !== "completed")
      throw new Error("仅已合入的任务可标准回退；其他任务副本不会覆盖原项目");
    await this.locks.run(task.projectId, async () => {
      const reverse = task.changes
        .filter((c) => !keep.includes(c.path))
        .map((c) => ({ path: c.path, before: c.after, after: c.before }));
      const p = this.project(task.projectId);
      task.status = "rolledBack";
      task.accepted = false;
      task.retainedFiles = keep;
      await this.journal.apply(p, reverse, task, "rollback");
      this.save(task);
      this.store.event(
        id,
        "rollback",
        `标准回退完成；保留：${keep.join(", ") || "无"}`,
      );
    });
  }
  async dialogueRollback(id: string, instruction: string): Promise<Task> {
    const source = this.get(id);
    return this.create(
      {
        projectId: source.projectId,
        title: "对话回退 · " + source.title,
        prompt: `请处理任务「${source.title}」的选择性回退/冲突整合。\n用户要求：${instruction}\n原任务变更记录与前后文件保存在 .beaver-context/rollback。保留其他任务的成果，不能直接覆盖整个项目。`,
      },
      async (task) => {
        const dir = path.join(task.workspace, ".beaver-context/rollback");
        await fs.mkdir(dir, { recursive: true });
        await fs.writeFile(
          path.join(dir, "changes.json"),
          JSON.stringify(source.changes, null, 2),
        );
        for (const side of ["before", "after"] as const) {
          const snapshot = Object.fromEntries(
            source.changes
              .filter((c) => c[side])
              .map((c) => [c.path, c[side]!]),
          );
          await this.files.restoreCopy(snapshot, path.join(dir, side));
        }
      },
    );
  }
  accept(id: string): void {
    const task = this.get(id);
    if (task.status !== "completed") throw new Error("任务尚未完成合入");
    if (task.accepted) return;
    if (task.feature) {
      const adopted = this.store.get<FeatureAdoption>(
        "feature",
        `${task.projectId}:${task.feature.id}`,
      );
      if (adopted?.taskId !== task.feature.previous?.taskId)
        throw new Error(
          "该功能块已由另一任务更新，请重新整合后认可，避免倒退官方版本基线。",
        );
    }
    task.accepted = true;
    task.approvalSource = "user";
    this.store.transaction(() => {
      this.save(task);
      if (task.feature)
        this.store.put("feature", `${task.projectId}:${task.feature.id}`, {
          id: task.feature.id,
          version: task.feature.version,
          snapshot: task.feature.snapshot,
          taskId: task.id,
        });
    });
    this.pump();
  }
  approval(id: string, automatic: boolean): Task {
    const task = this.get(id);
    if (task.accepted) throw new Error("已审批任务不能更改历史审批来源");
    task.autoAccept = automatic;
    this.store.put("task", id, task);
    for (const child of this.store.list<Task>("task")) {
      if (
        child.parentTaskId === id &&
        typeof child.workspacePrepared === "boolean" &&
        !child.accepted
      ) {
        child.autoAccept = automatic;
        this.store.put("task", child.id, child);
      }
    }
    this.notify();
    return task;
  }
  async followup(id: string, text: string): Promise<Task> {
    const source = this.get(id);
    if (!["completed", "rolledBack"].includes(source.status))
      throw new Error("当前任务尚未交付，请在原任务中补充");
    return this.create(
      {
        projectId: source.projectId,
        title: "继续创作 · " + source.title.slice(0, 80),
        references: source.references,
        capability: source.capability,
        stopConditions: source.stopConditions,
        maxMinutes: source.maxMinutes,
        prompt: `基于项目当前版本继续任务「${source.title}」。原任务 ID：${source.id}。原目标、汇报与已确认回答保存在 .beaver-context/followup/task.json。阅读相关记录并检查现有文件，保留其他任务成果，不重复已完成的工作。\n用户新要求：${text}`,
        direction: source.direction,
      },
      async (task) => {
        task.parentTaskId = source.id;
        task.relation = "followup";
        const dir = path.join(task.workspace, ".beaver-context/followup");
        await fs.mkdir(dir, { recursive: true });
        await fs.writeFile(
          path.join(dir, "task.json"),
          JSON.stringify(
            {
              id: source.id,
              title: source.title,
              prompt: source.prompt,
              report: source.report,
              clarifications: source.clarifications,
              references: source.references,
            },
            null,
            2,
          ),
          "utf8",
        );
      },
    );
  }
  async delegate(id: string, text: string): Promise<Task> {
    const source = this.get(id);
    return this.create(
      {
        projectId: source.projectId,
        title: "子任务 · " + text.trim().slice(0, 70),
        references: source.references,
        prompt: `完成父任务「${source.title}」委派的独立子目标。父会话记录在 .beaver-context/parent/task.json。当前工作副本来自项目已合入版本，不包含父任务尚未合入的修改；缺少前置成果时询问用户，不假定其存在。不要替父任务宣布完成。\n子目标：${text}`,
        direction: source.direction,
        capability: source.capability,
        stopConditions: source.stopConditions,
        maxMinutes: source.maxMinutes,
      },
      async (task) => {
        task.parentTaskId = source.id;
        task.relation = "child";
        const dir = path.join(task.workspace, ".beaver-context/parent");
        await fs.mkdir(dir, { recursive: true });
        await fs.writeFile(
          path.join(dir, "task.json"),
          JSON.stringify(
            {
              id: source.id,
              title: source.title,
              prompt: source.prompt,
              status: source.status,
              report: source.report,
              references: source.references,
              clarifications: source.clarifications,
              conversation: this.store.events(source.id),
            },
            null,
            2,
          ),
          "utf8",
        );
      },
    );
  }
  async feature(projectId: string, id: string): Promise<Task> {
    if (!/^[a-z0-9-]+$/.test(id)) throw new Error("非法功能块标识");
    if (enginePackageId(id))
      throw new Error("该功能包仅支持规划，尚未提供可接入源码");
    const source = path.join(this.resources, "features", id);
    const manifest = featureSchema.parse(
      JSON.parse(await fs.readFile(path.join(source, "feature.json"), "utf8")),
    );
    if (manifest.id !== id) throw new Error("功能块目录与标识不一致");
    const snapshot = await this.files.capture(source);
    const old = this.store.get<FeatureAdoption>(
      "feature",
      `${projectId}:${id}`,
    );
    return this.create(
      {
        projectId,
        title: `${old ? "更新" : "添加"}功能块 · ${manifest.name}`,
        prompt: `为项目${old ? "更新" : "接入"}功能块 ${manifest.name}。已采用版本：${old?.version ?? "无"}；新版本：${manifest.version}。读取 .beaver-context/feature 中的上游新旧源文件，理解差异后适配到游戏，保留项目定制，验证运行效果。汇报实际集成结果。不要直接覆盖现有定制代码。`,
      },
      async (task) => {
        const dir = path.join(task.workspace, ".beaver-context/feature");
        await this.files.restoreCopy(snapshot, path.join(dir, "new"));
        if (old)
          await this.files.restoreCopy(old.snapshot, path.join(dir, "old"));
        await fs.writeFile(
          path.join(dir, "upstream-changes.json"),
          JSON.stringify(
            {
              from: old?.version ?? null,
              to: manifest.version,
              changes: this.files.changes(old?.snapshot ?? {}, snapshot),
            },
            null,
            2,
          ),
        );
        task.feature = {
          id,
          version: manifest.version,
          snapshot,
          previous: old,
        };
      },
    );
  }
  async shutdown(): Promise<void> {
    this.closing = true;
    await this.executor.shutdown();
    for (const t of this.store.list<Task>("task"))
      if (t.status === "queued" || t.status === "running") {
        t.status = "interrupted";
        this.save(t);
      }
  }
}
import {
  askRatioSchema,
  automaticChoice,
  type AskRatio,
} from "../shared/autonomy";
import { normalizeSources } from "./source-encoding";
