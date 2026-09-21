import { useState } from "react";
import type { Task } from "../../shared/types";
import { Dialog, Field } from "../components";
import { Icon } from "../Icon";
import type { Flow } from "./flow";
import { FlowEditor } from "./FlowEditor";
import { FlowList } from "./FlowList";
import { ValidationRecords } from "./ValidationRecords";
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
import "./validation-workspace.css";
import "./validation-results.css";

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
  const [page, setPage] = useState<"workbench" | "coverage" | "releases">(
    "workbench",
  );
  const [settingsOpen, setSettingsOpen] = useState(false);
  if (!data)
    return (
      <section className="validation-page">
        <p className={error ? "validation-error" : "muted"}>
          {error ?? "正在读取测试…"}
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
  const openRun = (id: string) => {
    setSelected({ runId: id });
    setPage("workbench");
  };
  const enqueue = (method: string, input: Record<string, unknown> = {}) =>
    perform(
      mutate<{ runIds: string[] }>(method, {
        ...input,
        ...(taskId ? { taskId } : {}),
      }).then((r) => {
        if (r.runIds[0]) openRun(r.runIds[0]);
      }),
    );
  const latestRuns = data.runs.filter(
    (run, index, runs) =>
      runs.findIndex(
        (item) =>
          (item.kind === "code" ? "code" : item.flow?.id) ===
          (run.kind === "code" ? "code" : run.flow?.id),
      ) === index,
  );
  const runCurrent = () =>
    flow
      ? enqueue("validation.flow.run", {
          flowId,
          expectedRevision: flow.revision,
        })
      : enqueue("validation.code.run");
  return (
    <section className="validation-page validation-workbench">
      <header className="validation-workspace-header">
        <h1>
          <Icon name="review" />
          测试
        </h1>
        <nav className="validation-section-nav" aria-label="测试页面">
          {(
            [
              ["workbench", "测试工作台"],
              ["coverage", "任务覆盖"],
              ["releases", "发布记录"],
            ] as const
          ).map(([id, title]) => (
            <button
              key={id}
              aria-current={page === id ? "page" : undefined}
              className={page === id ? "active" : ""}
              onClick={() => setPage(id)}
            >
              {title}
            </button>
          ))}
        </nav>
        <div className="validation-workspace-actions">
          <button onClick={() => setEditor({})} disabled={busy}>
            <Icon name="add" />
            新增流程
          </button>
          <button title="刷新测试" aria-label="刷新测试" onClick={refresh}>
            <Icon name="refresh" />
          </button>
          <button
            title="测试设置"
            aria-label="测试设置"
            onClick={() => setSettingsOpen(true)}
          >
            <Icon name="settings" />
          </button>
        </div>
      </header>
      {error && <p className="validation-error">{error}</p>}
      <div className="validation-workspace-body" hidden={page !== "workbench"}>
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
            <header className="validation-flow-heading">
              <div>
                <span className="validation-eyebrow">
                  {flow ? "画面测试" : "代码测试"}
                  {flow && ` / v${flow.revision}`}
                </span>
                <h2>{flow?.definition.name ?? "GUT 代码验收"}</h2>
                <p className="muted" title={flow?.definition.purpose}>
                  {flow?.definition.purpose ??
                    "运行项目中的 GUT 测试，查看用例结果和错误。"}
                </p>
              </div>
              <div className="validation-toolbar">
                <button
                  className="primary"
                  disabled={busy || !!flow?.definition.retiredReason}
                  onClick={runCurrent}
                >
                  <Icon name="play" />
                  运行当前测试
                </button>
                {flow && (
                  <button disabled={busy} onClick={() => setEditor({ flow })}>
                    编辑流程
                  </button>
                )}
                {flow?.definition.roaming && (
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
              </div>
            </header>
            {flow?.definition.retiredReason && (
              <p className="validation-notice">
                退役原因：{flow.definition.retiredReason}
              </p>
            )}
            {runId && (
              <div className="validation-history-bar">
                <Field label="运行记录">
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
                        {time(r.createdAt)} ·{" "}
                        {r.status === "completed"
                          ? verdictLabel(r)
                          : label(r.status)}
                        {r.releaseId ? " · 发布候选" : ""}
                      </option>
                    ))}
                  </select>
                </Field>
                <span className="muted">
                  {history.length} 次记录 · 切换仅查看历史结果
                </span>
              </div>
            )}
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
              <div className="validation-empty validation-run-empty">
                <Icon name={flow ? "assets" : "code"} />
                <h3>等待第一次测试</h3>
                <p>
                  {flow
                    ? "运行后在这里查看截图、视频与对比结果，框选画面即可提交修改意见。"
                    : "运行后在这里查看通过、失败的用例，以及对应的代码与日志。"}
                </p>
                <button
                  className="primary"
                  disabled={busy || !!flow?.definition.retiredReason}
                  onClick={runCurrent}
                >
                  <Icon name="play" />
                  运行当前测试
                </button>
              </div>
            )}
          </div>
        </div>
      </div>
      {page !== "workbench" && (
        <ValidationRecords
          page={page}
          data={data}
          tasks={tasks}
          openTask={openTask}
          openRun={openRun}
          exportGame={exportGame}
        />
      )}
      <footer className="validation-workspace-status">
        <span className="validation-busy">
          运行中 {data.runs.filter(running).length}
        </span>
        <span className="validation-good">
          最近通过 {latestRuns.filter((run) => tone(run) === "good").length}
        </span>
        <span className="validation-attention">
          需要关注{" "}
          {
            latestRuns.filter((run) => ["bad", "attention"].includes(tone(run)))
              .length
          }
        </span>
        <span className="muted">每个流程按最近一次结果统计</span>
      </footer>
      {settingsOpen && (
        <Dialog title="测试设置" close={() => setSettingsOpen(false)}>
          <ValidationSettings
            key={data.settings.revision}
            settings={data.settings}
            mutate={mutate}
            busy={busy}
          />
          <div className="validation-settings-extra">
            <h3>批量运行</h3>
            <p className="muted">
              日常调试可在工作台单独运行当前测试。需要整体检查时，可重跑当前任务范围内的全部流程。
            </p>
            <button
              disabled={busy}
              onClick={() => {
                enqueue("validation.run.all");
                setSettingsOpen(false);
              }}
            >
              重跑全部流程{taskId ? "（当前任务）" : ""}
            </button>
          </div>
        </Dialog>
      )}
      {editor && (
        <FlowEditor
          flow={editor.flow}
          taskId={taskId}
          tasks={tasks}
          mutate={mutate}
          close={() => setEditor(undefined)}
          saved={(next) => {
            setSelected({ flowId: next.id });
            setPage("workbench");
          }}
        />
      )}
    </section>
  );
}
