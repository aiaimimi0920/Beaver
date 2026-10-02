import { useCallback, useEffect, useState } from "react";
import type { PublicationPreview } from "../../shared/object-publication";
import {
  savedSchema,
  type SavedPreviewFrame,
} from "../object-preview/saved-preview-frames";
import { PreviewFrameRegions } from "../object-preview/PreviewFrameRegions";
import type { ObjectPublication } from "./object-publication";

export function parsePublishedFeedbackFrame(
  raw: unknown,
  projectId: string,
  objectId: string,
  feedback: PublicationPreview["feedback"][number],
) {
  const frames = savedSchema.array().length(1).parse(raw);
  const frame = frames[0]!;
  const target = frame.source.target;
  if (
    !feedback.origin?.publishedFrame ||
    !feedback.previewFrame ||
    frame.projectId !== projectId ||
    frame.source.runId !== frame.runId ||
    frame.runId !== feedback.previewFrame.runId ||
    frame.id !== feedback.previewFrame.frameId ||
    !("versionId" in target) ||
    target.projectId !== projectId ||
    target.objectId !== objectId ||
    target.versionId !== feedback.origin.versionId ||
    !frame.frame.frozen ||
    !frame.selection ||
    frame.selection.sha256 !== frame.frame.sha256 ||
    frame.selection.sequence !== frame.frame.sequence
  )
    throw new Error("PREVIEW_FEEDBACK_SOURCE_MISMATCH");
  return frame;
}

export function PublishedFeedbackFrame({
  session,
  feedback,
  onLoaded,
}: {
  session: ObjectPublication;
  feedback: PublicationPreview["feedback"][number];
  onLoaded?: (frame: SavedPreviewFrame | undefined) => void;
}) {
  const [loaded, setLoaded] = useState<{
    scope: string;
    frame: SavedPreviewFrame;
  }>();
  const [error, setError] = useState("");
  const [revision, setRevision] = useState(0);
  const scope = JSON.stringify([
    session.review,
    feedback.origin,
    feedback.previewFrame,
    revision,
  ]);
  const [readableScope, setReadableScope] = useState("");
  const frame = loaded?.scope === scope ? loaded.frame : undefined;
  const ready = useCallback(
    (value: boolean) => setReadableScope(value ? scope : ""),
    [scope],
  );
  useEffect(() => {
    let active = true;
    setLoaded(undefined);
    setError("");
    void session
      .publicationFrames(
        feedback.origin!.publicationRequestId,
        feedback.previewFrame!,
      )
      .then((raw) => {
        const result = parsePublishedFeedbackFrame(
          raw,
          session.review.projectId,
          session.review.target.objectId,
          feedback,
        );
        if (active) setLoaded({ scope, frame: result });
      })
      .catch((reason: unknown) => {
        if (active) setError(String(reason));
      });
    return () => {
      active = false;
    };
  }, [session, scope]);
  useEffect(() => {
    onLoaded?.(readableScope === scope ? frame : undefined);
    return () => onLoaded?.(undefined);
  }, [onLoaded, frame, readableScope, scope]);
  return (
    <section aria-label="原发布版本编号帧">
      <p>原发布版本：{feedback.origin?.versionId}</p>
      <button type="button" onClick={() => setRevision((value) => value + 1)}>
        重新读取原发布编号帧
      </button>
      {error && <p role="alert">{error}</p>}
      {frame && (
        <PreviewFrameRegions
          key={scope}
          frame={frame.frame}
          selection={frame.selection}
          onReady={ready}
        />
      )}
    </section>
  );
}
