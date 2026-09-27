import type { QueueItem } from "../../shared/object-task-queue";

const blockers: Record<QueueItem["blockers"][number], string> = {
  objectHeld: "对象仍被执行或验收占用",
  earlierQueued: "等待同对象前序任务",
  paused: "任务派发已暂停",
  coarsePaused: "所属粗修派发已暂停",
  dependencies: "依赖尚未验收",
};
const states: Record<QueueItem["state"], string> = {
  queued: "待执行",
  claimed: "准备中",
  running: "执行中",
  awaitingAcceptance: "等待验收",
  failed: "等待恢复",
  cancelled: "已取消",
};
export function ObjectTaskQueueList({
  items,
  disabled,
  dragId,
  start,
  preview,
  drop,
  end,
}: {
  items: QueueItem[];
  disabled: boolean;
  dragId: string | null;
  start: (id: string) => void;
  preview: (id: string, index: number) => void;
  drop: () => void;
  end: () => void;
}) {
  const queued = items.filter((item) => item.state === "queued");
  const remaining = queued.filter((item) => item.taskId !== dragId);
  return (
    <>
      <ol className="object-task-queue-list" aria-label="当前待执行队列">
        {queued.map((item, index) => (
          <li
            key={item.taskId}
            draggable={!disabled}
            onDragStart={(event) => {
              if (disabled) {
                event.preventDefault();
                return;
              }
              event.dataTransfer.effectAllowed = "move";
              event.dataTransfer.setData("text/plain", item.taskId);
              start(item.taskId);
            }}
            onDragOver={(event) => {
              if (disabled || !dragId || dragId === item.taskId) return;
              event.preventDefault();
              event.dataTransfer.dropEffect = "move";
              preview(
                dragId,
                remaining.findIndex((other) => other.taskId === item.taskId),
              );
            }}
            onDrop={(event) => {
              event.preventDefault();
              if (!disabled && dragId) drop();
            }}
            onDragEnd={end}
          >
            <strong>{item.title}</strong> <span>对象 {item.objectId}</span>
            <p>
              {item.blockers.map((code) => blockers[code]).join("；") ||
                "可派发，等待调度"}
            </p>
            <button
              disabled={disabled || index === 0}
              aria-label={"上移 " + item.title}
              onClick={() => preview(item.taskId, index - 1)}
            >
              上移预览
            </button>
            <button
              disabled={disabled || index === queued.length - 1}
              aria-label={"下移 " + item.title}
              onClick={() => preview(item.taskId, index + 1)}
            >
              下移预览
            </button>
          </li>
        ))}
      </ol>
      {dragId && (
        <div
          className="object-task-queue-drop"
          onDragOver={(event) => {
            if (disabled) return;
            event.preventDefault();
            preview(dragId, remaining.length);
          }}
          onDrop={(event) => {
            event.preventDefault();
            if (!disabled) drop();
          }}
        >
          插入队尾
        </div>
      )}
      {!queued.length && <p>当前没有待执行任务。</p>}
      {items.some((item) => item.state !== "queued") && (
        <details>
          <summary>执行与历史记录（不可排序）</summary>
          <ul>
            {items
              .filter((item) => item.state !== "queued")
              .map((item) => (
                <li key={item.taskId}>
                  {item.title} · {states[item.state]}
                </li>
              ))}
          </ul>
        </details>
      )}
    </>
  );
}
