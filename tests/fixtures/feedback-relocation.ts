import type { FeedbackRelocation } from "../../src/shared/preview-feedback-relocation";
import { savedSchema } from "../../src/ui/object-preview/saved-preview-frames";

export const relocation: FeedbackRelocation = {
  sourceAttemptId: "old-attempt",
  sourceFrame: { runId: "old-preview", frameId: "old-frame" },
  regions: [
    { status: "matched", sourceRegion: 0, targetRegion: 0 },
    { status: "absent", sourceRegion: 1, note: "Badge removed" },
  ],
  confirmed: true,
};

export function feedbackFrame(
  historical: boolean,
  dataUrl = "data:image/png;base64,AA==",
) {
  const runId = historical ? "old-preview" : "preview-run";
  const sha256 = (historical ? "a" : "b").repeat(64);
  const regions = [
    { x: 0.1, y: 0.2, width: 0.3, height: 0.4, prompt: "Move label" },
  ];
  if (historical)
    regions.push({
      x: 0.7,
      y: 0.1,
      width: 0.2,
      height: 0.2,
      prompt: "Remove badge",
    });
  return savedSchema.parse({
    id: historical ? "old-frame" : "saved-frame",
    projectId: "p",
    runId,
    snapshotId: historical ? "old-snapshot" : "snapshot",
    savedAt: "today",
    source: {
      runId,
      sourceDigest: sha256,
      projectConfig: "project.godot",
      target: {
        projectId: "p",
        runId: "run-hero",
        attemptId: historical ? "old-attempt" : "attempt-1",
        checkpoint: "output",
        path: "hero.tscn",
        sha256,
      },
    },
    frame: {
      sessionId: "s",
      sequence: 2,
      revision: 1,
      frozen: true,
      width: 400,
      height: 200,
      sha256,
      dataUrl,
      engine: "4.4",
      camera: {
        transform: Array.from({ length: 4 }, () => [0, 0, 0]),
        projection: Array.from({ length: 4 }, () => [0, 0, 0, 0]),
        near: 0.1,
        far: 100,
        mode: 0,
      },
    },
    selection: {
      kind: "image-regions",
      sequence: 2,
      sha256,
      regions,
      prompt: "Keep the rest",
      coordinateSpace: "normalized-image",
      hitCapability: "unavailable",
    },
  });
}
