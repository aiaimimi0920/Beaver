import { useState } from "react";
import type { Task } from "../../shared/types";
import { Field } from "../components";
import {
  label,
  type Feedback,
  type Mutate,
  type Selection,
  type ValidationRun,
} from "./types";
import { perform } from "./useValidation";

const statuses: Record<string, string> = {
  taskQueued: "修改任务已排队",
  reviewQueued: "画面审查已排队",
  reviewReady: "审查意见已返回",
  rerunning: "修复后重跑中",
  evidenceReady: "已有新的运行证据",
  rerunFailed: "重跑失败",
  taskFailed: "修改任务需要处理",
  interrupted: "已中断",
  coordinationFailed: "后续处理失败",
};
export function FeedbackPanel({
  run,
  selection,
  change,
  tasks,
  feedback,
  mutate,
  busy,
  openTask,
  openRun,
}: {
  run: ValidationRun;
  selection: Selection;
  change: (value: Selection) => void;
  tasks: Task[];
  feedback: Feedback[];
  mutate: Mutate;
  busy: boolean;
  openTask: (id: string) => void;
  openRun: (id: string) => void;
}) {
  const [text, setText] = useState("");
  const [mode, setMode] = useState("new");
  const [parent, setParent] = useState(run.taskId ?? "");
  const parentTask = tasks.find((t) => t.id === parent);
  const activeParent =
    parentTask && !["completed", "rolledBack"].includes(parentTask.status);
  const media = run.evidence.find((e) => e.id === selection.evidenceId);
  const range = selection.range ?? (media ? [media.start, media.end] : [0, 0]);
  const invalid =
    (mode === "child" && !parent) ||
    (mode === "followup" && (!parent || !!activeParent));
  return (
    <section className="validation-feedback">
      <h3>对话反馈</h3>
      <p className="muted">
        附带此运行的历史代码和原始证据，版本 {run.snapshotId.slice(0, 10)}
        {media ? ` · ${media.point}` : " · 整个运行"}。
      </p>
      {media?.kind === "video" && (
        <div className="validation-form-grid">
          <Field label="反馈起点（秒）">
            <input
              type="number"
              step={0.01}
              min={media.start}
              max={range[1]}
              value={range[0]}
              onChange={(e) =>
                change({
                  ...selection,
                  range: [Number(e.target.value), range[1]],
                })
              }
            />
          </Field>
          <Field label="反馈终点（秒）">
            <input
              type="number"
              step={0.01}
              min={range[0]}
              max={media.end}
              value={range[1]}
              onChange={(e) =>
                change({
                  ...selection,
                  range: [range[0], Number(e.target.value)],
                })
              }
            />
          </Field>
        </div>
      )}
      {selection.region && (
        <p>
          已框选区域：
          {selection.region.map((n) => `${(n * 100).toFixed(1)}%`).join(" / ")}
        </p>
      )}
      <Field label="希望如何修改 / 审查">
        <textarea
          value={text}
          maxLength={10000}
          onChange={(e) => setText(e.target.value)}
          placeholder="例如：打开背包后关闭按钮被遮住，请调整布局。"
        />
      </Field>
      <div className="validation-form-grid">
        <Field label="处理方式">
          <select value={mode} onChange={(e) => setMode(e.target.value)}>
            <option value="new">创建独立修改任务</option>
            <option value="child">创建关联子任务</option>
            <option value="followup">创建已交付任务的后续任务</option>
            <option value="review">请 AI 只读审查画面</option>
          </select>
        </Field>
        <Field label="关联已有任务">
          <select value={parent} onChange={(e) => setParent(e.target.value)}>
            <option value="">不关联</option>
            {tasks.map((t) => (
              <option key={t.id} value={t.id}>
                {t.title}
              </option>
            ))}
          </select>
        </Field>
      </div>
      {activeParent && (
        <p className="validation-notice">
          该任务仍在进行，可直接继续对话指导修改。
          <button onClick={() => openTask(parent)}>进入任务对话</button>
        </p>
      )}
      <button
        className="primary"
        disabled={busy || !text.trim() || invalid}
        onClick={() =>
          perform(
            mutate<{ taskId: string }>("validation.feedback.create", {
              runId: run.id,
              snapshotId: run.snapshotId,
              text,
              mode,
              ...selection,
              ...(parent ? { taskId: parent } : {}),
            }).then(() => setText("")),
          )
        }
      >
        提交反馈并创建任务
      </button>
      <p className="muted">
        AI
        审查只生成意见；用户认可基准需要你在画面上明确确认。修复任务通过后自动重跑原流程。
      </p>
      {feedback
        .filter((f) => f.runId === run.id)
        .map((f) => (
          <article className="validation-feedback-item" key={f.id}>
            <p>{f.text}</p>
            <span>{statuses[f.status] ?? label(f.status)}</span>
            <div className="validation-toolbar">
              <button onClick={() => openTask(f.taskId)}>查看任务</button>
              {f.rerunId && (
                <button onClick={() => openRun(f.rerunId!)}>
                  查看修复后运行
                </button>
              )}
            </div>
            {f.error && <p className="validation-error">{f.error}</p>}
            {f.reason && <p className="validation-error">{f.reason}</p>}
            {f.taskStatus && <small>关联任务：{label(f.taskStatus)}</small>}
            {f.findings != null && (
              <pre>
                {typeof f.findings === "string"
                  ? f.findings
                  : JSON.stringify(f.findings, null, 2)}
              </pre>
            )}
          </article>
        ))}
    </section>
  );
}
