import { useEffect, useMemo, useState } from "react";
import type { PublicationPreview } from "../../shared/object-publication";
import type { ObjectPublication } from "./object-publication";
import { ObjectReworkImageSummary } from "./ObjectReworkImageSummary";
import { AttemptFramePicker } from "./AttemptFramePicker";
import { FrozenFeedbackImage } from "./FrozenFeedbackImage";
import type { FrozenFeedbackPng } from "./final-relocation-source";
import { FeedbackRelocationSummary } from "./FeedbackRelocationSummary";
import { PublishedFeedbackFrame } from "./PublishedFeedbackFrame";
import type { SavedPreviewFrame } from "../object-preview/saved-preview-frames";

type Feedback = PublicationPreview["feedback"][number];

export function ObjectPublicationFeedback({
  session,
  feedback,
  onLoaded,
  onImageLoaded,
}: {
  session: ObjectPublication;
  feedback: Feedback;
  onLoaded?: (frame: SavedPreviewFrame | undefined) => void;
  onImageLoaded?: (image: FrozenFeedbackPng | undefined) => void;
}) {
  const [expanded, setOpen] = useState(false);
  const open = expanded || !!onLoaded || !!onImageLoaded;
  const viewer = useMemo(
    () => session.createFeedbackFile(feedback),
    [session, feedback.attemptId, feedback.origin?.runId],
  );
  useEffect(() => () => viewer.cancel(), [viewer]);
  return (
    <section aria-label="已保存返工反馈">
      <p>
        返工反馈（{feedback.attemptId}）：{feedback.feedback}
      </p>
      <ObjectReworkImageSummary image={feedback.image} />
      <FeedbackRelocationSummary relocation={feedback.relocation} />
      {open && feedback.relocation && (
        <AttemptFramePicker
          session={session}
          attemptId={feedback.relocation.sourceAttemptId}
          sourceRunId={feedback.origin?.runId}
          reference={feedback.relocation.sourceFrame}
          label="历史来源编号帧"
        />
      )}
      {feedback.previewFrame && (
        <>
          <button
            type="button"
            aria-expanded={open}
            onClick={() => setOpen(!open)}
          >
            {open ? "收起原始存档" : "回看原始存档与编号"}
          </button>
          {open &&
            (feedback.origin?.publishedFrame ? (
              <PublishedFeedbackFrame
                session={session}
                feedback={feedback}
                onLoaded={onLoaded}
              />
            ) : (
              <AttemptFramePicker
                session={session}
                attemptId={feedback.attemptId}
                sourceRunId={feedback.origin?.runId}
                reference={feedback.previewFrame}
                onLoaded={onLoaded}
              />
            ))}
        </>
      )}
      {feedback.later && (
        <p>
          后续中修：{feedback.later.title} · 验收：{feedback.later.acceptance}
          。发布时创建计划；图片位置须按新版本重新核对。
        </p>
      )}
      {feedback.image && (
        <>
          <button
            type="button"
            aria-expanded={open}
            onClick={() => setOpen(!open)}
          >
            {open ? "收起原始图片" : "回看原始图片与编号"}
          </button>
          {open && (
            <FrozenFeedbackImage
              viewer={viewer}
              image={feedback.image}
              onLoaded={onImageLoaded}
            />
          )}
        </>
      )}
    </section>
  );
}
