import { useState } from "react";
import type { Task } from "../../shared/types";
import { CodeResults } from "./CodeResults";
import { ConfirmEvidence } from "./ConfirmEvidence";
import { EvidenceViewer } from "./EvidenceViewer";
import { FeedbackPanel } from "./FeedbackPanel";
import { SourcePanel } from "./SourcePanel";
import {
  label,
  running,
  shortId,
  time,
  tone,
  verdictLabel,
  type Feedback,
  type Mutate,
  type Selection,
  type ValidationRun,
} from "./types";
import { perform, useValidationQuery } from "./useValidation";

export function RunDetail({
  projectId,
  runId,
  tasks,
  feedback,
  mutate,
  busy,
  openRun,
  openTask,
}: {
  projectId: string;
  runId: string;
  tasks: Task[];
  feedback: Feedback[];
  mutate: Mutate;
  busy: boolean;
  openRun: (id: string) => void;
  openTask: (id: string) => void;
}) {
  const {
    data: run,
    error,
    refresh,
  } = useValidationQuery<ValidationRun>(
    "validation.run.get",
    { projectId, runId },
    running,
  );
  const [selection, change] = useState<Selection>({});
  const [confirm, setConfirm] = useState(false);
  if (!run)
    return (
      <p className={error ? "validation-error" : "muted"}>
        {error ?? "正在读取运行…"}
        <button onClick={refresh}>刷新</button>
      </p>
    );
  const first = run.evidence[0];
  const selected = selection.evidenceId
    ? selection
    : {
        evidenceId: first?.id,
        ...(first?.kind === "video"
          ? { range: [first.start, first.end] as [number, number] }
          : {}),
      };
  const media = run.evidence.find((e) => e.id === selected.evidenceId);
  const refs = [
    ...(run.flow?.definition.references ?? []),
    ...(media?.references ?? []),
  ];
  const references = refs.filter(
    (r, i) =>
      refs.findIndex((other) => JSON.stringify(other) === JSON.stringify(r)) ===
      i,
  );
  const approved = new Set(
    run.confirmations
      .filter((c) => c.source === "user")
      .flatMap((c) => c.evidenceIds),
  );
  return (
    <article className="validation-run-detail">
      <header>
        <h2>{run.kind === "code" ? "代码验收" : run.flow?.definition.name}</h2>
        <span className={`validation-status validation-${tone(run)}`}>
          {label(run.status)} · {verdictLabel(run)}
        </span>
      </header>
      <p className="muted">
        运行 {shortId(run.id)} · {time(run.createdAt)} · 快照{" "}
        <code title={run.snapshotId}>{shortId(run.snapshotId)}</code>
        {run.flow && ` · 流程 v${run.flow.revision}`}
        {run.releaseId && " · 正式发布候选"}
      </p>
      {run.flow && <p>{run.flow.definition.purpose}</p>}
      {running(run) && (
        <div className="validation-progress" role="status">
          <span>
            {label(run.phase)}
            {run.flow
              ? ` · ${run.completedSteps}/${run.flow.definition.steps.length} 步`
              : " · 忙碌中"}
          </span>
          <progress
            max={run.flow?.definition.steps.length ?? 1}
            value={
              run.flow && run.completedSteps ? run.completedSteps : undefined
            }
          />
        </div>
      )}
      <div className="validation-toolbar">
        <button
          disabled={busy}
          onClick={() =>
            perform(
              mutate<{ runIds: string[] }>("validation.run.rerun", {
                runId: run.id,
              }).then((result) => {
                const firstRun = result.runIds[0];
                if (firstRun) openRun(firstRun);
              }),
            )
          }
        >
          在当前游戏版本重跑原流程
        </button>
        {running(run) && (
          <button
            disabled={busy}
            onClick={() =>
              perform(
                run.managed
                  ? mutate("validation.run.cancel", { runId: run.id })
                  : mutate("task.interrupt", { id: run.taskId }),
              )
            }
          >
            停止本次运行
          </button>
        )}
        {run.taskId && (
          <button onClick={() => openTask(run.taskId!)}>关联任务</button>
        )}
        <button onClick={refresh}>刷新结果</button>
      </div>
      {error && <p className="validation-error">{error}</p>}
      {run.error && <p className="validation-error">{run.error}</p>}
      {run.integrityError && (
        <p className="validation-error">证据完整性异常：{run.integrityError}</p>
      )}
      {run.kind === "code" ? (
        <CodeResults run={run} />
      ) : (
        <>
          <EvidenceViewer run={run} selection={selected} change={change} />
          <div className="validation-toolbar">
            <button
              className="primary"
              disabled={
                busy ||
                run.status !== "completed" ||
                !!run.integrityError ||
                !run.evidence.length
              }
              onClick={() => setConfirm(true)}
            >
              标记画面正确
            </button>
            <span>
              用户已认可 {approved.size}/{run.evidence.length} 项
            </span>
          </div>
          {run.judgments.length > 0 && (
            <details>
              <summary>自动对比依据与差异</summary>
              <pre>{JSON.stringify(run.judgments, null, 2)}</pre>
            </details>
          )}
        </>
      )}
      <SourcePanel key={run.id} run={run} references={references} />
      <FeedbackPanel
        key={run.id}
        run={run}
        selection={selected}
        change={change}
        tasks={tasks}
        feedback={feedback}
        mutate={mutate}
        busy={busy}
        openTask={openTask}
        openRun={openRun}
      />
      <details>
        <summary>引擎、运行器和日志</summary>
        <p>
          {run.engineVersion || "引擎尚未就绪"} · {run.runnerVersion}
        </p>
        <pre>{run.log || "暂无日志"}</pre>
      </details>
      {confirm && (
        <ConfirmEvidence
          run={run}
          selected={selected.evidenceId}
          mutate={mutate}
          busy={busy}
          close={() => setConfirm(false)}
          saved={refresh}
        />
      )}
    </article>
  );
}
