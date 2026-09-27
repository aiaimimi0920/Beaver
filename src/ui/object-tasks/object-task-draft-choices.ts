import type { ObjectTaskProposal } from "../../shared/object-tasks";
import type { ObjectTaskWorkspaceState } from "./object-task-workspace";

type ReadyState = Extract<ObjectTaskWorkspaceState, { kind: "ready" }>;

export function parentTasks(state: ReadyState, task: ObjectTaskProposal) {
  const parentGranularity =
    task.granularity === "fine"
      ? "medium"
      : task.granularity === "medium"
        ? "coarse"
        : null;
  const cancelled = new Set(
    state.snapshot.tasks
      .filter((candidate) => candidate.status === "cancelled")
      .map((candidate) => candidate.id),
  );
  const planned = state.plan.tasks.filter(
    (candidate) =>
      candidate.granularity === parentGranularity &&
      !cancelled.has(candidate.id),
  );
  const plannedIds = new Set(planned.map((candidate) => candidate.id));
  const committed = state.snapshot.tasks
    .filter(
      (candidate) =>
        candidate.granularity === parentGranularity &&
        candidate.status === "planned" &&
        !plannedIds.has(candidate.id),
    )
    .map((candidate) => ({
      id: candidate.id,
      title: `${candidate.title}（已提交）`,
    }));
  const choices = [...planned, ...committed].map(({ id, title }) => ({
    id,
    title,
    disabled: false,
  }));
  if (
    task.parentTaskId &&
    !choices.some((parent) => parent.id === task.parentTaskId)
  ) {
    choices.push({
      id: task.parentTaskId,
      title: `${task.parentTaskId}（${cancelled.has(task.parentTaskId) ? "已撤销" : "不可用"}，请更换父任务）`,
      disabled: true,
    });
  }
  return choices;
}

export function objectChoices(state: ReadyState) {
  const planned = state.plan.objects.map(({ id, name }) => ({ id, name }));
  const known = new Set(planned.map(({ id }) => id));
  const committed = state.snapshot.tasks.flatMap((task) =>
    task.objectId && !known.has(task.objectId)
      ? [{ id: task.objectId, name: `${task.objectId}（已提交）` }]
      : [],
  );
  return [...planned, ...committed];
}
