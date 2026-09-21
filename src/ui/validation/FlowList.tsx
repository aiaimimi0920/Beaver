import { useState } from "react";
import type { Task } from "../../shared/types";
import { Field } from "../components";
import { Icon } from "../Icon";
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
  const matchesStatus = (run: ValidationRun | undefined) => {
    const state = run ? tone(run) : "none";
    return (
      !status ||
      (status === "attention"
        ? ["bad", "attention"].includes(state)
        : state === status)
    );
  };
  const search = query.trim().toLowerCase();
  const visible = flows.filter((flow) => {
    const def = flow.definition;
    return (
      (!category || def.category === category) &&
      (!taskId ||
        def.taskIds.includes(taskId) ||
        runs.some(
          (run) => run.flow?.id === flow.id && run.taskId === taskId,
        )) &&
      matchesStatus(latest(flow)) &&
      `${def.name} ${def.purpose}`.toLowerCase().includes(search)
    );
  });
  const showCode =
    !category &&
    matchesStatus(code) &&
    "gut 代码验收 代码测试".includes(search);
  return (
    <aside className="validation-flow-list">
      <header className="validation-list-heading">
        <h2>测试流程</h2>
        <span>{visible.length + Number(showCode)}</span>
      </header>
      <div className="validation-list-filters">
        <label className="validation-flow-search">
          <Icon name="search" />
          <input
            aria-label="搜索测试流程"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="搜索名称或目的"
          />
          {query && (
            <button aria-label="清空搜索" onClick={() => setQuery("")}>
              <Icon name="close" />
            </button>
          )}
        </label>
        <Field label="任务范围">
          <select value={taskId} onChange={(e) => changeTask(e.target.value)}>
            <option value="">全部任务</option>
            {tasks.map((t) => (
              <option key={t.id} value={t.id}>
                {t.title}
              </option>
            ))}
          </select>
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
      </div>
      <div className="validation-flow-scroll">
        {showCode && (
          <button
            className={`validation-flow-item${selected === "code" ? " active" : ""}`}
            aria-pressed={selected === "code"}
            onClick={() => select("code")}
          >
            <strong>
              <Icon name="code" />
              GUT 代码验收
            </strong>
            <small className={code ? `validation-${tone(code)}` : "muted"}>
              {code
                ? `${label(code.status)} · ${verdictLabel(code)}`
                : "尚未运行"}
            </small>
          </button>
        )}
        {Object.entries(categories)
          .filter(([id]) => !category || category === id)
          .map(([id, title]) => {
            const group = visible.filter(
              (flow) => flow.definition.category === id,
            );
            if (!group.length) return null;
            return (
              <section key={id}>
                <h3>
                  {title}
                  <span>{group.length}</span>
                </h3>
                {group.map((flow) => {
                  const recent = latest(flow);
                  return (
                    <button
                      key={flow.id}
                      className={`validation-flow-item${selected === flow.id ? " active" : ""}`}
                      aria-pressed={selected === flow.id}
                      title={flow.definition.purpose}
                      onClick={() => select(flow.id)}
                    >
                      <strong>{flow.definition.name}</strong>
                      <small
                        className={
                          recent ? `validation-${tone(recent)}` : "muted"
                        }
                      >
                        {recent
                          ? `${label(recent.status)} · ${verdictLabel(recent)}`
                          : "尚未运行"}
                        <span className="muted">
                          {" "}
                          · v{flow.revision}
                          {flow.definition.retiredReason ? " · 已退役" : ""}
                          {recent && recent.flow?.revision !== flow.revision
                            ? " · 旧流程结果"
                            : ""}
                        </span>
                      </small>
                    </button>
                  );
                })}
              </section>
            );
          })}
        {!showCode && !visible.length && (
          <div className="validation-list-empty">
            <p>没有匹配的测试流程</p>
            <button
              onClick={() => {
                setQuery("");
                setStatus("");
                setCategory("");
                changeTask("");
              }}
            >
              清除筛选
            </button>
          </div>
        )}
        {!flows.length && !search && !category && !status && (
          <p className="validation-list-empty">
            使用右上角的“新增流程”添加画面测试。
          </p>
        )}
      </div>
    </aside>
  );
}
