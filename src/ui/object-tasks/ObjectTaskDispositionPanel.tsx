import type { ObjectTaskRecovery } from "./object-task-recovery";

export function ObjectTaskDispositionPanel({
  session,
  state,
}: {
  session: ObjectTaskRecovery;
  state: ReturnType<ObjectTaskRecovery["getSnapshot"]>;
}) {
  const busy = state.refreshing || state.phase === "submitting";
  const confirmation = state.confirmation;
  const result = state.dispositionReceipt?.result;
  const retained = result?.status === "cancelledAndRetained";
  const removed = result?.status === "cancelledAndWorkspaceRemoved";
  return (
    <section aria-label="对象任务处置">
      <h4>对象任务处置</h4>
      {state.action === "dispose" && (
        <p role="status">
          正在重新核验并保存处置结果，关闭窗口后请求仍可能完成。
        </p>
      )}
      {confirmation && (
        <div role="group" aria-label="对象任务处置确认">
          <p>
            {confirmation.choice === "cancelAndRemoveWorkspace"
              ? "将取消当前任务，删除本任务已核验的工作目录，并保留执行记录与冻结输出。不会删除 shared content/blob、Codex HOME 或其他任务工作目录。此操作不接受或发布版本。"
              : "将取消当前任务并保留已执行记录、共享工作目录、Codex HOME 和冻结输出。此操作不接受或发布版本。"}
          </p>
          <dl>
            <dt>任务 / 运行</dt>
            <dd>
              {confirmation.target.taskId} / {confirmation.target.runId}
            </dd>
            <dt>确认依据</dt>
            <dd>
              第 {confirmation.target.recoveryGeneration} 次核验，
              {confirmation.verificationRequestId}
            </dd>
          </dl>
          <p>
            确认目标已固定。执行前将再次检查停止证据、记录和文件；不匹配时保留现场与占用。
          </p>
          <div className="object-task-editor-actions">
            <button
              disabled={busy || state.phase !== "ready"}
              onClick={() => {
                void session.dispose();
              }}
            >
              {confirmation.choice === "cancelAndRemoveWorkspace"
                ? "确认取消并删除工作区"
                : "确认取消并保留成果"}
            </button>
            <button disabled={busy} onClick={session.dismissDisposition}>
              返回
            </button>
          </div>
        </div>
      )}
      {state.dispositionRetryAvailable && (
        <p>处置结果尚未确认，重试将使用保存的完整原请求，不会更换目标。</p>
      )}
      {result && (
        <>
          <p role="status">
            {retained
              ? "已取消并保留成果，对象占用已释放。"
              : removed
                ? "已取消并删除本任务工作区，对象占用已释放。"
                : "处置未完成，对象占用尚未释放；请检查报告与现场后重新核验。"}
          </p>
          {retained && (
            <dl>
              <dt>保留工作目录</dt>
              <dd>{result.retained.workspace}</dd>
              <dt>执行记录 / 精修任务</dt>
              <dd>
                {result.retained.attemptId} / {result.retained.fineTaskId}
              </dd>
              <dt>冻结输出文件</dt>
              <dd>{result.retained.outputFileCount}</dd>
            </dl>
          )}
          {removed && (
            <dl>
              <dt>已删除工作目录</dt>
              <dd>{result.removed.workspace}</dd>
              <dt>执行记录 / 精修任务</dt>
              <dd>
                {result.removed.attemptId} / {result.removed.fineTaskId}
              </dd>
            </dl>
          )}
          {result.report.issues.length > 0 && (
            <ul aria-label="处置问题">
              {result.report.issues.map((issue, index) => (
                <li key={index}>{issue}</li>
              ))}
            </ul>
          )}
          <p>回执记录本次处置结果；查看历史不会重新扫描文件。</p>
        </>
      )}
      {!confirmation &&
        !retained &&
        !removed &&
        (state.dispositionRetryAvailable ? (
          <button
            disabled={busy}
            onClick={() => {
              void session.dispose();
            }}
          >
            重试处置
          </button>
        ) : (
          <div className="object-task-editor-actions">
            <button
              disabled={
                busy ||
                state.phase !== "ready" ||
                !state.view?.canDispose ||
                state.retryAvailable
              }
              onClick={session.prepareCancelAndKeep}
            >
              取消并保留成果
            </button>
            <button
              disabled={
                busy ||
                state.phase !== "ready" ||
                !state.view?.canDispose ||
                state.retryAvailable
              }
              onClick={session.prepareCancelAndRemoveWorkspace}
            >
              取消并删除工作区
            </button>
          </div>
        ))}
    </section>
  );
}
