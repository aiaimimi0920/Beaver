import { z } from "zod";
import { previewSelectionSchema } from "../../shared/preview-selection";
import { frameSchema } from "./live-scene-preview";
import { targetSchema, type SceneTarget } from "./object-scene-preview";

export const savedSchema = z.object({
  id: z.string(),
  projectId: z.string(),
  runId: z.string(),
  snapshotId: z.string(),
  savedAt: z.string(),
  frame: frameSchema,
  selection: previewSelectionSchema.optional(),
  source: z.object({
    target: targetSchema,
    runId: z.string(),
    sourceDigest: z.string(),
    projectConfig: z.string(),
  }),
});
export type SavedPreviewFrame = z.infer<typeof savedSchema>;
export function parseSavedFrames(
  raw: unknown,
  target: SceneTarget,
  runId: string,
  snapshotId: string,
) {
  const result = z
    .object({
      projectId: z.string(),
      runId: z.string(),
      snapshotId: z.string(),
      frames: z.array(savedSchema).max(8),
    })
    .parse(raw);
  if (
    result.projectId !== target.projectId ||
    result.runId !== runId ||
    result.snapshotId !== snapshotId
  )
    throw new Error("PREVIEW_SAVED_IDENTITY_MISMATCH");
  for (const item of result.frames) {
    if (
      item.selection &&
      (!item.frame.frozen ||
        item.selection.sequence !== item.frame.sequence ||
        item.selection.sha256 !== item.frame.sha256)
    )
      throw new Error("PREVIEW_SELECTION_FRAME_MISMATCH");
    const actual = item.source.target;
    if (
      item.projectId !== target.projectId ||
      item.runId !== runId ||
      item.snapshotId !== snapshotId ||
      item.source.runId !== runId ||
      Object.keys(actual).length !== Object.keys(target).length ||
      Object.entries(target).some(
        ([key, value]) => actual[key as keyof typeof actual] !== value,
      )
    )
      throw new Error("PREVIEW_SAVED_SOURCE_MISMATCH");
  }
  return result.frames;
}
