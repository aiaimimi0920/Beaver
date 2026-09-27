import { useEffect, useSyncExternalStore } from "react";
import { Dialog } from "../components";
import type { ObjectTaskExecution } from "./object-task-execution";
import { ObjectTaskRecoveryPanel } from "./ObjectTaskRecoveryPanel";
import { ObjectTaskCheckpointFiles } from "./ObjectTaskCheckpointFiles";
import { ObjectAttemptCheckPanel } from "./ObjectAttemptCheckPanel";
import { ObjectStageAdvancePanel } from "./ObjectStageAdvancePanel";
import { ObjectCandidateReviewPanel } from "./ObjectCandidateReviewPanel";
import { successorFor } from "./object-stage-advance";
import { ObjectAttemptTracePanel } from "./ObjectAttemptTracePanel";
import type { ObjectAttemptCheckRequest } from "../../shared/object-attempt-checks";

const labels = {
  running: "执行中",
  awaitingGate: "输出已保存，等待验收",
  failed: "执行失败",
  interrupted: "已中断",
} as const;

export function ObjectTaskExecutionDialog({
  session,
  close,
  initialCheck,
}: {
  session: ObjectTaskExecution;
  close: () => void;
  initialCheck?: ObjectAttemptCheckRequest;
}) {
  const state = useSyncExternalStore(
    session.subscribe,
    session.getSnapshot,
    session.getSnapshot,
  );
  const recovery = useSyncExternalStore(
    session.recovery.subscribe,
    session.recovery.getSnapshot,
    session.recovery.getSnapshot,
  );
  const retained =
    recovery.dispositionReceipt?.result?.status === "cancelledAndRetained";
  const workspaceRemoved =
    recovery.dispositionReceipt?.result?.status ===
    "cancelledAndWorkspaceRemoved";
  const interruptBlocked = session.recovery.blocksInterrupt();
  useEffect(() => {
    const unsubscribe = window.beaver.subscribe(() => {
      if (!session.getSnapshot().receipt) void session.refresh();
    });
    void session.refresh();
    return () => {
      unsubscribe();
      session.cancel();
    };
  }, [session]);
  const dismiss = () => {
    session.cancel();
    close();
  };
  const busy = state.refreshing || state.phase === "submitting";
  return (
    <Dialog title="任务执行记录" close={dismiss}>
      <p>当前迭代：{session.task.title}</p>
      <p>
        {retained
          ? "当前迭代已取消，历史执行与成果已保留，对象占用已释放。"
          : workspaceRemoved
            ? "当前迭代已取消，工作区已删除，执行回执仍可查看，对象占用已释放。"
            : "中断会保存已有输出并保留当前迭代，后续任务继续等待处理。"}
      </p>
      {state.phase === "loading" && <p role="status">正在读取执行记录…</p>}
      {state.error && <p role="alert">{state.error}</p>}
      {initialCheck &&
        state.phase === "ready" &&
        !state.executions.some(
          ({ attempt }) =>
            attempt.target.attemptId === initialCheck.target.attemptId,
        ) && <p role="alert">指定尝试不存在，以下为该迭代的其他历史记录。</p>}
      {state.phase === "submitting" && (
        <p role="status">正在停止执行并保存输出。关闭窗口后请求仍可能完成。</p>
      )}
      {state.receipt && (
        <p role="status">中断请求已确认；以下为执行实际保存的结果。</p>
      )}
      {state.receipt && state.refreshing && (
        <p role="status">正在刷新任务列表…</p>
      )}
      {state.phase === "ready" && state.executions.length === 0 && (
        <p role="status">本次迭代尚无执行记录。</p>
      )}
      <ol className="object-task-tree" aria-label="本次迭代的执行记录">
        {state.executions.map(
          ({ attempt, availability, definition, checkpoints }) => (
            <li
              key={attempt.target.attemptId}
              aria-current={
                initialCheck?.target.attemptId === attempt.target.attemptId
                  ? "true"
                  : undefined
              }
            >
              <article className="object-task-record">
                {initialCheck?.target.attemptId ===
                  attempt.target.attemptId && (
                  <p role="status">
                    测试页所选尝试 · 报告 {initialCheck.requestId}
                  </p>
                )}
                <header>
                  <h3>{definition.title}</h3>
                  <span className="object-task-status">
                    {availability === "recoveryRequired" &&
                    !retained &&
                    !workspaceRemoved
                      ? "需要恢复处理"
                      : "冻结记录：" + labels[attempt.state]}
                  </span>
                </header>
                <details>
                  <summary>查看本次冻结任务定义</summary>
                  <p>尝试 ID：{attempt.target.attemptId}</p>
                  <p>定义 revision：{definition.revision}</p>
                  <p>以下内容在本次尝试开始时保存，仅供查看。</p>
                  <h4>提示词</h4>
                  <p
                    style={{ whiteSpace: "pre-wrap", overflowWrap: "anywhere" }}
                  >
                    {definition.prompt}
                  </p>
                  <h4>验收要求</h4>
                  <p
                    style={{ whiteSpace: "pre-wrap", overflowWrap: "anywhere" }}
                  >
                    {definition.acceptance || "未填写验收要求"}
                  </p>
                </details>
                <ObjectTaskCheckpointFiles
                  checkpoints={checkpoints}
                  outputCaptured={attempt.outputCaptured}
                  viewer={session.filesFor(attempt)}
                />
                <ObjectAttemptTracePanel session={session.traceFor(attempt)} />
                {availability === "recoveryRequired" &&
                  !retained &&
                  !workspaceRemoved && (
                    <p>
                      该执行记录已失去活动连接，需要恢复处理。重新打开项目不会自动重新执行。
                    </p>
                  )}
                {attempt.outputCaptured && (
                  <ObjectAttemptCheckPanel
                    session={session.checksFor(attempt)}
                    selectedRequestId={
                      initialCheck?.target.attemptId ===
                      attempt.target.attemptId
                        ? initialCheck.requestId
                        : undefined
                    }
                  />
                )}
                {attempt.outputCaptured && (
                  <ObjectStageAdvancePanel
                    session={session}
                    attempt={attempt}
                  />
                )}
                {attempt.state === "awaitingGate" &&
                  attempt.outputCaptured &&
                  !successorFor(session, attempt) && (
                    <ObjectCandidateReviewPanel
                      execution={session}
                      attempt={attempt}
                      session={session.candidateFor(attempt)}
                      checks={session.checksFor(attempt)}
                    />
                  )}
                {attempt.outputCaptured && (
                  <p>
                    {retained
                      ? "本次输出与执行记录已保留，可继续查看历史。"
                      : workspaceRemoved
                        ? "工作区已删除；本次输出与执行回执仍可查看。"
                        : "本次输出已保存。当前接受版本与对象占用状态请查看最终接受与发布记录。"}
                  </p>
                )}
                {attempt.error && <p>执行信息：{attempt.error}</p>}
                {availability === "active" &&
                  !interruptBlocked &&
                  !state.receipt &&
                  !state.retryAvailable && (
                    <button
                      disabled={busy || state.phase !== "ready"}
                      onClick={() => {
                        void session.interrupt(attempt.target.attemptId);
                      }}
                    >
                      中断执行
                    </button>
                  )}
              </article>
            </li>
          ),
        )}
      </ol>
      <ObjectTaskRecoveryPanel session={session.recovery} />
      <footer className="object-task-editor-actions">
        <button
          disabled={busy}
          onClick={() => {
            void session.refresh(true);
          }}
        >
          刷新文件清单
        </button>
        {state.retryAvailable && !interruptBlocked && (
          <button
            disabled={busy}
            onClick={() => {
              void session.interrupt();
            }}
          >
            重试同一中断请求
          </button>
        )}
        <button
          disabled={busy}
          onClick={() => {
            void session.refresh();
          }}
        >
          {state.receipt ? "刷新任务列表" : "刷新执行记录"}
        </button>
        <button onClick={dismiss}>关闭</button>
      </footer>
    </Dialog>
  );
}
