import {
  objectTaskSnapshotSchema,
  type ObjectTaskSnapshot,
} from "./object-tasks";

function requireMatch(condition: boolean, message: string): void {
  if (!condition) throw new Error(`对象任务快照无效：${message}`);
}

export function parseObjectTaskSnapshot(
  input: unknown,
  projectId: string,
): ObjectTaskSnapshot {
  const snapshot = objectTaskSnapshotSchema.parse(input);
  const tasks = new Map(snapshot.tasks.map((task) => [task.id, task]));
  const runs = new Map(snapshot.runs.map((run) => [run.id, run]));
  requireMatch(
    tasks.size === snapshot.tasks.length && runs.size === snapshot.runs.length,
    "重复 ID",
  );
  for (const run of snapshot.runs) {
    const medium = tasks.get(run.mediumTaskId);
    requireMatch(run.projectId === projectId, "迭代不属于当前项目");
    requireMatch(
      medium?.granularity === "medium" &&
        medium.objectId === run.objectId &&
        medium.runId === run.id,
      "迭代与中修关联不一致",
    );
    requireMatch(medium?.status === run.status, "迭代与中修状态不一致");
  }
  for (const task of snapshot.tasks) {
    requireMatch(task.projectId === projectId, "任务不属于当前项目");
    requireMatch(
      task.identity.layer === task.granularity,
      "任务层级与身份不一致",
    );
    requireMatch(
      new Set(task.dependsOn).size === task.dependsOn.length &&
        task.dependsOn.every((id) => id !== task.id && tasks.has(id)),
      "依赖重复、缺失或指向自身",
    );
    const parent = task.parentTaskId ? tasks.get(task.parentTaskId) : undefined;
    if (task.granularity === "coarse") {
      requireMatch(
        task.objectId === null &&
          task.parentTaskId === null &&
          task.runId === null &&
          task.stageId === null,
        "粗修不能绑定对象、父任务或迭代",
      );
      continue;
    }
    const run = task.runId ? runs.get(task.runId) : undefined;
    requireMatch(
      !!run && run.objectId === task.objectId,
      "任务与迭代对象不一致",
    );
    if (task.identity.layer === "medium") {
      requireMatch(
        task.objectId === task.identity.objectId &&
          task.stageId === null &&
          run?.mediumTaskId === task.id,
        "中修身份不一致",
      );
      requireMatch(
        task.parentTaskId === null || parent?.granularity === "coarse",
        "中修责任父任务无效",
      );
    } else if (task.identity.layer === "fine") {
      requireMatch(
        parent?.granularity === "medium" &&
          parent.objectId === task.objectId &&
          parent.runId === task.runId,
        "精修责任父任务无效",
      );
      requireMatch(
        task.objectId === task.identity.objectId &&
          task.parentTaskId === task.identity.mediumTaskId &&
          task.runId === task.identity.runId &&
          task.stageId === task.identity.stageId,
        "精修身份不一致",
      );
    }
  }
  const controlledTasks = new Set<string>();
  for (const control of snapshot.dispatchControls) {
    const medium = tasks.get(control.taskId);
    requireMatch(control.projectId === projectId, "派发控制不属于当前项目");
    requireMatch(!controlledTasks.has(control.taskId), "派发控制重复");
    requireMatch(
      medium?.granularity === "medium" &&
        medium.objectId === control.objectId &&
        medium.runId === control.runId,
      "派发控制与中修关联不一致",
    );
    controlledTasks.add(control.taskId);
  }
  const coarseControls = new Set<string>();
  for (const control of snapshot.coarseDispatchControls) {
    requireMatch(control.projectId === projectId, "粗修派发控制不属于当前项目");
    requireMatch(!coarseControls.has(control.taskId), "粗修派发控制重复");
    requireMatch(
      tasks.get(control.taskId)?.granularity === "coarse",
      "派发控制与粗修关联不一致",
    );
    coarseControls.add(control.taskId);
  }
  return snapshot;
}
