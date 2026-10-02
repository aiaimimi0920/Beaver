import type { PublicationPreview } from "../../shared/object-publication";
import {
  objectReworkImageSchema,
  type ObjectReworkImage,
} from "../../shared/object-rework-image";
import type { SavedPreviewFrame } from "../object-preview/saved-preview-frames";

export interface FrozenFeedbackPng {
  kind: "png";
  request: {
    projectId: string;
    runId: string;
    attemptId: string;
    checkpoint: "input" | "output";
    path: string;
    sha256: string;
  };
  image: ObjectReworkImage;
  width: number;
  height: number;
}
export type FinalRelocationSource = SavedPreviewFrame | FrozenFeedbackPng;

export function numbered(frame: SavedPreviewFrame) {
  return (
    frame.frame.frozen &&
    frame.source.runId === frame.runId &&
    !!frame.selection &&
    frame.selection.sequence === frame.frame.sequence &&
    frame.selection.sha256 === frame.frame.sha256
  );
}

export function matchesFinalRelocationSource(
  preview: PublicationPreview,
  feedback: PublicationPreview["feedback"][number],
  source: FinalRelocationSource,
) {
  const required = feedback.relocationRequirement;
  const review = preview.review;
  if (!required) return false;
  if ("kind" in source) {
    const saved = objectReworkImageSchema.safeParse(feedback.image);
    const loaded = objectReworkImageSchema.safeParse(source.image);
    return (
      !feedback.previewFrame &&
      !feedback.origin?.publishedFrame &&
      saved.success &&
      loaded.success &&
      JSON.stringify(saved.data) === JSON.stringify(loaded.data) &&
      source.request.projectId === review.projectId &&
      source.request.runId ===
        (feedback.origin?.runId ?? review.target.runId) &&
      source.request.attemptId === feedback.attemptId &&
      source.request.checkpoint === "output" &&
      source.request.path === saved.data.path &&
      source.request.sha256 === saved.data.sha256 &&
      source.width === saved.data.width &&
      source.height === saved.data.height &&
      saved.data.regions.length === required.regionCount
    );
  }
  const binding = source.source.target;
  if (
    feedback.image ||
    source.id !== feedback.previewFrame?.frameId ||
    source.runId !== feedback.previewFrame?.runId ||
    source.projectId !== review.projectId ||
    binding.projectId !== review.projectId ||
    !numbered(source) ||
    source.selection!.regions.length !== required.regionCount
  )
    return false;
  return feedback.origin?.publishedFrame
    ? "versionId" in binding &&
        binding.objectId === review.target.objectId &&
        binding.versionId === feedback.origin.versionId
    : "attemptId" in binding &&
        binding.attemptId === feedback.attemptId &&
        binding.runId === (feedback.origin?.runId ?? review.target.runId) &&
        binding.checkpoint === "output";
}
