import fs from "node:fs/promises";
import path from "node:path";
import { randomUUID } from "node:crypto";
import type { Task, Project } from "../shared/types";
import { taskPlanSchema } from "../shared/task-plan";
import { Store } from "./store";
import { Files, safePath } from "./files";

export function planEligible(task: Task, tasks: Task[]): boolean {
  if (
    typeof task.workspacePrepared === "boolean" &&
    !tasks.some(
      (t) =>
        t.id === task.parentTaskId &&
        t.status === "waitingChildren" &&
        !t.planPaused,
    )
  )
    return false;
  return (task.dependsOn ?? []).every((id) =>
    tasks.some((t) => t.id === id && t.status === "completed" && t.accepted),
  );
}

export function reconcilePlans(store: Store): void {
  const tasks = store.list<Task>("task");
  for (const parent of tasks.filter(
    (t) => t.status === "waitingChildren" && !t.planPaused,
  )) {
    if (!parent.subtaskIds?.length) {
      const plan = taskPlanSchema.parse(parent.plan);
      const ids = plan.steps.map(() => randomUUID());
      const children: Task[] = plan.steps.map((step, i) => ({
        id: ids[i]!,
        projectId: parent.projectId,
        parentTaskId: parent.id,
        relation: "child",
        title: step.title,
        prompt: `子目标：${step.prompt}\n验收标准：${step.acceptance}\n阅读 .beaver-context/parent/task.json 中的原始目标和已确认回答，以及当前副本内的前置成果。仅完成本子目标并验证，不代替其他子任务宣布完成。`,
        direction: step.direction,
        capability: "code",
        decompose: false,
        autoAccept: parent.autoAccept ?? true,
        askRatio: parent.askRatio,
        projectContext: parent.projectContext,
        design: parent.design,
        references: parent.references,
        stopConditions: parent.stopConditions,
        maxMinutes: parent.maxMinutes,
        status: "queued",
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
        workspace: path.join(store.root, "workspaces", ids[i]!),
        workspacePrepared: false,
        baseline: {},
        changes: [],
        conflicts: [],
        dependsOn: i ? [ids[i - 1]!] : [],
      }));
      parent.subtaskIds = ids;
      store.transaction(() => {
        for (const child of children) store.put("task", child.id, child);
        store.put("task", parent.id, parent);
        store.event(
          parent.id,
          "plan",
          `已创建 ${children.length} 个独立子任务，按依赖顺序执行并审批。`,
        );
      });
    } else if (
      parent.subtaskIds.every((id) =>
        tasks.some(
          (t) => t.id === id && t.status === "completed" && t.accepted,
        ),
      )
    ) {
      parent.status = "completed";
      parent.report = `${parent.plan?.summary ?? ""}\n\n${parent.subtaskIds.length} 个子任务已完成合入并通过审批。请检查各子任务的验证报告；这不替代实际试玩。`;
      parent.updatedAt = new Date().toISOString();
      store.put("task", parent.id, parent);
    }
  }
}

export async function preparePlannedTask(
  store: Store,
  files: Files,
  task: Task,
): Promise<void> {
  if (task.workspacePrepared !== false) return;
  const project = store.get<Project>("project", task.projectId);
  const parent = store.get<Task>("task", task.parentTaskId!);
  if (!project || !parent) throw new Error("项目或父任务不存在");
  const baseline = await files.capture(project.path);
  const workspace = await safePath(store.root, `workspaces/${task.id}`);
  try {
    await fs.access(workspace);
    const captured = await files.capture(workspace);
    if (
      Object.keys(captured).length !== Object.keys(baseline).length ||
      Object.entries(baseline).some(([p, hash]) => captured[p] !== hash)
    )
      throw new Error("未启动的子任务副本已有不同内容，请保留并检查后重试");
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== "ENOENT") throw error;
  }
  await files.restoreCopy(baseline, workspace);
  const dir = await safePath(workspace, ".beaver-context/parent");
  await fs.mkdir(dir, { recursive: true });
  await fs.writeFile(
    path.join(dir, "task.json"),
    JSON.stringify(
      {
        title: parent.title,
        prompt: parent.prompt,
        plan: parent.plan,
        clarifications: parent.clarifications,
      },
      null,
      2,
    ),
    "utf8",
  );
  task.baseline = baseline;
  task.workspace = workspace;
  task.workspacePrepared = true;
}
