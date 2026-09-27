import { z } from "zod";
import {
  feedbackRelocationSchema,
  previewFrameReferenceSchema,
} from "../../shared/preview-feedback-relocation";
import type { SavedPreviewFrame } from "../object-preview/saved-preview-frames";

const index = z.number().int().min(0).max(7);
export const relocationDraftSchema = z.strictObject({
  sourceAttemptId: z.string().max(128),
  sourceFrame: previewFrameReferenceSchema.optional(),
  regions: z
    .array(
      z.discriminatedUnion("status", [
        z.strictObject({ status: z.literal("pending"), sourceRegion: index }),
        z.strictObject({
          status: z.literal("matched"),
          sourceRegion: index,
          targetRegion: index,
        }),
        z.strictObject({
          status: z.literal("absent"),
          sourceRegion: index,
          note: z.string(),
        }),
      ]),
    )
    .max(8),
  confirmed: z.boolean(),
});
export type RelocationDraft = z.infer<typeof relocationDraftSchema>;

export function confirmedRelocation(
  draft: RelocationDraft | undefined,
  source: SavedPreviewFrame | undefined,
  target: SavedPreviewFrame | undefined,
) {
  if (!draft || !source || !target) return undefined;
  const parsed = feedbackRelocationSchema.safeParse(draft);
  if (!parsed.success) return undefined;
  const a = source.source.target;
  const b = target.source.target;
  if (
    !("attemptId" in a) ||
    !("attemptId" in b) ||
    a.attemptId !== draft.sourceAttemptId ||
    a.attemptId === b.attemptId ||
    a.projectId !== b.projectId ||
    a.runId !== b.runId ||
    a.checkpoint !== "output" ||
    b.checkpoint !== "output" ||
    source.id !== draft.sourceFrame?.frameId ||
    source.runId !== draft.sourceFrame.runId ||
    draft.regions.length !== source.selection?.regions.length ||
    !target.selection ||
    draft.regions.some(
      (region) =>
        region.status === "matched" &&
        region.targetRegion >= target.selection!.regions.length,
    )
  )
    return undefined;
  return parsed.data;
}
