import { useState } from "react";
import type { Task } from "../../shared/types";
import { Field } from "../components";
import type { Flow } from "./flow";
import { FlowEditor } from "./FlowEditor";
import { FlowList } from "./FlowList";
import { ReleaseStatus } from "./ReleaseStatus";
import { RunDetail } from "./RunDetail";
import { ValidationSettings } from "./ValidationSettings";
import {
  label,
  running,
  shortId,
  time,
  tone,
  verdictLabel,
  type ValidationData,
} from "./types";
import {
  perform,
  useValidationMutation,
  useValidationQuery,
} from "./useValidation";
import "./validation.css";
import "./validation-media.css";

export function ValidationView({
  projectId,
  tasks,
  initialTaskId = "",
  initialRunId,
  openTask,
  exportGame,
}: {
  projectId: string;
  tasks: Task[];
  initialTaskId?: string;
  initialRunId?: string;
  openTask: (id: string) => void;
  exportGame: (releaseId?: string) => void;
}) {
  const { data, error, refresh } = useValidationQuery<ValidationData>(
    "validation.list",
    { projectId },
    (v) => v.runs.some(running),
  );
  const { busy, mutate } = useValidationMutation(projectId, refresh);
  const [taskId, setTaskId] = useState(initialTaskId);
  const [selected, setSelected] = useState<{ flowId?: string; runId?: string }>(
    { runId: initialRunId },
  );
  const [editor, setEditor] = useState<{ flow?: Flow }>();
  if (!data)
    return (
      <section className="validation-page">
        <p className={error ? "validation-error" : "muted"}>
          {error ?? "正在读取测试与画面…"}
        </p>
        <button onClick={refresh}>刷新</button>
      </section>
    );
  const activeRun = data.runs.find((r) => r.id === selected.runId);
  const flowId = selected.flowId ?? activeRun?.flow?.id ?? "code";
  const flow = data.flows.find((f) => f.id === flowId);
  const history = data.runs.filter(
    (r) =>
      (flowId === "code" ? r.kind === "code" : r.flow?.id === flowId) &&
      (!taskId ||
        r.taskId === taskId ||
        r.flow?.definition.taskIds.includes(taskId)),
  );
  const runId = selected.runId ?? history[0]?.id;
  const openRun = (id: string) => setSelected({ runId: id });
  const enqueue = (method: string, input: Record<string, unknown> = {}) =>
    perform(
      mutate<{ runIds: string[] }>(method, {
        ...input,
        ...(taskId ? { taskId } : {}),
      }).then((r) => {
        if (r.runIds[0]) openRun(r.runIds[0]);
      }),
    );
  const latestCode = data.runs.find((r) => r.kind === "code");
  return (
    <section className="validation-page">
      <div className="validation-summary">
        <span
          className={latestCode ? `validation-${tone(latestCode)}` : "muted"}
        >
          最近代码：{latestCode ? verdictLabel(latestCode) : "尚未建立验收结果"}
        </span>
        <span>
          运行中 {data.runs.filter(running).length} · 画面流程{" "}
          {data.flows.filter((f) => !f.definition.retiredReason).length}
        </span>
        <span>日常画面红项不撤销已交付任务</span>
      </div>
      <ValidationSettings
        key={data.settings.revision}
        settings={data.settings}
        mutate={mutate}
        busy={busy}
      />
      <div className="validation-toolbar">
        <button disabled={busy} onClick={() => enqueue("validation.code.run")}>
          运行代码验收
        </button>
        <button disabled={busy} onClick={() => enqueue("validation.run.all")}>
          重跑全部流程
        </button>
        <button disabled={busy} onClick={() => setEditor({})}>
          新增画面流程
        </button>
        <button onClick={() => exportGame()}>正式发布检查 / 内部导出</button>
        <button onClick={refresh}>刷新</button>
      </div>
      {error && <p className="validation-error">{error}</p>}
      <div className="validation-layout">
        <FlowList
          flows={data.flows}
          runs={data.runs}
          tasks={tasks}
          taskId={taskId}
          changeTask={(id) => {
            setTaskId(id);
            setSelected({});
          }}
          selected={flowId}
          select={(id) => setSelected({ flowId: id })}
        />
        <div className="validation-main">
          {flow && (
            <div className="validation-toolbar">
              <button
                disabled={busy || !!flow.definition.retiredReason}
                onClick={() =>
                  enqueue("validation.flow.run", {
                    flowId,
                    expectedRevision: flow.revision,
                  })
                }
              >
                运行流程 v{flow.revision}
              </button>
              <button disabled={busy} onClick={() => setEditor({ flow })}>
                编辑流程
              </button>
              {flow.definition.roaming && (
                <button
                  disabled={busy || !!flow.definition.retiredReason}
                  onClick={() =>
                    perform(
                      mutate<Flow>("validation.flow.explore", {
                        flowId,
                        expectedRevision: flow.revision,
                        seed: crypto.getRandomValues(new Uint32Array(1))[0],
                      }).then((next) => setSelected({ flowId: next.id })),
                    )
                  }
                >
                  生成新随机路线
                </button>
              )}
              {flow.definition.retiredReason && (
                <span>退役原因：{flow.definition.retiredReason}</span>
              )}
            </div>
          )}
          <Field label="运行历史（每次运行独立保存，列表展示最近 200 次）">
            <select
              value={runId ?? ""}
              onChange={(e) => openRun(e.target.value)}
            >
              {!runId && <option value="">尚无运行</option>}
              {runId && !history.some((r) => r.id === runId) && (
                <option value={runId}>指定运行 {shortId(runId)}</option>
              )}
              {history.map((r) => (
                <option key={r.id} value={r.id}>
                  {time(r.createdAt)} · {shortId(r.snapshotId)} ·{" "}
                  {label(r.status)} · {verdictLabel(r)}
                  {r.releaseId ? " · 发布候选" : ""}
                </option>
              ))}
            </select>
          </Field>
          {runId ? (
            <RunDetail
              key={runId}
              projectId={projectId}
              runId={runId}
              tasks={tasks}
              feedback={data.feedback}
              mutate={mutate}
              busy={busy}
              openRun={openRun}
              openTask={openTask}
            />
          ) : (
            <p className="validation-notice">
              选择已有流程运行，或让开发任务根据玩法生成流程。这里会保留每次运行的截图、视频和历史代码。
            </p>
          )}
        </div>
      </div>
      <details className="validation-coverage">
        <summary>
          任务画面覆盖与自动修复 · {data.coverage.length} 个任务
        </summary>
        {data.coverage.map((c) => (
          <article key={c.taskId}>
            <button onClick={() => openTask(c.taskId)}>
              {tasks.find((t) => t.id === c.taskId)?.title ?? c.taskId}
            </button>
            <span
              className={
                c.status === "missing" || c.status === "failed"
                  ? "validation-bad"
                  : "muted"
              }
            >
              画面：{label(c.status)}
            </span>
            <p>{c.reason}</p>
            {c.runIds?.map((id) => (
              <button key={id} onClick={() => openRun(id)}>
                查看运行 {shortId(id)}
              </button>
            ))}
          </article>
        ))}
        {data.repairDecisions.map((d) => (
          <article key={d.runId}>
            <button onClick={() => openRun(d.runId)}>
              代码修复 {shortId(d.runId)}
            </button>
            <span>
              {label(d.status)} · {d.reason}
            </span>
            {d.taskId && (
              <button onClick={() => openTask(d.taskId!)}>关联修复任务</button>
            )}
          </article>
        ))}
        {!data.coverage.length && (
          <p className="muted">
            大任务完成后会登记画面覆盖；缺少流程会在此明确显示。
          </p>
        )}
      </details>
      <details className="validation-releases">
        <summary>正式发布诊断历史 · {data.releases.length}</summary>
        {data.releases.map((check) => (
          <details key={check.id}>
            <summary>
              {time(check.createdAt)} · {check.preset} ·{" "}
              {check.ready ? "通过" : "待处理"}
            </summary>
            <ReleaseStatus check={check} openRun={openRun} />
            <button onClick={() => exportGame(check.id)}>
              使用此固定候选导出
            </button>
          </details>
        ))}
      </details>
      {editor && (
        <FlowEditor
          flow={editor.flow}
          taskId={taskId}
          tasks={tasks}
          mutate={mutate}
          close={() => setEditor(undefined)}
          saved={(next) => setSelected({ flowId: next.id })}
        />
      )}
    </section>
  );
}
