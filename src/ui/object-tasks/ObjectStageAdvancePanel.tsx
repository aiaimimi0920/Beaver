import { useState, useSyncExternalStore } from "react";
import type { ObjectExecution } from "../../shared/object-attempts";
import type { ObjectTaskExecution } from "./object-task-execution";
import { prepareStageAdvance, successorFor } from "./object-stage-advance";

export function ObjectStageAdvancePanel({
  session,
  attempt,
}: {
  session: ObjectTaskExecution;
  attempt: ObjectExecution["attempt"];
}) {
  const checks = session.checksFor(attempt);
  const state = useSyncExternalStore(
    checks.subscribe,
    checks.getSnapshot,
    checks.getSnapshot,
  );
  const recovery = useSyncExternalStore(
    session.recovery.subscribe,
    session.recovery.getSnapshot,
    session.recovery.getSnapshot,
  );
  const [note, setNote] = useState("");
  const [error, setError] = useState("");
  if (
    attempt.state !== "awaitingGate" ||
    attempt.taskRevision !== recovery.view?.target.taskRevision
  )
    return null;
  const next = successorFor(session, attempt);
  if (!next)
    return (
      <p>
        当前已是最后细任务，可审阅整体候选；最终人工接受与版本发布尚未开放。
      </p>
    );
  const report = state.reports.at(-1);
  const ready =
    !!report?.passed &&
    !state.busy &&
    !state.retry &&
    recovery.view.canDispose &&
    session.recovery.resumeAvailable() &&
    !session.recovery.resume.blocks() &&
    next.status === "planned" &&
    note.trim().length > 0 &&
    new TextEncoder().encode(note.trim()).length <= 4000;
  return (
    <section aria-label="接受并推进细任务">
      <h4>接受当前细任务，启动下一细任务</h4>
      <p>
        请先完成技术检查和下方的恢复核验，再填写人工验收说明。提交时将再次检查；对象占用继续保留。
      </p>
      <p>
        下一细任务：{next.title}（revision {next.revision}）
      </p>
      <details>
        <summary>查看下一细任务定义</summary>
        <p style={{ whiteSpace: "pre-wrap" }}>{next.prompt}</p>
        <p style={{ whiteSpace: "pre-wrap" }}>
          {next.acceptance || "未填写验收要求"}
        </p>
      </details>
      {report && (
        <p>
          技术报告：{report.request.requestId}（
          {report.passed ? "通过" : "未通过"}）
        </p>
      )}
      <label>
        人工验收说明（最多 4000 UTF-8 字节）
        <textarea
          value={note}
          onChange={(event) => setNote(event.target.value)}
        />
      </label>
      {error && <p role="alert">{error}</p>}
      <button
        disabled={!ready}
        onClick={() => {
          if (prepareStageAdvance(session, attempt, note.trim())) setError("");
          else setError("当前证据或任务版本已变化，请重新检查并核验。");
        }}
      >
        预览接受并推进
      </button>
      {session.recovery.resume.getSnapshot().confirmation?.advance
        ?.attemptId === attempt.target.attemptId && (
        <p role="status">请在下方确认区域检查并启动下一细任务。</p>
      )}
    </section>
  );
}
