import type { PublicationPreview } from "../../src/shared/object-publication";
import type { FinalRelocation } from "../../src/shared/preview-feedback-relocation";
import { execution } from "./object-attempts";
import { feedbackFrame } from "./feedback-relocation";
import type { FrozenFeedbackPng } from "../../src/ui/object-tasks/final-relocation-source";

export function finalRelocationFixture(published = false) {
  const source = feedbackFrame(true);
  const target = feedbackFrame(false);
  if (published) {
    source.source.target = {
      projectId: "p",
      objectId: "hero",
      versionId: "original-version",
      path: "hero.tscn",
      sha256: "a".repeat(64),
    };
  }
  const feedback: PublicationPreview["feedback"][number] = {
    requestId: "feedback",
    attemptId: "old-attempt",
    feedback: "Move the label and remove the badge",
    previewFrame: { runId: source.runId, frameId: source.id },
    relocationRequirement: { sourceDigest: "c".repeat(64), regionCount: 2 },
    ...(published
      ? {
          origin: {
            publicationRequestId: "original-publication",
            versionId: "original-version",
            runId: "original-run",
            publishedFrame: true,
          },
        }
      : {}),
  };
  const preview: PublicationPreview = {
    review: {
      projectId: "p",
      target: execution("awaitingGate").attempt.target,
      reviewRequestId: "review",
    },
    digest: "d".repeat(64),
    outputDigest: "b".repeat(64),
    baselineVersionId: null,
    acceptedVersionId: null,
    objectRevision: 1,
    replacementRequired: false,
    files: [{ path: "hero.tscn", role: "source" }],
    paths: [{ path: "hero.tscn", after: "b".repeat(64) }],
    feedback: [feedback],
  };
  const confirmation: FinalRelocation = {
    sourceDigest: feedback.relocationRequirement!.sourceDigest,
    targetFrame: { runId: target.runId, frameId: target.id },
    regions: [
      { status: "matched", sourceRegion: 0, targetRegion: 0 },
      {
        status: "absent",
        sourceRegion: 1,
        note: "Badge removed from final output",
      },
    ],
    confirmed: true,
  };
  return { source, target, feedback, preview, confirmation };
}

export function publicationStorage() {
  const values = new Map<string, string>();
  return {
    values,
    getItem: (key: string) => values.get(key) ?? null,
    setItem: (key: string, value: string) => {
      values.set(key, value);
    },
  };
}

export function finalRelocationPngFixture(inherited = false) {
  const f = finalRelocationFixture();
  delete f.feedback.previewFrame;
  f.feedback.image = {
    path: "preview.png",
    sha256: "a".repeat(64),
    width: 400,
    height: 200,
    regions: structuredClone(f.source.selection!.regions),
  };
  if (inherited)
    f.feedback.origin = {
      publicationRequestId: "original-publication",
      versionId: "original-version",
      runId: "original-run",
      publishedFrame: false,
    };
  const source: FrozenFeedbackPng = {
    kind: "png",
    request: {
      projectId: f.preview.review.projectId,
      runId: f.feedback.origin?.runId ?? f.preview.review.target.runId,
      attemptId: f.feedback.attemptId,
      checkpoint: "output",
      path: f.feedback.image.path,
      sha256: f.feedback.image.sha256,
    },
    image: structuredClone(f.feedback.image),
    width: f.feedback.image.width,
    height: f.feedback.image.height,
  };
  return { ...f, source };
}
