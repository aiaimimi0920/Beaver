import { z } from "zod";

const id = z
  .string()
  .min(1)
  .max(128)
  .regex(/^[A-Za-z0-9_.:-]+$/);
const version = z.string().regex(/^[0-9a-f]{64}$/);
export const queueItemSchema = z.strictObject({
  taskId: id,
  objectId: id,
  title: z.string(),
  state: z.enum([
    "queued",
    "claimed",
    "running",
    "awaitingAcceptance",
    "accepted",
    "failed",
    "cancelled",
  ]),
  blockers: z.array(
    z.enum([
      "objectHeld",
      "earlierQueued",
      "paused",
      "coarsePaused",
      "dependencies",
    ]),
  ),
});
export const queueViewSchema = z
  .strictObject({
    projectId: id,
    version,
    items: z.array(queueItemSchema),
  })
  .refine(
    (view) =>
      new Set(view.items.map((item) => item.taskId)).size === view.items.length,
    "Duplicate queue task",
  );
export const queueMoveSchema = z.strictObject({
  projectId: id,
  requestId: id,
  expectedVersion: version,
  taskId: id,
  previousTaskId: id.nullable(),
  nextTaskId: id.nullable(),
});
export const queueReceiptSchema = z
  .strictObject({ request: queueMoveSchema, result: queueViewSchema })
  .refine(({ request, result }) => {
    const ids = result.items
      .filter((item) => item.state === "queued")
      .map((item) => item.taskId);
    const index = ids.indexOf(request.taskId);
    return (
      result.projectId === request.projectId &&
      result.version !== request.expectedVersion &&
      index >= 0 &&
      (ids[index - 1] ?? null) === request.previousTaskId &&
      (ids[index + 1] ?? null) === request.nextTaskId
    );
  }, "Queue receipt does not match move");
export type QueueView = z.infer<typeof queueViewSchema>;
export type QueueItem = z.infer<typeof queueItemSchema>;
export type QueueMove = z.infer<typeof queueMoveSchema>;
export type QueueReceipt = z.infer<typeof queueReceiptSchema>;

export function parseQueueView(input: unknown, projectId: string): QueueView {
  const view = queueViewSchema.parse(input);
  if (view.projectId !== projectId) throw new Error("Queue project mismatch");
  return view;
}

// index is an insertion slot after removing the dragged task.
export function previewQueueMove(
  view: QueueView,
  taskId: string,
  index: number,
) {
  const queued = view.items.filter((item) => item.state === "queued");
  const task = queued.find((item) => item.taskId === taskId);
  if (!task) throw new Error("只能移动待执行任务");
  const result = queued.filter((item) => item.taskId !== taskId);
  if (!Number.isInteger(index) || index < 0 || index > result.length)
    throw new Error("插入位置已失效");
  result.splice(index, 0, task);
  const objects = new Set<string>();
  for (const head of queued) {
    if (objects.has(head.objectId)) continue;
    objects.add(head.objectId);
    if (
      head.blockers.length &&
      result.find((item) => item.objectId === head.objectId)?.taskId !==
        head.taskId
    )
      throw new Error("不能绕过同对象的阻塞队头；请先处理依赖或派发控制");
  }
  return {
    items: result,
    previousTaskId: result[index - 1]?.taskId ?? null,
    nextTaskId: result[index + 1]?.taskId ?? null,
  };
}
