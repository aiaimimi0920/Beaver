import { useState } from "react";
import type { Task } from "../../shared/types";
import { Field } from "../components";
import { categories, type Flow } from "./flow";
import { label, tone, verdictLabel, type ValidationRun } from "./types";

export function FlowList({
  flows,
  runs,
  tasks,
  taskId,
  changeTask,
  selected,
  select,
}: {
  flows: Flow[];
  runs: ValidationRun[];
  tasks: Task[];
  taskId: string;
  changeTask: (id: string) => void;
  selected: string;
  select: (id: string) => void;
}) {
  const [category, setCategory] = useState("");
  const [status, setStatus] = useState("");
  const [query, setQuery] = useState("");
  const code = runs.find(
    (r) => r.kind === "code" && (!taskId || r.taskId === taskId),
  );
  const latest = (flow: Flow) =>
    runs.find(
      (run) =>
        run.flow?.id === flow.id &&
        (!taskId ||
          run.taskId === taskId ||
          flow.definition.taskIds.includes(taskId)),
    );
  return (
    <aside className="validation-flow-list">
      <Field label="关联任务">
        <select value={taskId} onChange={(e) => changeTask(e.target.value)}>
          <option value="">全部任务</option>
          {tasks.map((t) => (
            <option key={t.id} value={t.id}>
              {t.title}
            </option>
          ))}
        </select>
      </Field>
      <Field label="查找流程">
        <input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="名称或目的"
        />
      </Field>
      <div className="validation-form-grid">
        <Field label="分类">
          <select
            value={category}
            onChange={(e) => setCategory(e.target.value)}
          >
            <option value="">全部分类</option>
            {Object.entries(categories).map(([id, title]) => (
              <option key={id} value={id}>
                {title}
              </option>
            ))}
          </select>
        </Field>
        <Field label="状态">
          <select value={status} onChange={(e) => setStatus(e.target.value)}>
            <option value="">全部状态</option>
            <option value="good">已通过</option>
            <option value="attention">需要关注</option>
            <option value="busy">运行中</option>
            <option value="none">未运行</option>
          </select>
        </Field>
      </div>
      <button
        className={`validation-flow-item${selected === "code" ? " active" : ""}`}
        onClick={() => select("code")}
      >
        <strong>GUT 代码验收</strong>
        <small className={code ? `validation-${tone(code)}` : "muted"}>
          {code ? `${label(code.status)} · ${verdictLabel(code)}` : "尚未运行"}
        </small>
      </button>
      {Object.entries(categories)
        .filter(([id]) => !category || category === id)
        .map(([id, title]) => {
          const visible = flows.filter((flow) => {
            const def = flow.definition;
            const recent = latest(flow);
            const state = recent ? tone(recent) : "none";
            return (
              def.category === id &&
              (!taskId ||
                def.taskIds.includes(taskId) ||
                runs.some(
                  (r) => r.flow?.id === flow.id && r.taskId === taskId,
                )) &&
              (!status ||
                (status === "attention"
                  ? ["bad", "attention"].includes(state)
                  : state === status)) &&
              `${def.name} ${def.purpose}`
                .toLowerCase()
                .includes(query.toLowerCase())
            );
          });
          return (
            <section key={id}>
              <h3>
                {title} · {visible.length}
              </h3>
              {visible.map((flow) => {
                const recent = latest(flow);
                return (
                  <button
                    key={flow.id}
                    className={`validation-flow-item${selected === flow.id ? " active" : ""}`}
                    onClick={() => select(flow.id)}
                  >
                    <strong>{flow.definition.name}</strong>
                    <small>
                      v{flow.revision}
                      {flow.definition.retiredReason ? " · 已退役" : ""}
                      {recent && recent.flow?.revision !== flow.revision
                        ? " · 最近结果来自旧流程"
                        : ""}
                    </small>
                    <small
                      className={
                        recent ? `validation-${tone(recent)}` : "muted"
                      }
                    >
                      {recent
                        ? `${label(recent.status)} · ${verdictLabel(recent)}`
                        : "尚未运行"}
                    </small>
                  </button>
                );
              })}
            </section>
          );
        })}
    </aside>
  );
}
