import type { ObjectTaskDefinition } from "../../shared/object-task-revisions";
import type { ObjectTaskSnapshot } from "../../shared/object-tasks";
import { ObjectTaskRequirementField } from "./ObjectTaskRequirementField";
import { ObjectTaskPlanningField } from "./ObjectTaskPlanningField";

export function ObjectTaskDefinitionEditor({
  snapshot,
  taskId,
  definition,
  reason,
  onDefinition,
  onReason,
}: {
  snapshot: ObjectTaskSnapshot;
  taskId: string;
  definition: ObjectTaskDefinition;
  reason: string;
  onDefinition: (patch: Partial<ObjectTaskDefinition>) => void;
  onReason: (reason: string) => void;
}) {
  const choices = snapshot.tasks.filter((task) => task.id !== taskId);
  const unavailable = definition.dependsOn.filter(
    (id) => !choices.some((task) => task.id === id),
  );
  const toggle = (id: string, checked: boolean) =>
    onDefinition({
      dependsOn: checked
        ? [...definition.dependsOn, id]
        : definition.dependsOn.filter((dependency) => dependency !== id),
    });
  return (
    <fieldset className="object-task-definition-editor">
      <legend>待提交的任务定义</legend>
      {snapshot.tasks.some(
        (task) => task.id === taskId && task.granularity !== "fine",
      ) && (
        <ObjectTaskPlanningField
          value={definition.pendingPlanning}
          onChange={(pendingPlanning) => onDefinition({ pendingPlanning })}
        />
      )}
      <ObjectTaskRequirementField
        value={definition.requirement}
        onChange={(requirement) => onDefinition({ requirement })}
      />
      <label>
        标题
        <input
          value={definition.title}
          maxLength={300}
          required
          onChange={(event) => onDefinition({ title: event.target.value })}
        />
      </label>
      <label>
        任务目标
        <textarea
          value={definition.prompt}
          maxLength={20_000}
          required
          onChange={(event) => onDefinition({ prompt: event.target.value })}
        />
      </label>
      <label>
        验收要求
        <textarea
          value={definition.acceptance}
          maxLength={10_000}
          onChange={(event) => onDefinition({ acceptance: event.target.value })}
        />
      </label>
      <fieldset className="object-task-revision-dependencies">
        <legend>显式依赖</legend>
        {choices.map((task) => {
          const checked = definition.dependsOn.includes(task.id);
          return (
            <label key={task.id}>
              <input
                type="checkbox"
                checked={checked}
                disabled={task.status === "cancelled" && !checked}
                onChange={(event) => toggle(task.id, event.target.checked)}
              />
              <span>
                {task.title}（<code>{task.id}</code>）
                {task.status === "cancelled" && "（已撤销，请移除）"}
              </span>
            </label>
          );
        })}
        {unavailable.map((id) => (
          <label key={id}>
            <input type="checkbox" checked onChange={() => toggle(id, false)} />
            <span>
              <code>{id}</code>（不可用，请移除）
            </span>
          </label>
        ))}
        {!choices.length && !unavailable.length && <p>当前没有可选依赖。</p>}
      </fieldset>
      <label>
        修订原因
        <textarea
          value={reason}
          maxLength={2_000}
          required
          onChange={(event) => onReason(event.target.value)}
        />
      </label>
      <p>
        上限按 UTF-8 字节计算：标题 300、目标 20,000、验收要求 10,000、原因
        2,000。 修订由项目所有者确认后保存。
      </p>
    </fieldset>
  );
}
