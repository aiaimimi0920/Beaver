import { useState } from "react";
import { call, type Run } from "./api";

interface Inspection {
  target: { path: string; taskId?: string | null };
  sourceCommit: string;
  targetCommit: string;
  runtimeSha256: string;
  changedFiles: string[];
  eligible: boolean;
  blockers: string[];
  reviewRequired: boolean;
  frozenWorkflow: unknown;
  currentWorkflow: unknown;
  latestOperation?: { operationId: string; status: string } | null;
  pendingOperations: { operationId: string; status: string }[];
}
interface UpgradeResult {
  operationId: string;
  status: string;
  reviewRequired: boolean;
}

export function NprUpgradePanel({ id, run }: { id: string; run: Run }) {
  const [taskId, setTaskId] = useState("");
  const [inspection, setInspection] = useState<Inspection>();
  const [result, setResult] = useState<UpgradeResult>();
  const [busy, setBusy] = useState(false);
  const [reason, setReason] = useState("");
  const [acceptance, setAcceptance] = useState("");
  const [reviewed, setReviewed] = useState(false);
  const target = { id, ...(taskId.trim() ? { taskId: taskId.trim() } : {}) };
  const execute = (work: () => Promise<void>) => {
    setBusy(true);
    void run(work).finally(() => setBusy(false));
  };
  const inspect = async () => {
    const next = await call<Inspection>("project.npr.inspectUpgrade", target);
    setInspection(next);
    setResult(
      next.latestOperation
        ? { ...next.latestOperation, reviewRequired: next.reviewRequired }
        : undefined,
    );
  };
  const operationId = result?.operationId;
  return (
    <details>
      <summary>升级 / 回退 NPR 依赖</summary>
      <p>
        先停止项目任务和预览。升级会核验旧文件并保留备份，定制文件冲突时停止。
      </p>
      <label className="field">
        <span>任务 ID（留空升级项目根目录；任务工作副本需分别升级）</span>
        <input
          value={taskId}
          disabled={busy}
          onChange={(event) => {
            setTaskId(event.target.value);
            setInspection(undefined);
            setResult(undefined);
            setReviewed(false);
          }}
        />
      </label>
      <button disabled={busy} onClick={() => execute(inspect)}>
        检查依赖升级
      </button>
      {inspection && (
        <>
          <p>目标：{inspection.target.path}</p>
          <p>
            {inspection.sourceCommit} → {inspection.targetCommit}
          </p>
          <p>
            变更文件：{inspection.changedFiles.length}；原计划和已验收版本保留。
          </p>
          {inspection.blockers.map((blocker) => (
            <p role="alert" key={blocker}>
              {blocker}
            </p>
          ))}
          {inspection.reviewRequired && (
            <p>任务冻结的 NPR 版本需要单独复审；升级不会自动继续任务。</p>
          )}
          <button
            disabled={busy || !inspection.eligible}
            onClick={() => {
              if (
                !window.confirm(
                  "为所选目标升级已核验的 NPR 依赖并保留回退备份？",
                )
              )
                return;
              execute(async () => {
                setResult(
                  await call<UpgradeResult>("project.npr.upgrade", {
                    ...target,
                    expectedOldCommit: inspection.sourceCommit,
                    expectedRuntimeSha256: inspection.runtimeSha256,
                  }),
                );
                setReviewed(false);
                await inspect();
              });
            }}
          >
            升级所选目标
          </button>
        </>
      )}
      {result && (
        <p>
          操作：{result.operationId}；状态：{result.status}
        </p>
      )}
      {operationId && (
        <button
          disabled={busy}
          onClick={() => {
            if (
              !window.confirm(
                "恢复该次升级的原依赖和运行时？后续定制改动会阻止回退。",
              )
            )
              return;
            execute(async () => {
              await call("project.npr.rollbackUpgrade", { id, operationId });
              setResult(undefined);
              setReviewed(false);
              await inspect();
            });
          }}
        >
          回退该次依赖升级
        </button>
      )}
      {result?.reviewRequired &&
        inspection &&
        Boolean(inspection.currentWorkflow) &&
        taskId && (
          <>
            <p>
              依赖复审保留原计划，仅记录本次升级后的执行版本。请核对新依赖和原验收范围。
            </p>
            <pre>{JSON.stringify(inspection.currentWorkflow, null, 2)}</pre>
            <label className="field">
              <span>复审理由</span>
              <textarea
                value={reason}
                disabled={busy}
                onChange={(event) => setReason(event.target.value)}
              />
            </label>
            <label className="field">
              <span>继续执行时保持的验收范围</span>
              <textarea
                value={acceptance}
                disabled={busy}
                onChange={(event) => setAcceptance(event.target.value)}
              />
            </label>
            <button
              disabled={
                busy || reviewed || !reason.trim() || !acceptance.trim()
              }
              onClick={() =>
                execute(async () => {
                  await call("project.npr.reviewUpgrade", {
                    ...target,
                    operationId: result.operationId,
                    expectedRuntimeSha256: inspection.runtimeSha256,
                    expectedFrozenWorkflow: inspection.frozenWorkflow,
                    reviewedWorkflow: inspection.currentWorkflow,
                    reviewReason: reason.trim(),
                    acceptance: acceptance.trim(),
                  });
                  setReviewed(true);
                })
              }
            >
              确认依赖复审（不自动继续）
            </button>
            {reviewed && <p>依赖复审已保存，可在任务面板单独继续。</p>}
          </>
        )}
    </details>
  );
}
