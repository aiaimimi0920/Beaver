import type {
  AssetWork,
  SubtaskStatus,
  WorkAttempt,
} from "../../shared/asset-work";

const labels: Record<SubtaskStatus, string> = {
  pending: "待执行",
  running: "执行中",
  completed: "已报告完成",
  failed: "执行失败",
  interrupted: "已中断，待检查",
  cancelled: "已取消",
  stale: "需重新检查与执行",
};

function Attempt({ attempt }: { attempt: WorkAttempt }) {
  return (
    <details>
      <summary>
        {labels[attempt.status]} · {attempt.startedAt} ·{" "}
        {attempt.id.slice(0, 8)}
      </summary>
      <p>尝试：{attempt.id}</p>
      <p>
        {attempt.definition.title} · {attempt.definition.goal}
      </p>
      <p>当时的验收要求：{attempt.definition.acceptance}</p>
      <p>
        Thread：{attempt.threadId} / Turn：{attempt.turnId}
      </p>
      <p>Blender 会话：{attempt.sessionId || "未绑定"}</p>
      <p>
        资产修订：{attempt.assetRevision} · 结束：
        {attempt.endedAt || "尚未结束"}
      </p>
      <p>输入候选：{attempt.inputCandidates.join(", ") || "无上游候选"}</p>
      <p>起始检查点：{attempt.checkpoint || "未记录"}</p>
      <p>结束检查点：{attempt.endCheckpoint || "未记录"}</p>
      {attempt.recoveryNote && <p>恢复检查说明：{attempt.recoveryNote}</p>}
      {attempt.summary && <p>{attempt.summary}</p>}
      <details>
        <summary>Beaver 保存的输入文件版本</summary>
        {attempt.inputFiles == null ? (
          <p>历史尝试未采集输入文件清单。</p>
        ) : attempt.inputFiles.length === 0 ? (
          <p>本次明确声明无文件输入；Beaver 未自动扫描依赖。</p>
        ) : (
          <>
            <p>
              以下 SHA-256
              来自开始执行时的文件副本。源文件允许编辑；依赖文件在完成报告、提交及批准时核验。
              文件用途由 Codex 声明，清单完整性仍需检查。
            </p>
            <ul>
              {attempt.inputFiles.map((file) => (
                <li key={file.path}>
                  <strong>{file.path}</strong> ·{" "}
                  {file.role === "source" ? "可编辑源文件" : "只读依赖"}
                  <p>SHA-256：{file.sha256}</p>
                </li>
              ))}
            </ul>
          </>
        )}
      </details>
      <details>
        <summary>Codex 报告的输入、输出与工具</summary>
        <pre>
          {JSON.stringify(
            {
              inputs: attempt.inputs,
              outputs: attempt.outputs,
              tools: attempt.tools,
            },
            null,
            2,
          )}
        </pre>
      </details>
    </details>
  );
}

export function WorkPanel({
  work,
  currentStage,
}: {
  work?: AssetWork;
  currentStage?: string;
}) {
  return (
    <section className="asset-work" aria-label="子任务与执行尝试">
      <h3>子任务与执行尝试</h3>
      <p>
        子任务按登记顺序执行。输入、输出与工具由 Codex
        报告；完成报告仍需阶段候选文件核验与用户批准。
      </p>
      {!work?.subtasks.length ? (
        <p>尚未登记子任务。</p>
      ) : (
        <ol>
          {work.subtasks.map((item) => (
            <li key={item.id}>
              <details>
                <summary>
                  {item.definition.title} · {labels[item.status]} ·{" "}
                  {item.stageId}
                  {item.stageId === currentStage ? "（当前阶段）" : ""}
                </summary>
                <p>子任务：{item.id}</p>
                <p>目标：{item.definition.goal}</p>
                <p>验收要求：{item.definition.acceptance}</p>
                {item.note && <p>{item.note}</p>}
                {item.lastAttemptId ? (
                  <p>最近尝试：{item.lastAttemptId}</p>
                ) : (
                  <p>尚无执行尝试。</p>
                )}
                {work.attempts
                  .filter((attempt) => attempt.subtaskId === item.id)
                  .map((attempt) => (
                    <Attempt key={attempt.id} attempt={attempt} />
                  ))}
              </details>
            </li>
          ))}
        </ol>
      )}
    </section>
  );
}
