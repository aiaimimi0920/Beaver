import { z } from "zod";
import { planningStateSchema } from "./object-task-planning-declaration";
import {
  objectTaskDispatchControlSchema,
  coarseDispatchControlSchema,
} from "./object-task-dispatch";
import {
  objectBaselineSchema,
  objectTaskIdentitySchema,
} from "./object-framework";

const id = z
  .string()
  .min(1)
  .max(128)
  .regex(/^[A-Za-z0-9_.:-]+$/);
const revision = z.number().int().nonnegative();
export const objectWorkRequirementSchema = z
  .enum(["required", "optional"])
  .default("required");
export const objectTaskAssumptionSchema = z.strictObject({
  id,
  statement: z
    .string()
    .min(1)
    .max(2_000)
    .refine((value) => value.trim().length > 0),
  basis: z
    .string()
    .min(1)
    .max(4_000)
    .refine((value) => value.trim().length > 0),
  source: z.enum(["user", "codex", "automatic"]).default("user"),
  sourceDetail: z.string().max(1_000).nullable().default(null),
});
export const objectTaskAssumptionRecordSchema = z.strictObject({
  ...objectTaskAssumptionSchema.shape,
  planRevision: revision,
});
const taskFields = {
  id,
  position: z.number().int().nonnegative().max(1_000_000_000).optional(),
  granularity: z.enum(["coarse", "medium", "fine"]),
  title: z.string().min(1).max(300),
  prompt: z.string().min(1).max(20_000),
  acceptance: z.string().max(10_000),
  requirement: objectWorkRequirementSchema,
  pendingPlanning: z
    .string()
    .max(4_000)
    .refine((value) => new TextEncoder().encode(value).length <= 4_000)
    .default(""),
};

export const objectTaskProposalSchema = z
  .strictObject({
    ...taskFields,
    objectId: id.nullable().default(null),
    parentTaskId: id.nullable().default(null),
    dependsOn: z.array(id).max(500).default([]),
    stageId: id.nullable().default(null),
    baseline: objectBaselineSchema.optional(),
  })
  .refine(
    (task) => task.granularity === "medium" || task.baseline === undefined,
    {
      path: ["baseline"],
      message: "Only medium tasks select a baseline",
    },
  )
  .refine((task) => task.granularity !== "fine" || !task.pendingPlanning, {
    path: ["pendingPlanning"],
    message: "Only parent tasks track pending planning",
  });
export const objectTaskPlanSchema = z.strictObject({
  objects: z
    .array(
      z.strictObject({
        id,
        name: z.string().min(1).max(256),
        category: z.string().min(1).max(128).default("其他"),
      }),
    )
    .max(128)
    .default([]),
  tasks: z.array(objectTaskProposalSchema).max(500).default([]),
  assumptions: z.array(objectTaskAssumptionSchema).max(500).default([]),
});
export const saveObjectTaskDraftSchema = z.strictObject({
  projectId: id,
  draftId: id,
  expectedRevision: revision,
  expectedPlanRevision: revision,
  plan: objectTaskPlanSchema,
});
export const objectTaskDraftSchema = z.strictObject({
  projectId: id,
  id,
  revision,
  planRevision: revision,
  plan: objectTaskPlanSchema,
  committedRequestId: id.optional(),
});
export const unlockObjectTaskDraftSchema = saveObjectTaskDraftSchema
  .omit({ plan: true })
  .extend({ requestId: id });
export const commitObjectTaskSchema = z.strictObject({
  projectId: id,
  requestId: id,
  draftId: id,
  expectedDraftRevision: revision,
  expectedPlanRevision: revision,
});
export const cancelPlannedObjectTaskSchema = z.strictObject({
  projectId: id,
  taskId: id,
  requestId: id,
  expectedTaskRevision: revision,
  expectedPlanRevision: revision,
});
export const objectTaskCancellationReceiptSchema = z.strictObject({
  projectId: id,
  taskId: id,
  requestId: id,
  previousTaskRevision: revision,
  taskRevision: revision,
  previousPlanRevision: revision,
  planRevision: revision,
});
export const objectTaskStatusSchema = z.enum([
  "planned",
  "queued",
  "running",
  "awaitingAcceptance",
  "accepted",
  "failed",
  "cancelled",
]);
export const objectTaskRecordSchema = z.strictObject({
  ...taskFields,
  projectId: id,
  objectId: id.nullable(),
  parentTaskId: id.nullable(),
  dependsOn: z.array(id),
  runId: id.nullable(),
  stageId: id.nullable(),
  identity: objectTaskIdentitySchema,
  status: objectTaskStatusSchema,
  revision,
});
export const objectRunSchema = z.strictObject({
  id,
  projectId: id,
  objectId: id,
  mediumTaskId: id,
  baselineVersionId: id.nullable(),
  status: objectTaskStatusSchema,
  revision,
});
export const objectTaskCommitReceiptSchema = z.strictObject({
  projectId: id,
  requestId: id,
  draftId: id,
  draftRevision: revision,
  previousPlanRevision: revision,
  planRevision: revision,
  objectIds: z.array(id),
  taskIds: z.array(id),
  runs: z.array(objectRunSchema),
});
export const objectTaskSnapshotSchema = z.strictObject({
  planningStates: z.array(planningStateSchema).optional(),
  planRevision: revision,
  tasks: z.array(objectTaskRecordSchema),
  runs: z.array(objectRunSchema),
  assumptions: z.array(objectTaskAssumptionRecordSchema).default([]),
  dispatchControls: z.array(objectTaskDispatchControlSchema),
  coarseDispatchControls: z.array(coarseDispatchControlSchema),
});

export type ObjectTaskAssumption = z.infer<typeof objectTaskAssumptionSchema>;
export type ObjectTaskAssumptionRecord = z.infer<
  typeof objectTaskAssumptionRecordSchema
>;
export type ObjectTaskProposal = z.infer<typeof objectTaskProposalSchema>;
export type ObjectTaskPlan = z.infer<typeof objectTaskPlanSchema>;
export type SaveObjectTaskDraftRequest = z.infer<
  typeof saveObjectTaskDraftSchema
>;
export type ObjectTaskDraft = z.infer<typeof objectTaskDraftSchema>;
export type CommitObjectTaskRequest = z.infer<typeof commitObjectTaskSchema>;
export type CancelPlannedObjectTaskRequest = z.infer<
  typeof cancelPlannedObjectTaskSchema
>;
export type ObjectTaskCancellationReceipt = z.infer<
  typeof objectTaskCancellationReceiptSchema
>;
export type ObjectTaskRecord = z.infer<typeof objectTaskRecordSchema>;
export type ObjectRun = z.infer<typeof objectRunSchema>;
export type ObjectTaskCommitReceipt = z.infer<
  typeof objectTaskCommitReceiptSchema
>;
export type ObjectTaskSnapshot = z.infer<typeof objectTaskSnapshotSchema>;
