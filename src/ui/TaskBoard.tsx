import { useState } from "react";
import type { Task } from "../shared/types";
import {
  boardColumns,
  directions,
  taskColumn,
  taskDirection,
  taskAttention,
  type TaskDirection,
} from "../shared/task-board";
import { Icon } from "./Icon";
import { statusNames } from "./api";
import "./task-board.css";
import "./task-session.css";

export function TaskBoard({
  tasks,
  open,
  create,
  createLabel = "新任务",
  composing = false,
}: {
  tasks: Task[];
  open: (id: string) => void;
  create: () => void;
  createLabel?: string;
  composing?: boolean;
}) {
  const [query, setQuery] = useState("");
  const [direction, setDirection] = useState<TaskDirection | "all">("all");
  const visible = tasks.filter(
    (task) =>
      (direction === "all" || taskDirection(task) === direction) &&
      `${task.title}\n${task.prompt}`
        .toLowerCase()
        .includes(query.toLowerCase()),
  );
  return (
    <section className="creation-board">
      <header className="board-toolbar">
        <input
          aria-label="搜索任务"
          placeholder="搜索任务"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
        <select
          aria-label="筛选任务方向"
          value={direction}
          onChange={(e) =>
            setDirection(e.target.value as TaskDirection | "all")
          }
        >
          <option value="all">全部方向</option>
          {Object.entries(directions).map(([id, info]) => (
            <option key={id} value={id}>
              {info.label}
            </option>
          ))}
        </select>
        <span className="board-total">{visible.length} 项</span>
        <button className={composing ? "" : "primary"} onClick={create}>
          <Icon name="add" />
          {createLabel}
        </button>
      </header>
      <div className="board-columns">
        {boardColumns.map((column) => {
          const items = visible.filter(
            (task) => taskColumn(task) === column.id,
          );
          return (
            <section
              className={`board-column column-${column.id}`}
              key={column.id}
              aria-label={column.label}
            >
              <header>
                <span className="column-mark" />
                <h2>{column.label}</h2>
                <span>{items.length}</span>
              </header>
              <div className="board-cards">
                {items.map((task) => {
                  const direction = taskDirection(task);
                  const info = directions[direction];
                  const pending =
                    task.clarifications
                      ?.filter((item) => !item.answers)
                      .flatMap((item) => item.questions) ?? [];
                  return (
                    <button
                      key={task.id}
                      className={`board-card direction-${direction} ${task.status === "running" ? "is-running" : ""} ${taskAttention(task) ? "needs-attention" : ""}`}
                      onClick={() => open(task.id)}
                      aria-label={`${task.title} · ${statusNames[task.status]}`}
                    >
                      <span className="direction-tag">
                        <Icon name={info.icon} />
                        {info.label}
                        {taskAttention(task) && (
                          <b className="task-attention">
                            ! {taskAttention(task)}
                          </b>
                        )}
                      </span>
                      <strong>{task.title}</strong>
                      {pending[0] && (
                        <span className="board-question">
                          {pending[0].question}
                        </span>
                      )}
                      <footer>
                        <span className={`card-status ${task.status}`}>
                          {task.accepted ? "已认可" : statusNames[task.status]}
                        </span>
                        <span>
                          {pending.length
                            ? `${pending.length} 个问题`
                            : task.subtaskIds?.length
                              ? `${task.subtaskIds.length} 个子任务`
                              : task.conflicts.length
                                ? `${task.conflicts.length} 个冲突`
                                : task.changes.length
                                  ? `${task.changes.length} 个文件`
                                  : ""}
                        </span>
                      </footer>
                    </button>
                  );
                })}
                {!items.length && (
                  <div className="board-empty">
                    {query || direction !== "all" ? "无匹配任务" : "暂无任务"}
                  </div>
                )}
              </div>
            </section>
          );
        })}
      </div>
    </section>
  );
}
