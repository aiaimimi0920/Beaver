import { useEffect, useSyncExternalStore } from "react";
import { Dialog } from "../components";
import type { ObjectTaskCancellation } from "./object-task-cancellation";

export function ObjectTaskCancellationDialog({
  session,
  close,
  refresh,
}: {
  session: ObjectTaskCancellation;
  close: () => void;
  refresh: () => Promise<boolean>;
}) {
  const state = useSyncExternalStore(
    session.subscribe,
    session.getSnapshot,
    session.getSnapshot,
  );
  useEffect(() => () => session.cancel(), [session]);
  const dismiss = () => {
    session.cancel();
    close();
  };
  const busy = state.phase === "submitting" || state.refreshing;
  const blocked = state.phase === "blocked";
  return (
    <Dialog
      title="撤销已规划任务"
      close={dismiss}
      className="object-task-cancel-dialog"
    >
      <p>
        目标：{session.task.title}（<code>{session.task.id}</code>）
      </p>
      <p>
        已核对计划版本 {session.planRevision} · 任务版本 {session.task.revision}
      </p>
      {!blocked && (
        <>
          <p>
            本次将撤销责任范围内的 {session.tasks.length}{" "}
            项已规划任务，并关闭所属的 {session.runs.length}{" "}
            次迭代。已撤销记录保持原样。
          </p>
          <ul className="object-task-cancel-scope" aria-label="本次撤销的任务">
            {session.tasks.map((task) => (
              <li key={task.id}>
                {task.title}（<code>{task.id}</code>）
              </li>
            ))}
          </ul>
          {session.runs.length > 0 && (
            <p>
              关闭的迭代：
              {session.runs.map((run) => (
                <code key={run.id}>{run.id} </code>
              ))}
            </p>
          )}
          <p>
            同一对象上的独立任务、依赖任务、对象文件、假设依据和历史记录会保留。
            若范围内已有任务或迭代开始执行，整次撤销会被拒绝。
          </p>
        </>
      )}
      {state.error && <p role="alert">{state.error}</p>}
      {state.phase === "submitting" && (
        <p role="status">正在撤销；关闭窗口后请求仍可能完成。</p>
      )}
      {state.refreshing && <p role="status">正在刷新任务列表…</p>}
      {state.receipt && (
        <p role="status">
          撤销已确认，计划版本为 {state.receipt.planRevision}
          。本地草稿内容保留； 如有版本冲突，请核对任务后在草稿编辑器中处理。
        </p>
      )}
      <footer className="object-task-editor-actions">
        {state.receipt ? (
          <button
            disabled={busy}
            onClick={() => {
              void session.refresh();
            }}
          >
            重试刷新任务列表
          </button>
        ) : (
          !blocked && (
            <button
              disabled={busy}
              onClick={() => {
                void session.submit();
              }}
            >
              {state.phase === "failed" ? "使用原请求重试撤销" : "确认撤销"}
            </button>
          )
        )}
        {(state.phase === "failed" || blocked) && (
          <button
            onClick={() => {
              dismiss();
              void refresh();
            }}
          >
            关闭并刷新后重新确认
          </button>
        )}
        <button onClick={dismiss}>{state.receipt ? "完成" : "返回"}</button>
      </footer>
    </Dialog>
  );
}
