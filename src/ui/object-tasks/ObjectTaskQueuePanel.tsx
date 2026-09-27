import {
  useEffect,
  useMemo,
  useRef,
  useState,
  useSyncExternalStore,
} from "react";
import type { ObjectTaskRecord } from "../../shared/object-tasks";
import { call } from "../api";
import { ObjectTaskQueueSession } from "./object-task-queue-session";
import { ObjectTaskQueueList } from "./ObjectTaskQueueList";
import "./object-task-queue.css";

export function ObjectTaskQueuePanel({
  projectId,
  tasks,
  disabled,
}: {
  projectId: string;
  tasks: ObjectTaskRecord[];
  disabled: boolean;
}) {
  const session = useMemo(
    () => new ObjectTaskQueueSession(projectId, call),
    [projectId],
  );
  const state = useSyncExternalStore(session.subscribe, session.getSnapshot);
  const [dragId, setDragId] = useState<string | null>(null);
  const dropped = useRef(false);
  useEffect(() => {
    void session.refresh();
    const unsubscribe = window.beaver.subscribe(session.notified);
    return () => {
      unsubscribe();
      session.cancel();
    };
  }, [session]);
  const locked = disabled || state.phase !== "ready";
  const items = state.view?.items ?? [];
  const proposal = state.proposal;
  const preview = items.filter(
    (item) => item.state === "queued" && item.taskId !== proposal?.taskId,
  );
  const moved = items.find((item) => item.taskId === proposal?.taskId);
  if (proposal && moved) {
    const index =
      proposal.previousTaskId === null
        ? 0
        : preview.findIndex((item) => item.taskId === proposal.previousTaskId) +
          1;
    preview.splice(index, 0, moved);
  }
  return (
    <section
      className="object-task-queue"
      aria-label="执行队列"
      aria-busy={state.phase === "submitting"}
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          setDragId(null);
          session.discard();
        }
      }}
    >
      <header>
        <h3>执行队列</h3>
        <button
          disabled={
            state.phase === "submitting" ||
            state.phase === "loading" ||
            state.retry
          }
          onClick={() => {
            setDragId(null);
            void session.refresh();
          }}
        >
          刷新队列
        </button>
      </header>
      <p>
        拖动到任务前方或队尾，或使用上移、下移按钮预览；确认后保存。阻塞队头不能被同对象任务绕过。
      </p>
      {state.phase === "loading" && <p role="status">正在读取队列…</p>}
      {state.error && <p role="alert">{state.error}</p>}
      {state.retry && (
        <button disabled={disabled} onClick={() => void session.retry()}>
          按原请求重试
        </button>
      )}
      <ObjectTaskQueueList
        items={items}
        disabled={locked}
        dragId={dragId}
        start={(id) => {
          session.discard();
          dropped.current = false;
          setDragId(id);
        }}
        preview={session.preview}
        drop={() => {
          dropped.current = true;
          setDragId(null);
        }}
        end={() => {
          setDragId(null);
          if (!dropped.current) session.discard();
        }}
      />
      {proposal && (
        <div className="object-task-queue-preview" role="status">
          <h4>插入位置预览（尚未保存）</h4>
          <ol>
            {preview.map((item) => (
              <li
                key={item.taskId}
                className={
                  item.taskId === proposal.taskId
                    ? "queue-insertion"
                    : undefined
                }
              >
                {item.title}
              </li>
            ))}
          </ol>
          <button
            disabled={locked || !!dragId}
            onClick={() => void session.confirm()}
          >
            确认调整顺序
          </button>
          <button disabled={locked} onClick={session.discard}>
            取消预览
          </button>
        </div>
      )}
      {state.receipt && (
        <p>
          已保存排序回执：{state.receipt.request.requestId}
          。上方列表来自最新队列读取，可能已继续派发。
        </p>
      )}
      <details>
        <summary>将未入队的中修加入执行队列</summary>
        <p>
          加入后调度器可立即开始准备与执行。需要先安排顺序时，请在任务的派发控制中暂停，排好后再解除暂停。
        </p>
        {tasks
          .filter(
            (task) =>
              task.granularity === "medium" &&
              task.status === "planned" &&
              !items.some((item) => item.taskId === task.id),
          )
          .map((task) => (
            <button
              key={task.id}
              disabled={locked || !!proposal}
              onClick={() => void session.enqueue(task.id)}
            >
              加入队列：{task.title}
            </button>
          ))}
      </details>
    </section>
  );
}
