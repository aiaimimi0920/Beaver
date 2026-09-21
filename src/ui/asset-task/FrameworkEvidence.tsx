import type { FrameworkAction, FrameworkState } from "../../shared/framework";
import { operationTerminal } from "../../shared/framework";

export function FrameworkEvidence({
  data,
  busy,
  action,
}: {
  data: FrameworkState;
  busy: boolean;
  action: FrameworkAction;
}) {
  return (
    <>
      <h4>持久操作</h4>
      {!data.operations.length && <p>尚无框架操作。</p>}
      {data.operations.map((op) => (
        <details key={op.id}>
          <summary>
            {op.job.kind} · {op.status} · {op.id}
          </summary>
          <p>
            {op.source} · {op.createdAt} · 结束：{op.endedAt || "未结束"}
          </p>
          {op.error && <p role="status">{op.error}</p>}
          {!operationTerminal(op.status) && (
            <button
              disabled={busy || op.status === "cancelRequested"}
              onClick={() =>
                void action({ operation: "cancel", operationId: op.id })
              }
            >
              取消操作
            </button>
          )}
          <pre>
            {JSON.stringify({ job: op.job, result: op.result }, null, 2)}
          </pre>
        </details>
      ))}
      <p>
        取消后先检查文件与场景。宿主重启保留中断记录，外部修改不会自动重放。
      </p>
      {(
        [
          ["checks", "候选技术检查"],
          ["judgments", "视觉评价与来源"],
          ["traces", "实际观察到的工具事件"],
          ["observations", "实时观察帧身份"],
          ["recovery", "恢复历史"],
        ] as const
      ).map(([key, label]) => (
        <details key={key}>
          <summary>
            {label} · {data[key].length}
          </summary>
          <pre>{JSON.stringify(data[key], null, 2)}</pre>
        </details>
      ))}
      <details>
        <summary>当前恢复与证据范围</summary>
        <pre>
          {JSON.stringify(
            { recovery: data.currentRecovery, coverage: data.coverage },
            null,
            2,
          )}
        </pre>
      </details>
      <p>
        工具事件只保存请求和结果的结构及摘要；未观察到的 Skill
        使用不会补造执行证明。实时帧与冻结候选预览分别保存。
      </p>
    </>
  );
}
