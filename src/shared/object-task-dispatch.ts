import { z } from "zod";

const id = z
  .string()
  .min(1)
  .max(128)
  .regex(/^[A-Za-z0-9_.:-]+$/);
const revision = z.number().int().nonnegative().max(Number.MAX_SAFE_INTEGER);

export const objectTaskDispatchControlSchema = z.strictObject({
  schemaVersion: z.literal(1),
  projectId: id,
  taskId: id,
  objectId: id,
  runId: id,
  paused: z.boolean(),
  revision: revision.positive(),
});

export const setObjectTaskPausedSchema = z.strictObject({
  projectId: id,
  taskId: id,
  objectId: id,
  runId: id,
  requestId: id,
  expectedTaskRevision: revision,
  expectedControlRevision: revision.max(Number.MAX_SAFE_INTEGER - 1),
  paused: z.boolean(),
});

export const objectTaskDispatchReceiptSchema = z
  .strictObject({
    request: setObjectTaskPausedSchema,
    result: objectTaskDispatchControlSchema,
  })
  .refine(
    ({ request, result }) =>
      result.projectId === request.projectId &&
      result.taskId === request.taskId &&
      result.objectId === request.objectId &&
      result.runId === request.runId &&
      result.paused === request.paused &&
      result.revision === request.expectedControlRevision + 1,
    "Dispatch receipt does not match its request",
  );

export type ObjectTaskDispatchControl = z.infer<
  typeof objectTaskDispatchControlSchema
>;
export type SetObjectTaskPausedRequest = z.infer<
  typeof setObjectTaskPausedSchema
>;
export type ObjectTaskDispatchReceipt = z.infer<
  typeof objectTaskDispatchReceiptSchema
>;

export const coarseDispatchControlSchema = objectTaskDispatchControlSchema.omit(
  {
    objectId: true,
    runId: true,
  },
);
export const setCoarsePausedSchema = setObjectTaskPausedSchema.omit({
  objectId: true,
  runId: true,
});
export const coarseDispatchReceiptSchema = z
  .strictObject({
    request: setCoarsePausedSchema,
    result: coarseDispatchControlSchema,
  })
  .refine(
    ({ request, result }) =>
      result.projectId === request.projectId &&
      result.taskId === request.taskId &&
      result.paused === request.paused &&
      result.revision === request.expectedControlRevision + 1,
    "Coarse dispatch receipt does not match its request",
  );
export type CoarseDispatchControl = z.infer<typeof coarseDispatchControlSchema>;
export type SetCoarsePausedRequest = z.infer<typeof setCoarsePausedSchema>;
export type CoarseDispatchReceipt = z.infer<typeof coarseDispatchReceiptSchema>;
