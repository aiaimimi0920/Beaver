import { useSyncExternalStore } from "react";
import { ObjectTaskDefinitionComparison } from "./ObjectTaskDefinitionComparison";
import type { ObjectTaskRevisionHistoryStore } from "./object-task-revision-history";

export function ObjectTaskRevisionHistory({
  store,
}: {
  store: ObjectTaskRevisionHistoryStore;
}) {
  const state = useSyncExternalStore(
    store.subscribe,
    store.getSnapshot,
    store.getSnapshot,
  );
  return (
    <section
      className="object-task-revision-history"
      aria-label="任务定义修订历史"
      aria-busy={state.loading}
    >
      <header>
        <h3>修订历史</h3>
        <button
          disabled={state.loading}
          onClick={() => {
            void store.load();
          }}
        >
          重新读取修订历史
        </button>
      </header>
      {state.loading && <p role="status">正在读取修订历史…</p>}
      {state.error && <p role="alert">修订历史读取失败：{state.error}</p>}
      {state.loaded && !state.entries.length && <p>该任务尚无定义修订记录。</p>}
      <ol>
        {state.entries.map((entry) => (
          <li key={entry.requestId}>
            <details>
              <summary>
                任务版本 {entry.previousTaskRevision} → {entry.taskRevision} ·
                计划版本 {entry.previousPlanRevision} → {entry.planRevision}
              </summary>
              <p>
                采纳者：项目所有者（{entry.adoptedBy}） ·{" "}
                <time dateTime={entry.createdAt}>{entry.createdAt}</time>
              </p>
              <p className="object-task-revision-reason">
                修订原因：{entry.reason}
              </p>
              <p>
                请求 ID：<code>{entry.requestId}</code>
              </p>
              <ObjectTaskDefinitionComparison
                before={entry.before}
                after={entry.after}
              />
              <p>当时的影响任务：</p>
              <ul aria-label="历史修订影响的任务">
                {entry.affectedTaskIds.map((id) => (
                  <li key={id}>
                    <code>{id}</code>
                  </li>
                ))}
              </ul>
            </details>
          </li>
        ))}
      </ol>
    </section>
  );
}
