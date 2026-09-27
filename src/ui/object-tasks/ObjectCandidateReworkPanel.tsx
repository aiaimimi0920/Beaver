import { useState, useSyncExternalStore } from "react";
import type { ObjectExecution } from "../../shared/object-attempts";
import type { ObjectCandidateReport } from "../../shared/object-candidate-review";
import type { ObjectTaskExecution } from "./object-task-execution";
import { successorFor } from "./object-stage-advance";
import type { ObjectReworkImage } from "../../shared/object-rework-image";
import { ObjectReworkImageEditor } from "./ObjectReworkImageEditor";
import { AttemptFramePicker } from "./AttemptFramePicker";
import type { PreviewFrameReference } from "../../shared/object-publication";
import type { FeedbackRelocation } from "../../shared/preview-feedback-relocation";
import type { SavedPreviewFrame } from "../object-preview/saved-preview-frames";
import { FeedbackRelocationEditor } from "./FeedbackRelocationEditor";
import { confirmedRelocation } from "./feedback-relocation-draft";
import { FeedbackRelocationSummary } from "./FeedbackRelocationSummary";

export function prepareCandidateRework(
  session: ObjectTaskExecution,
  attempt: ObjectExecution["attempt"],
  report: ObjectCandidateReport,
  feedback: string,
  image?: ObjectReworkImage,
  previewFrame?: PreviewFrameReference,
  relocation?: FeedbackRelocation,
) {
  const view = session.recovery.getSnapshot().view;
  if (
    attempt.state !== "awaitingGate" ||
    attempt.taskRevision !== view?.target.taskRevision ||
    !view.canDispose ||
    successorFor(session, attempt) ||
    report.request.projectId !== session.projectId ||
    (image &&
      !report.files.some(
        (file) => file.path === image.path && file.after === image.sha256,
      )) ||
    JSON.stringify(report.request.target) !== JSON.stringify(attempt.target)
  )
    return false;
  return session.recovery.resume.prepareRework({
    reviewRequestId: report.request.requestId,
    attemptId: attempt.target.attemptId,
    fineTaskId: attempt.target.fineTaskId,
    feedback,
    ...(image ? { image } : {}),
    ...(previewFrame ? { previewFrame } : {}),
    ...(relocation ? { relocation } : {}),
  });
}

export function ObjectCandidateReworkPanel({
  session,
  attempt,
  report,
}: {
  session: ObjectTaskExecution;
  attempt: ObjectExecution["attempt"];
  report: ObjectCandidateReport;
}) {
  const [error, setError] = useState("");
  const [image, setImage] = useState<ObjectReworkImage>();
  const [imagePending, setImagePending] = useState(false);
  const [sourceFrame, setSourceFrame] = useState<SavedPreviewFrame>();
  const [targetFrame, setTargetFrame] = useState<SavedPreviewFrame>();
  const publication = session.publicationFor(report);
  const draft = publication.feedbackDraft;
  const draftState = useSyncExternalStore(
    draft.subscribe,
    draft.getSnapshot,
    draft.getSnapshot,
  );
  const { feedback, route, title, acceptance, previewFrame } = draftState.draft;
  const currentFrame =
    targetFrame?.id === previewFrame?.frameId &&
    targetFrame?.runId === previewFrame?.runId
      ? targetFrame
      : undefined;
  const relocation = confirmedRelocation(
    draftState.draft.relocation,
    sourceFrame,
    currentFrame,
  );
  const relocationPending = !!draftState.draft.relocation && !relocation;
  const deferred = publication.deferred;
  const deferredState = useSyncExternalStore(
    deferred.subscribe,
    deferred.getSnapshot,
    deferred.getSnapshot,
  );
  const locked =
    deferredState.busy ||
    !!deferredState.pending ||
    deferredState.blocked ||
    draftState.blocked;
  const recovery = useSyncExternalStore(
    session.recovery.subscribe,
    session.recovery.getSnapshot,
    session.recovery.getSnapshot,
  );
  return (
    <section aria-label="按审阅意见返工">
      {draftState.restored && (
        <p role="status">已恢复此审阅的本地反馈草稿；不会自动提交。</p>
      )}
      {draftState.error && <p role="alert">{draftState.error}</p>}
      {draftState.blocked ? (
        <button onClick={draft.restore}>重新读取反馈草稿</button>
      ) : (
        draftState.error && (
          <button onClick={draft.persist}>重试保存反馈草稿</button>
        )
      )}
      <p>
        填写修改意见后确认，将沿用候选输出重做最后细任务。已接受阶段和原审阅记录保留。请先在恢复面板核验当前输出。
      </p>
      <label>
        反馈去向
        <select
          value={route}
          disabled={locked}
          onChange={(event) =>
            draft.edit({ route: event.target.value as "current" | "later" })
          }
        >
          <option value="current">本轮返工</option>
          <option value="later">排入后续中修</option>
        </select>
      </label>
      <label>
        修改意见
        <textarea
          value={feedback}
          maxLength={4000}
          disabled={locked}
          onChange={(event) => draft.edit({ feedback: event.target.value })}
        />
      </label>
      <AttemptFramePicker
        session={publication}
        attemptId={attempt.target.attemptId}
        reference={previewFrame}
        onLoaded={setTargetFrame}
        onChange={(previewFrame) => {
          setImage(undefined);
          setImagePending(false);
          draft.edit({ previewFrame });
        }}
        disabled={
          locked || session.recovery.resume.blocks() || !!draftState.draft.image
        }
      />
      <FeedbackRelocationEditor
        session={session}
        publication={publication}
        draft={draftState.draft.relocation}
        source={sourceFrame}
        target={currentFrame}
        onLoaded={setSourceFrame}
        onChange={(relocation) => draft.edit({ relocation })}
        disabled={
          locked || session.recovery.resume.blocks() || !!draftState.draft.image
        }
      />
      {!previewFrame && (
        <ObjectReworkImageEditor
          key={`${report.request.requestId}:${draftState.restoreVersion}`}
          draft={draftState.draft.image}
          onDraftChange={(image) => draft.edit({ image })}
          viewer={session.filesFor(
            attempt,
            `feedback:${report.request.requestId}`,
          )}
          report={report}
          disabled={locked || session.recovery.resume.blocks()}
          onChange={(next, pending) => {
            setImage(next);
            setImagePending(pending);
          }}
        />
      )}
      {route === "current" && !deferredState.pending && (
        <button
          disabled={
            !recovery.view?.canDispose ||
            locked ||
            session.recovery.resume.blocks() ||
            imagePending ||
            relocationPending ||
            !!draftState.error ||
            (!!draftState.draft.image && !image) ||
            !feedback.trim()
          }
          onClick={() =>
            setError(
              prepareCandidateRework(
                session,
                attempt,
                report,
                feedback,
                image,
                previewFrame,
                relocation,
              )
                ? ""
                : "当前审阅或核验已过期，或修改意见超过 4000 UTF-8 字节。请刷新并重新核验。",
            )
          }
        >
          准备按此意见返工
        </button>
      )}
      {route === "later" && !deferredState.pending && (
        <>
          <p>
            当前候选须通过发布检查。保存后，发布时须确认排入后续中修；后续任务固定到新发布版本，保持
            planned，需另行入队。图片标注保留原始来源，执行前须重新核对位置。
          </p>
          <label>
            后续中修标题
            <input
              value={title}
              disabled={locked}
              maxLength={300}
              onChange={(event) => draft.edit({ title: event.target.value })}
            />
          </label>
          <label>
            后续验收标准
            <textarea
              value={acceptance}
              disabled={locked}
              maxLength={10000}
              onChange={(event) =>
                draft.edit({ acceptance: event.target.value })
              }
            />
          </label>
          <button
            disabled={
              locked ||
              !!draftState.error ||
              (!!draftState.draft.image && !image) ||
              imagePending ||
              relocationPending ||
              session.recovery.resume.blocks() ||
              !feedback.trim() ||
              !title.trim() ||
              !acceptance.trim()
            }
            onClick={() =>
              void deferred.save({
                feedback,
                later: { title, acceptance },
                ...(previewFrame ? { previewFrame } : image ? { image } : {}),
                ...(relocation ? { relocation } : {}),
              })
            }
          >
            保存并排入后续中修
          </button>
        </>
      )}
      {deferredState.pending && (
        <div>
          <p>
            原请求尚未确认：{deferredState.pending.later.title} ·{" "}
            {deferredState.pending.feedback} · 验收：
            {deferredState.pending.later.acceptance}
          </p>
          <button
            disabled={deferredState.busy || deferredState.blocked}
            onClick={() => void deferred.save()}
          >
            重试原后续反馈请求
          </button>
          <FeedbackRelocationSummary
            relocation={deferredState.pending.relocation}
          />
        </div>
      )}
      {deferredState.saved && (
        <p role="status">
          后续反馈已保存：{deferredState.saved.later.title}
          。请在发布面板确认处置。
        </p>
      )}
      {deferredState.error && <p role="alert">{deferredState.error}</p>}
      {error && <p role="alert">{error}</p>}
    </section>
  );
}
