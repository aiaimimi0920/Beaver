import { z } from "zod";
import { publicationReviewSchema } from "./object-publication";

const id = publicationReviewSchema.shape.reviewRequestId;
const text = (max: number) =>
  z
    .string()
    .refine(
      (value) =>
        value.trim().length > 0 &&
        new TextEncoder().encode(value).length <= max,
    );
export const publicationFollowupRequestSchema = z.strictObject({
  projectId: id,
  requestId: id,
  publicationRequestId: id,
  versionId: id,
  title: text(300),
  feedback: text(16000),
  acceptance: text(10000),
  previewFrame: z.strictObject({ runId: id, frameId: id }).optional(),
});
export const publicationFollowupReceiptSchema = z.strictObject({
  request: publicationFollowupRequestSchema,
  source: publicationReviewSchema.shape.target,
  mediumTaskId: id,
  fineTaskId: id,
  runId: id,
  planRevision: z.number().int().nonnegative().max(Number.MAX_SAFE_INTEGER),
});
export type PublicationFollowupRequest = z.infer<
  typeof publicationFollowupRequestSchema
>;
export type PublicationFollowupReceipt = z.infer<
  typeof publicationFollowupReceiptSchema
>;
