import type {
  ObjectTaskSnapshot,
  ObjectTaskRecord,
} from "../../shared/object-tasks";
import type { DemoTask } from "./mock-tasks";
import type { DemoStatus } from "./mock-objects";

function statusFor(task: ObjectTaskRecord): DemoStatus {
  switch (task.status) {
    case "planned":
    case "queued":
      return "排队中";
    case "running":
      return "执行中";
    case "awaitingAcceptance":
      return "待验收";
    case "accepted":
      return "已验收";
    case "failed":
    case "cancelled":
      return "执行失败";
  }
}

function levelFor(task: ObjectTaskRecord): DemoTask["level"] {
  if (task.granularity === "coarse") return "粗修";
  if (task.granularity === "medium") return "中修";
  return "精修";
}

/** Maps persisted task identity without inventing execution progress. */
export function previewTasksFromSnapshot(
  snapshot: ObjectTaskSnapshot,
): DemoTask[] {
  return [...snapshot.tasks]
    .sort(
      (left, right) =>
        (left.position ?? 0) - (right.position ?? 0) ||
        left.id.localeCompare(right.id),
    )
    .map((task) => ({
      id: task.id,
      title: task.title,
      summary: task.title,
      level: levelFor(task),
      status: statusFor(task),
      parentId: task.parentTaskId,
      objectId: task.objectId,
      detail:
        task.status === "cancelled"
          ? `任务已取消。${task.acceptance}`
          : task.prompt,
      prompt: task.prompt,
      progress: 0,
    }));
}

/**
 * Returns the persisted medium tasks that represent an object's iteration
 * stack.  Fine tasks remain children of the selected medium task and are
 * consumed by the manufacture view, so they must not become duplicate stack
 * entries.
 */
export function previewIterationsFromSnapshot(
  snapshot: ObjectTaskSnapshot,
  objectId: string,
): DemoTask[] {
  return previewTasksFromSnapshot(snapshot).filter(
    (task) => task.objectId === objectId && task.level === "中修",
  );
}
