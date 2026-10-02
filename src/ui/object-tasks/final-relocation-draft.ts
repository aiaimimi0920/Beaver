import { z } from "zod";
import {
  finalRelocationSchema,
  previewFrameReferenceSchema,
  type FinalRelocation,
} from "../../shared/preview-feedback-relocation";
import type {
  PublicationDecision,
  PublicationPreview,
} from "../../shared/object-publication";
import type { SavedPreviewFrame } from "../object-preview/saved-preview-frames";
import { relocationDraftSchema } from "./feedback-relocation-draft";
import {
  matchesFinalRelocationSource,
  numbered,
  type FinalRelocationSource,
} from "./final-relocation-source";

export const finalRelocationDraftSchema = z.strictObject({
  sourceDigest: finalRelocationSchema.shape.sourceDigest,
  targetFrame: previewFrameReferenceSchema.optional(),
  regions: relocationDraftSchema.shape.regions,
  confirmed: z.boolean(),
});
export type FinalRelocationDraft = z.infer<typeof finalRelocationDraftSchema>;
type Feedback = PublicationPreview["feedback"][number];

export function matchesFinalRelocationReceipt(
  draft: FinalRelocationDraft | undefined,
  receipt: FinalRelocation | undefined,
) {
  const current = finalRelocationSchema.safeParse(draft);
  const confirmed = finalRelocationSchema.safeParse(receipt);
  return (
    current.success &&
    confirmed.success &&
    JSON.stringify(current.data) === JSON.stringify(confirmed.data)
  );
}

export function updateFinalRelocation(
  previous: FinalRelocationDraft | undefined,
  next: FinalRelocationDraft | undefined,
) {
  if (!next) return undefined;
  if (
    previous &&
    (previous.sourceDigest !== next.sourceDigest ||
      JSON.stringify(previous.targetFrame) !== JSON.stringify(next.targetFrame))
  )
    return { ...next, regions: [], confirmed: false };
  return next;
}

export function confirmedFinalRelocation(
  preview: PublicationPreview,
  feedback: Feedback,
  draft: FinalRelocationDraft | undefined,
  source: FinalRelocationSource | undefined,
  target: SavedPreviewFrame | undefined,
) {
  if (!draft || !source || !target || !feedback.relocationRequirement)
    return undefined;
  const parsed = finalRelocationSchema.safeParse(draft);
  if (!parsed.success) return undefined;
  const required = feedback.relocationRequirement;
  const b = target.source.target;
  const review = preview.review;
  if (
    draft.sourceDigest !== required.sourceDigest ||
    !matchesFinalRelocationSource(preview, feedback, source) ||
    draft.regions.length !== required.regionCount ||
    target.id !== draft.targetFrame?.frameId ||
    target.runId !== draft.targetFrame.runId ||
    target.projectId !== review.projectId ||
    !numbered(target) ||
    !("attemptId" in b) ||
    b.projectId !== review.projectId ||
    b.runId !== review.target.runId ||
    b.attemptId !== review.target.attemptId ||
    b.checkpoint !== "output" ||
    target.source.sourceDigest !== preview.outputDigest ||
    !preview.paths.some(
      (path) => path.path === b.path && path.after === b.sha256,
    ) ||
    draft.regions.some(
      (region) =>
        region.status === "matched" &&
        region.targetRegion >= target.selection!.regions.length,
    )
  )
    return undefined;
  return parsed.data;
}

export function validateFinalRelocations(
  preview: PublicationPreview,
  decisions: PublicationDecision[],
) {
  for (const feedback of preview.feedback) {
    const confirmation = decisions.find(
      (decision) => decision.requestId === feedback.requestId,
    )?.finalRelocation;
    const required = feedback.relocationRequirement;
    if (!required && !confirmation) continue;
    const parsed = finalRelocationSchema.safeParse(confirmation);
    if (
      !required ||
      !parsed.success ||
      parsed.data.sourceDigest !== required.sourceDigest ||
      parsed.data.regions.length !== required.regionCount
    )
      throw new Error("请逐区核对历史反馈与最终候选编号帧，并明确确认后再发布");
  }
}
