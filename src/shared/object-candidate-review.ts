import { z } from "zod";
import {
  objectAttemptCheckRequestSchema,
  objectAttemptCheckReportSchema,
} from "./object-attempt-checks";

const id = objectAttemptCheckRequestSchema.shape.requestId;
const digest = z.string().regex(/^[a-f0-9]{64}$/);
const revision = z.number().int().nonnegative().max(Number.MAX_SAFE_INTEGER);
export const objectCandidateRequestSchema =
  objectAttemptCheckRequestSchema.extend({
    checkRequestId: id,
  });
export const objectCandidateReportSchema = z
  .strictObject({
    schemaVersion: z.union([z.literal(1), z.literal(2)]),
    request: objectCandidateRequestSchema,
    time: z.string().min(1),
    sourceDigest: digest,
    outputDigest: digest,
    baselineVersionId: id.nullable(),
    acceptedVersionIdAtClaim: id.nullable(),
    acceptedVersionIdAtReview: id.nullable(),
    objectRevisionAtClaim: revision,
    objectRevisionAtReview: revision,
    stages: z
      .array(
        z.strictObject({
          taskId: id,
          title: z.string(),
          revision,
          status: z.enum(["accepted", "awaitingAcceptance"]),
          acceptance: z.string(),
        }),
      )
      .min(1),
    references: z.array(z.strictObject({ objectId: id, versionId: id })),
    files: z.array(
      z
        .strictObject({
          path: z.string().min(1),
          before: digest.nullable(),
          after: digest.nullable(),
          owners: z.array(id),
          reference: z.boolean(),
        })
        .refine((file) => file.before !== null || file.after !== null),
    ),
    rules: objectAttemptCheckReportSchema.shape.rules,
    blockers: z.array(z.string().min(1)),
  })
  .refine(
    (report) =>
      report.stages.at(-1)?.taskId === report.request.target.fineTaskId &&
      report.stages.at(-1)?.status === "awaitingAcceptance" &&
      report.stages
        .slice(0, -1)
        .every((stage) => stage.status === "accepted") &&
      new Set(report.stages.map((stage) => stage.taskId)).size ===
        report.stages.length &&
      new Set(report.files.map((file) => file.path)).size ===
        report.files.length &&
      report.rules[0].id === "checkpoint-integrity" &&
      report.rules[1].id === "code-structure" &&
      report.blockers.includes("FINAL_FINE_OWNER_ACCEPTANCE_REQUIRED") &&
      (report.schemaVersion === 1
        ? report.blockers.includes("PUBLICATION_NOT_IMPLEMENTED") &&
          report.blockers.includes("FEEDBACK_REVIEW_UNAVAILABLE") &&
          !report.blockers.includes("PUBLICATION_CONFIRMATION_REQUIRED")
        : report.blockers.includes("PUBLICATION_CONFIRMATION_REQUIRED") &&
          !report.blockers.includes("PUBLICATION_NOT_IMPLEMENTED") &&
          !report.blockers.includes("FEEDBACK_REVIEW_UNAVAILABLE")),
  );
export type ObjectCandidateRequest = z.infer<
  typeof objectCandidateRequestSchema
>;
export type ObjectCandidateReport = z.infer<typeof objectCandidateReportSchema>;
