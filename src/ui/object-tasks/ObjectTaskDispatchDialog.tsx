import { useEffect, useSyncExternalStore } from "react";
import { Dialog } from "../components";
import { objectTaskStatusLabels } from "./object-task-status";
import type { ObjectTaskDispatch } from "./object-task-dispatch";

export function ObjectTaskDispatchDialog({
  session,
  close,
}: {
  session: ObjectTaskDispatch;
  close: () => void;
}) {
  const state = useSyncExternalStore(
    session.subscribe,
    session.getSnapshot,
    session.getSnapshot,
  );
  useEffect(() => {
    const unsubscribe = window.beaver.subscribe(() => {
      if (!session.getSnapshot().receipt) void session.refresh();
    });
    void session.refresh();
    return () => {
      unsubscribe();
      session.cancel();
    };
  }, [session]);
  const dismiss = () => {
    session.cancel();
    close();
  };
  const busy = state.refreshing || state.phase === "submitting";
  return (
    <Dialog title="派发控制" close={dismiss}>
      <p>
        {state.task.granularity === "coarse" ? "当前粗修" : "当前迭代"}：
        {state.task.title}
      </p>
      <p>任务状态：{objectTaskStatusLabels[state.task.status]}</p>
      <p>
        {state.task.granularity === "coarse"
          ? "暂停会阻止该粗修下现有及后续新增中修的新派发；解除粗修暂停会保留每个中修自身的暂停设置。"
          : "暂停会阻止本次迭代的新派发。"}
        已经派发的准备和执行继续完成。如需立即停止，请使用执行记录中的中断操作。
      </p>
      <p>
        暂停的队首仍保留顺序，同对象的后续迭代继续等待，其他对象可继续派发。
      </p>
      <p>
        解除暂停只恢复排队资格；失败、失去连接和等待验收的任务仍需后续处理。
      </p>
      {state.phase === "loading" && <p role="status">正在读取派发状态…</p>}
      {state.error && <p role="alert">{state.error}</p>}
      {state.parentPaused && (
        <p role="status">
          所属粗修已暂停派发，解除本次迭代的暂停后仍需等待粗修解除暂停。
        </p>
      )}
      {state.phase === "submitting" && (
        <p role="status">正在保存派发控制。关闭窗口后请求仍可能完成。</p>
      )}
      {state.receipt ? (
        <p role="status">
          {state.receipt.result.paused ? "暂停派发" : "解除暂停"}
          请求已确认。回执记录本次操作结果，最新状态请查看任务列表。
        </p>
      ) : (
        state.phase === "ready" && (
          <p role="status">
            当前派发状态：{state.control.paused ? "已暂停" : "未暂停"}
          </p>
        )
      )}
      {state.task.status === "cancelled" && (
        <p>该任务已撤销，不能再更改派发状态。</p>
      )}
      {state.retryAvailable && (
        <p>上次请求结果尚未确认，重试会核验同一次操作。</p>
      )}
      {state.receipt && state.refreshing && (
        <p role="status">正在刷新任务列表…</p>
      )}
      <footer className="object-task-editor-actions">
        {!state.receipt &&
          !state.retryAvailable &&
          state.task.status !== "cancelled" && (
            <button
              disabled={busy || state.phase !== "ready"}
              onClick={() => {
                void session.confirm();
              }}
            >
              {state.control.paused ? "确认解除暂停" : "确认暂停派发"}
            </button>
          )}
        {state.retryAvailable && (
          <button
            disabled={busy}
            onClick={() => {
              void session.confirm();
            }}
          >
            重试同一派发控制请求
          </button>
        )}
        <button
          disabled={busy}
          onClick={() => {
            void session.refresh();
          }}
        >
          {state.receipt ? "刷新任务列表" : "刷新派发状态"}
        </button>
        <button onClick={dismiss}>关闭</button>
      </footer>
    </Dialog>
  );
}
