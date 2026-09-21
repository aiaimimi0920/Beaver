import type { CSSProperties } from "react";
import { Icon } from "../Icon";
import { demoObjects, type DemoObject } from "./mock-objects";
import type { DemoTask } from "./mock-tasks";
import { objectCategories } from "./object-categories";
import { ongoingObjectTasks } from "./useObjectTaskNavigation";
import "./preview-object-task-rail.css";

export function ObjectTaskRail({
  expanded,
  toggle,
  activeTaskId,
  slotObject,
  selectSlot,
  selectTask,
}: {
  expanded: boolean;
  toggle: () => void;
  activeTaskId: string | null;
  slotObject: DemoObject | null;
  selectSlot: () => void;
  selectTask: (task: DemoTask) => void;
}) {
  const slotCategory = objectCategories.find(
    (item) => item.name === slotObject?.objectType,
  );
  const slotLabel = slotObject
    ? `${slotObject.name} · 暂无进行中的任务`
    : "空白对象入口";
  return (
    <aside className="op-object-task-rail" aria-label="进行中的对象任务">
      <button
        type="button"
        className="op-task-rail-toggle"
        aria-label={expanded ? "收起对象详情" : "展开对象详情"}
        title={expanded ? "收起对象详情" : "展开对象详情"}
        aria-expanded={expanded}
        aria-controls="op-object-inspector"
        onClick={toggle}
      >
        <Icon name="back" />
      </button>
      <div className="op-task-rail-list">
        <button
          type="button"
          className={`op-task-rail-item op-task-rail-slot ${!activeTaskId ? "is-active" : ""}`}
          style={
            {
              "--task-color": slotCategory?.color ?? "#9da6b3",
            } as CSSProperties
          }
          title={slotLabel}
          aria-label={slotLabel}
          aria-pressed={!activeTaskId}
          data-object-id={slotObject?.id}
          onClick={selectSlot}
        >
          <Icon name={slotCategory?.icon ?? "layers"} />
        </button>
        {ongoingObjectTasks.map((task) => {
          const object = demoObjects.find((item) => item.id === task.objectId);
          if (!object) return null;
          const category = objectCategories.find(
            (item) => item.name === object.objectType,
          );
          const label = `${task.title} · ${task.status} · ${task.progress}%`;
          return (
            <button
              type="button"
              key={task.id}
              className={`op-task-rail-item ${task.id === activeTaskId ? "is-active" : ""}`}
              style={{ "--task-color": category?.color } as CSSProperties}
              title={`${object.name}\n${task.id} · ${task.level}\n${label}`}
              aria-label={label}
              aria-pressed={task.id === activeTaskId}
              data-task-id={task.id}
              onClick={() => selectTask(task)}
            >
              <Icon name={category?.icon ?? "tasks"} />
              <span
                className="op-task-rail-progress"
                role="progressbar"
                aria-label={`${task.title}的制作进度`}
                aria-valuemin={0}
                aria-valuemax={100}
                aria-valuenow={task.progress}
              >
                <span style={{ width: `${task.progress}%` }} />
              </span>
            </button>
          );
        })}
      </div>
    </aside>
  );
}
