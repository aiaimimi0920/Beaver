import { useState, useSyncExternalStore } from "react";
import type { ObjectTaskRevision } from "./object-task-revision";

export function ObjectTaskPromptHistory({
  session,
}: {
  session: ObjectTaskRevision;
}) {
  const history = session.promptHistory;
  const state = useSyncExternalStore(
    history.subscribe,
    history.getSnapshot,
    history.getSnapshot,
  );
  const [attemptId, setAttemptId] = useState("");
  const sources = history.sources();
  const entry = state.entries.find(
    (entry) => entry.attempt.target.attemptId === attemptId,
  );
  if (!sources.length) return <p>暂无同一对象的其他历史迭代可供载入。</p>;
  return (
    <details>
      <summary>载入历史提示词</summary>
      <p>
        载入将替换当前编辑中的提示词和验收要求；保留标题、依赖和修订原因。核对并确认修订后才会保存，不会执行任务。
      </p>
      <label>
        来源迭代
        <select
          value={state.sourceTaskId}
          onChange={(event) => {
            setAttemptId("");
            void history.load(event.target.value);
          }}
        >
          <option value="" disabled>
            选择同一对象的历史迭代
          </option>
          {sources.map((task) => (
            <option key={task.id} value={task.id}>
              {task.title} · {task.id}
            </option>
          ))}
        </select>
      </label>
      {state.loading && <p role="status">正在读取冻结定义…</p>}
      {state.error && <p role="alert">{state.error}</p>}
      {state.sourceTaskId && !state.loading && (
        <button
          onClick={() => {
            setAttemptId("");
            void history.load(state.sourceTaskId);
          }}
        >
          重新读取历史尝试
        </button>
      )}
      {state.loaded && state.entries.length === 0 && (
        <p role="status">所选迭代没有执行尝试。</p>
      )}
      {state.entries.length > 0 && (
        <label>
          来源尝试
          <select
            value={attemptId}
            onChange={(event) => setAttemptId(event.target.value)}
          >
            <option value="" disabled>
              选择冻结定义
            </option>
            {state.entries.map(({ attempt, definition }) => (
              <option
                key={attempt.target.attemptId}
                value={attempt.target.attemptId}
              >
                {definition.title} · {attempt.target.attemptId} · revision{" "}
                {definition.revision}
              </option>
            ))}
          </select>
        </label>
      )}
      {entry && (
        <section aria-label="待载入的历史定义">
          <p>
            来源精修：{entry.attempt.target.fineTaskId} · 定义 revision：
            {entry.definition.revision}
          </p>
          <h4>提示词</h4>
          <p style={{ whiteSpace: "pre-wrap", overflowWrap: "anywhere" }}>
            {entry.definition.prompt}
          </p>
          <h4>验收要求</h4>
          <p style={{ whiteSpace: "pre-wrap", overflowWrap: "anywhere" }}>
            {entry.definition.acceptance || "未填写验收要求"}
          </p>
          <button
            disabled={state.loading}
            onClick={() => session.loadHistoricalPrompt(attemptId)}
          >
            载入并替换当前提示词与验收要求
          </button>
        </section>
      )}
    </details>
  );
}
