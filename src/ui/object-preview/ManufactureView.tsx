import { useState, type ComponentProps, type CSSProperties } from "react";
import { Icon } from "../Icon";
import { CreationView } from "./CreationView";
import { objectById } from "./mock-objects";
import type { DemoTask } from "./mock-tasks";
import { objectCategories } from "./object-categories";
import { ongoingObjectTasks } from "./useObjectTaskNavigation";
import "./preview-object-task-rail.css";
import "./preview-manufacture.css";

export function ManufactureView(
  props: ComponentProps<typeof CreationView> & {
    openCreation: (id: string, stage?: number, version?: string) => void;
  },
) {
  const [selectedTask, setSelectedTask] = useState<DemoTask | null>(null);
  const activeTask =
    selectedTask?.objectId === props.object.id &&
    selectedTask.stage === props.initialStage
      ? selectedTask
      : ongoingObjectTasks.find(
          (task) =>
            task.objectId === props.object.id &&
            (props.initialStage === undefined
              ? task.id === props.object.taskId
              : task.stage === props.initialStage),
        );

  function selectTask(task: DemoTask) {
    if (!task.objectId) return;
    setSelectedTask(task);
    props.openCreation(task.objectId, task.stage);
  }

  return (
    <div className="op-manufacture-workspace">
      <CreationView
        key={`${props.object.id}:${activeTask?.id ?? props.initialStage}:${props.initialVersion}`}
        {...props}
        taskTitle={activeTask?.title}
      />
      <aside className="op-manufacture-tasks" aria-label="尚未完成的制造任务">
        <div className="op-task-rail-list" id="op-manufacture-task-list">
          {ongoingObjectTasks.map((task) => {
            const object = objectById(task.objectId!);
            const category = objectCategories.find(
              (item) => item.name === object.objectType,
            );
            const label = `${task.title} · ${task.progress}%`;
            return (
              <button
                type="button"
                key={task.id}
                className={`op-task-rail-item${activeTask?.id === task.id ? " is-active" : ""}`}
                style={{ "--task-color": category?.color } as CSSProperties}
                title={`${object.name}\n${task.id} · ${task.status}\n${label}`}
                aria-label={label}
                aria-pressed={activeTask?.id === task.id}
                onClick={() => selectTask(task)}
              >
                <span className="op-manufacture-task-icon">
                  <Icon name={category?.icon ?? "tasks"} />
                  <span
                    className="op-task-rail-progress"
                    role="progressbar"
                    aria-label={`${task.title}的制造进度`}
                    aria-valuemin={0}
                    aria-valuemax={100}
                    aria-valuenow={task.progress}
                  >
                    <span style={{ width: `${task.progress}%` }} />
                  </span>
                </span>
              </button>
            );
          })}
        </div>
      </aside>
    </div>
  );
}
