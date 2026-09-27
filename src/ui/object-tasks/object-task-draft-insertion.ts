import type {
  ObjectTaskPlan,
  ObjectTaskProposal,
} from "../../shared/object-tasks";

export type TaskInsertionAnchor =
  { edge: "start" | "end" } | { taskId: string; side: "before" | "after" };

let nextId = 0;

export function newDraftItemId(prefix: string): string {
  nextId++;
  return `${prefix}-${Date.now().toString(36)}-${nextId.toString(36)}`;
}

export function reindexTasks(
  tasks: ObjectTaskProposal[],
): ObjectTaskProposal[] {
  return tasks.map((task, position) => ({ ...task, position }));
}

export function insertDraftTask(
  plan: ObjectTaskPlan,
  anchor: TaskInsertionAnchor,
  granularity: ObjectTaskProposal["granularity"] = "medium",
): ObjectTaskPlan {
  let index: number;
  if ("edge" in anchor) {
    index = anchor.edge === "start" ? 0 : plan.tasks.length;
  } else {
    index = plan.tasks.findIndex((task) => task.id === anchor.taskId);
    if (index < 0) {
      throw new Error("插入位置对应的任务已不存在，请重新选择位置。");
    }
    if (anchor.side === "after") index++;
  }
  const task: ObjectTaskProposal = {
    id: newDraftItemId("task"),
    position: 0,
    granularity,
    title: "",
    prompt: "",
    acceptance: "",
    requirement: "required",
    pendingPlanning: "",
    objectId: null,
    parentTaskId: null,
    dependsOn: [],
    stageId: null,
  };
  const tasks = [...plan.tasks];
  tasks.splice(index, 0, task);
  return { ...plan, tasks: reindexTasks(tasks) };
}
