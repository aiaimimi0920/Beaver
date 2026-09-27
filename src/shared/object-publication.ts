import { z } from "zod";
import { objectReworkImageSchema } from "./object-rework-image";
import { objectCandidateRequestSchema } from "./object-candidate-review";
import {
  previewFrameReferenceSchema,
  feedbackRelocationSchema,
} from "./preview-feedback-relocation";
export {
  previewFrameReferenceSchema,
  type PreviewFrameReference,
} from "./preview-feedback-relocation";

const id = objectCandidateRequestSchema.shape.requestId;
const digest = z.string().regex(/^[a-f0-9]{64}$/);
const revision = z.number().int().nonnegative().max(Number.MAX_SAFE_INTEGER);
const note = z
  .string()
  .min(1)
  .refine(
    (value) =>
      value.trim().length > 0 && new TextEncoder().encode(value).length <= 4000,
  );
export const publicationReviewSchema = z.strictObject({
  projectId: id,
  target: objectCandidateRequestSchema.shape.target,
  reviewRequestId: id,
});
export const publicationDecisionSchema = z.strictObject({
  requestId: id,
  resolution: z.enum(["resolved", "waived", "deferred"]),
  note,
});
export const publicationPreviewSchema = z.strictObject({
  review: publicationReviewSchema,
  digest,
  outputDigest: digest,
  baselineVersionId: id.nullable(),
  acceptedVersionId: id.nullable(),
  objectRevision: revision,
  replacementRequired: z.boolean(),
  files: z.array(
    z.strictObject({ path: z.string().min(1), role: z.string().min(1) }),
  ),
  paths: z.array(
    z.strictObject({
      path: z.string().min(1),
      before: digest.optional(),
      after: digest.optional(),
    }),
  ),
  feedback: z.array(
    z
      .strictObject({
        requestId: id,
        attemptId: id,
        feedback: z.string(),
        image: objectReworkImageSchema.optional(),
        previewFrame: previewFrameReferenceSchema.optional(),
        relocation: feedbackRelocationSchema.optional(),
        later: z
          .strictObject({ title: z.string(), acceptance: z.string() })
          .optional(),
      })
      .refine(
        (value) =>
          !(value.image && value.previewFrame) &&
          (!value.relocation || !!value.previewFrame),
      ),
  ),
});
export const publicationRequestSchema = publicationReviewSchema.extend({
  requestId: id,
  previewDigest: digest,
  acceptanceNote: note,
  confirmFiles: z.literal(true),
  confirmReplacement: z.boolean(),
  feedback: z.array(publicationDecisionSchema),
});
export const publicationOperationSchema = z
  .strictObject({
    schemaVersion: z.literal(1),
    request: publicationRequestSchema,
    preview: publicationPreviewSchema,
    versionId: id,
    state: z.enum(["applying", "aborting", "published", "aborted"]),
    error: z.string().nullable(),
    result: z
      .strictObject({
        versionId: id,
        objectRevision: revision,
        taskRevision: revision,
        runRevision: revision,
        planRevision: revision,
      })
      .nullable(),
  })
  .refine(
    (op) =>
      (op.state === "published") === (op.result !== null) &&
      (!op.result || op.result.versionId === op.versionId) &&
      op.request.previewDigest === op.preview.digest &&
      op.request.projectId === op.preview.review.projectId &&
      op.request.reviewRequestId === op.preview.review.reviewRequestId &&
      JSON.stringify(op.request.target) ===
        JSON.stringify(op.preview.review.target),
  );
export type PublicationReview = z.infer<typeof publicationReviewSchema>;
export type PublicationPreview = z.infer<typeof publicationPreviewSchema>;
export type PublicationRequest = z.infer<typeof publicationRequestSchema>;
export type PublicationOperation = z.infer<typeof publicationOperationSchema>;
export type PublicationDecision = z.infer<typeof publicationDecisionSchema>;
