import { z } from "zod";

const id = z
  .string()
  .min(1)
  .max(128)
  .regex(/^[A-Za-z0-9_.:-]+$/);
const revision = z.number().int().nonnegative();
const text = z.string().min(1).max(128);

export const objectAttemptTargetSchema = z.strictObject({
  taskId: id,
  fineTaskId: id,
  objectId: id,
  runId: id,
  attemptId: id,
  owner: text.refine((value) => value.trim().length > 0),
  claimToken: text,
  generation: z.number().int().positive(),
  threadId: text.nullable(),
  turnId: text.nullable(),
});

export const objectAttemptViewSchema = z
  .strictObject({
    projectId: id,
    target: objectAttemptTargetSchema,
    taskRevision: revision,
    state: z.enum(["running", "awaitingGate", "failed", "interrupted"]),
    outputCaptured: z.boolean(),
    error: z.string().nullable(),
  })
  .refine((view) => view.outputCaptured === (view.state !== "running"), {
    message: "执行状态与输出保存状态不一致",
  });

export const objectExecutionSchema = z
  .strictObject({
    attempt: objectAttemptViewSchema,
    availability: z.enum(["active", "recoveryRequired", "finished"]),
    definition: z.strictObject({
      title: z.string().min(1).max(300),
      prompt: z.string().min(1).max(20_000),
      acceptance: z.string().max(10_000),
      revision,
    }),
    checkpoints: z.strictObject({
      input: z.record(z.string(), z.string().regex(/^[a-f0-9]{64}$/)),
      output: z
        .record(z.string(), z.string().regex(/^[a-f0-9]{64}$/))
        .nullable(),
    }),
  })
  .refine(
    (execution) =>
      (execution.availability === "finished") ===
      (execution.attempt.state !== "running"),
    { message: "执行状态与活动状态不一致" },
  )
  .refine(
    ({ attempt, checkpoints }) =>
      attempt.outputCaptured === (checkpoints.output !== null),
    { message: "执行状态与输出清单不一致" },
  );

export const objectAttemptInterruptSchema = z.strictObject({
  projectId: id,
  requestId: id,
  target: objectAttemptTargetSchema,
  expectedTaskRevision: revision,
});

export const objectAttemptInterruptReceiptSchema = z
  .strictObject({
    request: objectAttemptInterruptSchema,
    result: objectAttemptViewSchema,
  })
  .refine(
    ({ request, result }) =>
      result.projectId === request.projectId &&
      JSON.stringify(result.target) === JSON.stringify(request.target) &&
      result.state !== "running" &&
      result.outputCaptured,
    { message: "中断回执与执行目标不一致" },
  );

export type ObjectExecution = z.infer<typeof objectExecutionSchema>;
export type ObjectAttemptInterrupt = z.infer<
  typeof objectAttemptInterruptSchema
>;
export type ObjectAttemptInterruptReceipt = z.infer<
  typeof objectAttemptInterruptReceiptSchema
>;
