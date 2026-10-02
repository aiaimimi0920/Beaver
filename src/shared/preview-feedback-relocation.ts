import { z } from "zod";

const id = z
  .string()
  .min(1)
  .max(128)
  .regex(/^[A-Za-z0-9_.:-]+$/);
export const previewFrameReferenceSchema = z.strictObject({
  runId: id,
  frameId: id,
});
export type PreviewFrameReference = z.infer<typeof previewFrameReferenceSchema>;
const index = z.number().int().min(0).max(7);
export const feedbackRegionSchema = z.discriminatedUnion("status", [
  z.strictObject({
    status: z.literal("matched"),
    sourceRegion: index,
    targetRegion: index,
  }),
  z.strictObject({
    status: z.literal("absent"),
    sourceRegion: index,
    note: z
      .string()
      .refine(
        (value) =>
          !!value.trim() && new TextEncoder().encode(value).length <= 1000,
      ),
  }),
]);
export const feedbackRelocationSchema = z.strictObject({
  sourceAttemptId: id,
  sourceFrame: previewFrameReferenceSchema,
  regions: z
    .array(feedbackRegionSchema)
    .min(1)
    .max(8)
    .refine((regions) =>
      regions.every((region, index) => region.sourceRegion === index),
    ),
  confirmed: z.literal(true),
});
export type FeedbackRelocation = z.infer<typeof feedbackRelocationSchema>;
export const finalRelocationSchema = feedbackRelocationSchema
  .omit({ sourceAttemptId: true, sourceFrame: true })
  .extend({
    sourceDigest: z.string().regex(/^[a-f0-9]{64}$/),
    targetFrame: previewFrameReferenceSchema,
  });
export type FinalRelocation = z.infer<typeof finalRelocationSchema>;
