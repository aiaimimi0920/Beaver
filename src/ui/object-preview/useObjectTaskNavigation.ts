import { useEffect, useState } from "react";
import { objectById, type DemoObject } from "./mock-objects";
import { demoTasks, type DemoTask } from "./mock-tasks";

export const ongoingObjectTasks = demoTasks.filter(
  (task) =>
    task.objectId && task.status !== "已验收" && task.status !== "未绑定任务",
);

function taskForObject(object: DemoObject) {
  const tasks = ongoingObjectTasks.filter(
    (task) => task.objectId === object.id,
  );
  return tasks.find((task) => task.id === object.taskId) ?? tasks[0] ?? null;
}

export function useObjectTaskNavigation(
  selected: DemoObject,
  select: (id: string) => void,
) {
  const [route, setRoute] = useState(() => ({
    objectId: selected.id,
    taskId: taskForObject(selected)?.id ?? null,
  }));
  const [slotObject, setSlotObject] = useState<DemoObject | null>(() =>
    taskForObject(selected) ? null : selected,
  );

  useEffect(() => {
    if (route.objectId === selected.id) return;
    const task = taskForObject(selected);
    setRoute({ objectId: selected.id, taskId: task?.id ?? null });
    if (!task) setSlotObject(selected);
  }, [selected, route.objectId]);

  function selectObject(id: string) {
    const object = objectById(id);
    const task = taskForObject(object);
    setRoute({ objectId: id, taskId: task?.id ?? null });
    if (!task) setSlotObject(object);
    select(id);
  }

  function selectTask(task: DemoTask) {
    if (!task.objectId) return;
    setRoute({ objectId: task.objectId, taskId: task.id });
    select(task.objectId);
  }

  function selectSlot() {
    const objectId = slotObject?.id ?? selected.id;
    setRoute({ objectId, taskId: null });
    if (slotObject) select(objectId);
  }

  const activeTask =
    ongoingObjectTasks.find((task) => task.id === route.taskId) ?? null;
  return {
    activeTask,
    slotObject,
    inspectorObject: activeTask?.objectId
      ? objectById(activeTask.objectId)
      : slotObject,
    selectObject,
    selectTask,
    selectSlot,
  };
}
