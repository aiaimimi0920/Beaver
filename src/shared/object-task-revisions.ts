import { z } from "zod";
import { objectTaskRevisionImpact } from "./object-task-revision-plan";
import {
  objectTaskRecordSchema,
  objectWorkRequirementSchema,
  type ObjectTaskRecord,
  type ObjectTaskSnapshot,
} from "./object-tasks";

const id = objectTaskRecordSchema.shape.id;
const revision = z.number().int().nonnegative().max(Number.MAX_SAFE_INTEGER);
const encoder = new TextEncoder();
function text(maxBytes: number, required = false) {
  return z
    .string()
    .max(maxBytes)
    .refine(
      (value) =>
        encoder.encode(value).length <= maxBytes &&
        (!required || value.trim().length > 0),
      `内容${required ? "不能为空，且" : ""}不能超过 ${maxBytes} 个 UTF-8 字节`,
    );
}
const uniqueIds = z
  .array(id)
  .refine((ids) => new Set(ids).size === ids.length, "ID 不能重复");

export const objectTaskDefinitionSchema = z.strictObject({
  title: text(300, true),
  prompt: text(20_000, true),
  acceptance: text(10_000),
  requirement: objectWorkRequirementSchema,
  pendingPlanning: text(4_000).default(""),
  dependsOn: uniqueIds.max(500),
});
export const revisePlannedObjectTaskSchema = z.strictObject({
  projectId: id,
  taskId: id,
  requestId: id,
  expectedTaskRevision: revision.max(Number.MAX_SAFE_INTEGER - 1),
  expectedPlanRevision: revision.max(Number.MAX_SAFE_INTEGER - 1),
  definition: objectTaskDefinitionSchema,
  reason: text(2_000, true),
});
export const objectTaskDefinitionRevisionSchema = z
  .strictObject({
    projectId: id,
    taskId: id,
    requestId: id,
    previousTaskRevision: revision,
    taskRevision: revision,
    previousPlanRevision: revision,
    planRevision: revision,
    before: objectTaskDefinitionSchema,
    after: objectTaskDefinitionSchema,
    reason: text(2_000, true),
    adoptedBy: z.literal("owner"),
    createdAt: z.string().datetime({ offset: true }),
    affectedTaskIds: uniqueIds,
  })
  .refine(
    (entry) =>
      entry.taskRevision === entry.previousTaskRevision + 1 &&
      entry.planRevision === entry.previousPlanRevision + 1 &&
      entry.affectedTaskIds.includes(entry.taskId),
    "修订版本或影响范围无效",
  );

export type ObjectTaskDefinition = z.infer<typeof objectTaskDefinitionSchema>;
export type RevisePlannedObjectTaskRequest = z.infer<
  typeof revisePlannedObjectTaskSchema
>;
export type ObjectTaskDefinitionRevision = z.infer<
  typeof objectTaskDefinitionRevisionSchema
>;

export function objectTaskDefinition(
  task: ObjectTaskRecord,
): ObjectTaskDefinition {
  return {
    title: task.title,
    prompt: task.prompt,
    acceptance: task.acceptance,
    requirement: task.requirement,
    pendingPlanning: task.pendingPlanning,
    dependsOn: [...task.dependsOn],
  };
}

export function sameObjectTaskDefinition(
  left: ObjectTaskDefinition,
  right: ObjectTaskDefinition,
): boolean {
  return (
    left.title === right.title &&
    left.prompt === right.prompt &&
    left.acceptance === right.acceptance &&
    left.requirement === right.requirement &&
    left.pendingPlanning === right.pendingPlanning &&
    JSON.stringify(left.dependsOn) === JSON.stringify(right.dependsOn)
  );
}

export function sameObjectTaskIdentity(
  left: ObjectTaskRecord,
  right: ObjectTaskRecord,
): boolean {
  return (
    left.id === right.id &&
    left.projectId === right.projectId &&
    left.granularity === right.granularity &&
    left.objectId === right.objectId &&
    left.parentTaskId === right.parentTaskId &&
    left.runId === right.runId &&
    left.stageId === right.stageId &&
    JSON.stringify(left.identity) === JSON.stringify(right.identity)
  );
}

export function parseObjectTaskRevisionReceipt(
  input: unknown,
  request: RevisePlannedObjectTaskRequest,
  reviewed: ObjectTaskSnapshot,
): ObjectTaskDefinitionRevision {
  const receipt = objectTaskDefinitionRevisionSchema.parse(input);
  const task = reviewed.tasks.find((task) => task.id === request.taskId);
  if (
    !task ||
    receipt.projectId !== request.projectId ||
    receipt.taskId !== request.taskId ||
    receipt.requestId !== request.requestId ||
    receipt.previousTaskRevision !== request.expectedTaskRevision ||
    receipt.previousPlanRevision !== request.expectedPlanRevision ||
    !sameObjectTaskDefinition(receipt.before, objectTaskDefinition(task)) ||
    !sameObjectTaskDefinition(receipt.after, request.definition) ||
    receipt.reason !== request.reason ||
    JSON.stringify(receipt.affectedTaskIds) !==
      JSON.stringify(objectTaskRevisionImpact(reviewed, task.id))
  ) {
    throw new Error("任务修订回执与已确认的请求、原定义或影响范围不一致");
  }
  return receipt;
}

export function parseObjectTaskRevisionHistory(
  input: unknown,
  projectId: string,
  taskId: string,
  known: readonly ObjectTaskDefinitionRevision[] = [],
): ObjectTaskDefinitionRevision[] {
  const entries = z.array(objectTaskDefinitionRevisionSchema).parse(input);
  const requests = new Set<string>();
  for (const [index, entry] of entries.entries()) {
    const previous = entries[index - 1];
    if (
      entry.projectId !== projectId ||
      entry.taskId !== taskId ||
      requests.has(entry.requestId) ||
      (previous &&
        (entry.previousTaskRevision < previous.taskRevision ||
          entry.previousPlanRevision < previous.planRevision ||
          !sameObjectTaskDefinition(entry.before, previous.after)))
    ) {
      throw new Error("任务修订历史的身份、顺序或定义衔接无效");
    }
    requests.add(entry.requestId);
  }
  for (const entry of known) {
    const received = entries.find((item) => item.requestId === entry.requestId);
    if (!received || JSON.stringify(received) !== JSON.stringify(entry)) {
      throw new Error("修订历史缺少或改写了已确认的记录；请重新读取");
    }
  }
  return entries;
}
