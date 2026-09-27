import type { ObjectTaskDefinition } from "./object-task-revisions";
import type { ObjectTaskRecord, ObjectTaskSnapshot } from "./object-tasks";

export function revisionTarget(
  snapshot: ObjectTaskSnapshot,
  taskId: string,
): ObjectTaskRecord {
  const task = snapshot.tasks.find((candidate) => candidate.id === taskId);
  if (!task) throw new Error("当前项目中已找不到该任务");
  return task;
}

function scope(
  snapshot: ObjectTaskSnapshot,
  root: string,
  includeDependents: boolean,
): Set<string> {
  const ids = new Set([root]);
  const pending = [root];
  for (const id of pending) {
    for (const task of snapshot.tasks) {
      if (
        (task.parentTaskId === id ||
          (includeDependents && task.dependsOn.includes(id))) &&
        !ids.has(task.id)
      ) {
        ids.add(task.id);
        pending.push(task.id);
      }
    }
  }
  return ids;
}

export function objectTaskRevisionImpact(
  snapshot: ObjectTaskSnapshot,
  taskId: string,
): string[] {
  const affected = scope(snapshot, taskId, true);
  return snapshot.tasks
    .filter((task) => affected.has(task.id) && task.status !== "cancelled")
    .map((task) => task.id)
    .sort();
}

export function requireObjectTaskRevisionEligibility(
  snapshot: ObjectTaskSnapshot,
  taskId: string,
): ObjectTaskRecord {
  const task = revisionTarget(snapshot, taskId);
  if (task.status !== "planned")
    throw new Error("只能修订尚未执行且未撤销的任务");
  if (
    task.parentTaskId &&
    revisionTarget(snapshot, task.parentTaskId).status === "cancelled"
  ) {
    throw new Error("责任父任务已撤销，不能修订该任务");
  }
  const owned = scope(snapshot, taskId, false);
  for (const candidate of snapshot.tasks) {
    if (!owned.has(candidate.id) || candidate.status === "cancelled") continue;
    if (candidate.status !== "planned")
      throw new Error("责任范围中已有任务开始执行");
    if (candidate.granularity !== "coarse") {
      const run = snapshot.runs.find((run) => run.id === candidate.runId);
      if (!run || run.status !== "planned" || run.baselineVersionId !== null) {
        throw new Error(
          "责任范围中的迭代已开始、已撤销或已有执行基准，不能修订",
        );
      }
    }
  }
  return task;
}

export function validateObjectTaskRevisionPlan(
  snapshot: ObjectTaskSnapshot,
  taskId: string,
  definition: ObjectTaskDefinition,
): void {
  requireObjectTaskRevisionEligibility(snapshot, taskId);
  const tasks = new Map(snapshot.tasks.map((task) => [task.id, task]));
  for (const id of definition.dependsOn) {
    if (id === taskId) throw new Error("任务不能依赖自身");
    const dependency = tasks.get(id);
    if (!dependency || dependency.status === "cancelled")
      throw new Error(`依赖任务 ${id} 已撤销或不存在，请更换依赖`);
  }
  const remaining = new Map<string, number>();
  const dependents = new Map<string, string[]>();
  for (const task of snapshot.tasks) {
    const dependencies =
      task.id === taskId ? definition.dependsOn : task.dependsOn;
    remaining.set(task.id, dependencies.length);
    for (const id of dependencies) {
      if (!tasks.has(id)) throw new Error(`依赖任务 ${id} 不存在`);
      const items = dependents.get(id) ?? [];
      items.push(task.id);
      dependents.set(id, items);
    }
  }
  const ready = [...remaining]
    .filter(([, count]) => count === 0)
    .map(([id]) => id);
  for (const id of ready) {
    for (const dependent of dependents.get(id) ?? []) {
      const count = remaining.get(dependent)! - 1;
      remaining.set(dependent, count);
      if (count === 0) ready.push(dependent);
    }
  }
  if (ready.length !== tasks.size) throw new Error("修订后的依赖存在环路");
}
