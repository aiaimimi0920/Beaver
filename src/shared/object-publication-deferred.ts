import { z } from "zod";
import {
  publicationReviewSchema,
  previewFrameReferenceSchema,
} from "./object-publication";
import { objectReworkImageSchema } from "./object-rework-image";
import { feedbackRelocationSchema } from "./preview-feedback-relocation";

const text = (limit: number) =>
  z
    .string()
    .refine(
      (value) =>
        value.trim().length > 0 &&
        new TextEncoder().encode(value).length <= limit,
    );
export const deferredFeedbackSchema = z
  .strictObject({
    projectId: publicationReviewSchema.shape.projectId,
    requestId: publicationReviewSchema.shape.reviewRequestId,
    review: publicationReviewSchema,
    feedback: text(4000),
    later: z.strictObject({ title: text(300), acceptance: text(10000) }),
    image: objectReworkImageSchema.optional(),
    previewFrame: previewFrameReferenceSchema.optional(),
    relocation: feedbackRelocationSchema.optional(),
  })
  .refine(
    (value) =>
      value.projectId === value.review.projectId &&
      (!value.relocation || !!value.previewFrame) &&
      !(value.image && value.previewFrame),
  );
export type DeferredFeedback = z.infer<typeof deferredFeedbackSchema>;
