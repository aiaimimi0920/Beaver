import { z } from "zod";
import { objectAttemptTargetSchema } from "./object-attempts";
import { objectAttemptCheckReportSchema } from "./object-attempt-checks";

const id = z
  .string()
  .min(1)
  .max(128)
  .regex(/^[A-Za-z0-9_.:-]+$/);
const digest = z.string().regex(/^[a-f0-9]{64}$/);
export const validationObjectReportSchema = z
  .strictObject({
    source: z.strictObject({
      kind: z.literal("objectAttempt"),
      projectId: id,
      target: objectAttemptTargetSchema,
      stageId: id,
      candidateReviews: z.array(
        z.strictObject({
          requestId: id,
          sourceDigest: digest,
          outputDigest: digest,
        }),
      ),
    }),
    report: objectAttemptCheckReportSchema,
  })
  .refine(
    ({ source, report }) =>
      source.projectId === report.request.projectId &&
      JSON.stringify(source.target) === JSON.stringify(report.request.target) &&
      new Set(source.candidateReviews.map((value) => value.requestId)).size ===
        source.candidateReviews.length &&
      source.candidateReviews.every(
        (value) => value.outputDigest === report.outputDigest,
      ),
  );
export type ValidationObjectSource = z.infer<
  typeof validationObjectReportSchema
>["source"];
