import { z } from "zod";
import { objectAttemptTargetSchema } from "./object-attempts";

const id = z
  .string()
  .min(1)
  .max(128)
  .regex(/^[A-Za-z0-9_.:-]+$/);
const digest = z.string().regex(/^[a-f0-9]{64}$/);
export const objectAttemptCheckRequestSchema = z.strictObject({
  projectId: id,
  requestId: id,
  target: objectAttemptTargetSchema,
});
const rule = z
  .strictObject({
    id: z.enum(["checkpoint-integrity", "code-structure"]),
    version: z.literal(1),
    passed: z.boolean(),
    issues: z.array(z.string()),
    filesChecked: z.number().int().nonnegative(),
  })
  .refine((value) => !value.passed || value.issues.length === 0);
export const objectAttemptCheckReportSchema = z
  .strictObject({
    request: objectAttemptCheckRequestSchema,
    runnerVersion: z.literal(1),
    attemptDigest: digest,
    inputDigest: digest,
    outputDigest: digest,
    time: z.string().min(1),
    passed: z.boolean(),
    rules: z.tuple([rule, rule]),
  })
  .refine(
    (report) =>
      report.rules[0].id === "checkpoint-integrity" &&
      report.rules[1].id === "code-structure" &&
      report.passed === report.rules.every((value) => value.passed),
  );
export type ObjectAttemptCheckRequest = z.infer<
  typeof objectAttemptCheckRequestSchema
>;
export type ObjectAttemptCheckReport = z.infer<
  typeof objectAttemptCheckReportSchema
>;
