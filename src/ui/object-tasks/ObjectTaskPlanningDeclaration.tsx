import { useState } from "react";
import type { PlanningState } from "../../shared/object-task-planning-declaration";
import { call } from "../api";
import { PlanningDeclarationSubmission } from "./object-task-planning-declaration";

export function ObjectTaskPlanningDeclaration({
  projectId,
  state,
  planRevision,
  disabled,
  refresh,
}: {
  projectId: string;
  state: PlanningState;
  planRevision: number;
  disabled: boolean;
  refresh: () => Promise<unknown>;
}) {
  const [submission] = useState(
    () => new PlanningDeclarationSubmission(projectId, state.taskId, call),
  );
  const [reason, setReason] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [saved, setSaved] = useState(false);
  const receipt = state.declaration;
  return (
    <section aria-label="规划完成声明">
      <p>
        {state.current
          ? "owner 已确认当前范围规划完整"
          : receipt
            ? "规划声明已失效，请检查范围变化后重新确认"
            : "尚未声明规划完整"}
      </p>
      <p>
        声明只确认需求已展开；任务验收、对象发布和目标完成仍需分别满足门槛。
      </p>
      {receipt && (
        <details>
          <summary>查看最近声明</summary>
          <p>{receipt.createdAt} · owner</p>
          <p>{receipt.request.reason}</p>
          <p>覆盖任务：{receipt.taskIds.join("、")}</p>
        </details>
      )}
      {state.blockers.length > 0 && (
        <ul>
          {state.blockers.map((message) => (
            <li key={message}>{message}</li>
          ))}
        </ul>
      )}
      {(!state.current || submission.pending) && (
        <details>
          <summary>确认当前责任范围的规划</summary>
          <label>
            确认依据
            <textarea
              value={reason}
              disabled={disabled || busy || submission.pending}
              onChange={(event) => {
                setReason(event.target.value);
                setSaved(false);
              }}
            />
          </label>
          <button
            disabled={
              disabled ||
              busy ||
              (!submission.pending &&
                (state.blockers.length > 0 || !reason.trim()))
            }
            onClick={() => {
              setBusy(true);
              setError("");
              setSaved(false);
              void submission
                .submit(state, planRevision, reason)
                .then(async (done) => {
                  if (done) {
                    setSaved(true);
                    setReason("");
                    await refresh();
                  }
                })
                .catch((error) =>
                  setError(
                    error instanceof Error ? error.message : String(error),
                  ),
                )
                .finally(() => setBusy(false));
            }}
          >
            {busy
              ? "正在保存…"
              : submission.pending
                ? "重试原声明"
                : "确认规划完整"}
          </button>
          {submission.pending && (
            <button
              disabled={disabled || busy}
              onClick={() => {
                submission.reset();
                setError("");
                void refresh().catch((error) => setError(String(error)));
              }}
            >
              放弃重试并刷新
            </button>
          )}
        </details>
      )}
      {saved && <p role="status">声明已保存；任务状态未改变。</p>}
      {error && <p role="alert">{error}</p>}
    </section>
  );
}
