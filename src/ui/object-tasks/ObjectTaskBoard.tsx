import type { ReactNode } from "react";
import { ObjectTaskPlanningDeclaration } from "./ObjectTaskPlanningDeclaration";
import type {
  ObjectTaskRecord,
  ObjectTaskSnapshot,
} from "../../shared/object-tasks";
import { objectTaskStatusLabels } from "./object-task-status";
import { objectTaskProgress } from "./object-task-progress";
import { objectTaskRequirementLabels } from "./ObjectTaskRequirementField";

const lanes: { title: string; statuses: ObjectTaskRecord["status"][] }[] = [
  { title: "等待开始", statuses: ["planned", "queued"] },
  { title: "执行中", statuses: ["running"] },
  { title: "需要关注", statuses: ["awaitingAcceptance", "failed"] },
  { title: "已接受", statuses: ["accepted"] },
  { title: "已撤销", statuses: ["cancelled"] },
];
const levels = { coarse: "粗修", medium: "中修", fine: "精修" };

export function ObjectTaskBoard({
  snapshot,
  disabled,
  execute,
  refresh,
}: {
  snapshot: ObjectTaskSnapshot;
  disabled: boolean;
  execute: (task: ObjectTaskRecord) => void;
  refresh?: () => Promise<unknown>;
}) {
  const tasks = [...snapshot.tasks].sort(
    (a, b) => (a.position ?? 0) - (b.position ?? 0) || a.id.localeCompare(b.id),
  );
  const byId = new Map(tasks.map((task) => [task.id, task]));
  const children = new Map<string, ObjectTaskRecord[]>();
  for (const task of tasks) {
    if (!task.parentTaskId) continue;
    const siblings = children.get(task.parentTaskId) ?? [];
    siblings.push(task);
    children.set(task.parentTaskId, siblings);
  }
  function history(task: ObjectTaskRecord) {
    const medium =
      task.granularity === "medium"
        ? task
        : task.granularity === "fine" && task.parentTaskId
          ? byId.get(task.parentTaskId)
          : undefined;
    if (!medium || medium.granularity !== "medium") return null;
    return (
      <button disabled={disabled} onClick={() => execute(medium)}>
        查看迭代执行记录
      </button>
    );
  }
  function descendants(task: ObjectTaskRecord): ReactNode {
    const items = children.get(task.id) ?? [];
    if (!items.length) return null;
    return (
      <details className="object-board-children">
        <summary>展开子任务（{items.length}）</summary>
        <ul>
          {items.map((child) => (
            <li key={child.id}>
              <p>
                {levels[child.granularity]} · {child.title} ·{" "}
                {objectTaskStatusLabels[child.status]}
              </p>
              <small>{child.id}</small>
              {history(child)}
              {descendants(child)}
            </li>
          ))}
        </ul>
      </details>
    );
  }
  return (
    <section className="object-task-board" aria-label="全层级对象任务泳道">
      <h3>任务泳道 · {tasks.length} 项</h3>
      <p>
        粗修、中修、精修共用持久身份；责任关系与依赖分别展示。已接受子项不代表父任务整体验收完成。
      </p>
      {!tasks.length && <p role="status">尚无已提交任务；草稿不计入进度。</p>}
      <div className="object-task-lanes">
        {lanes.map((lane) => {
          const items = tasks.filter((task) =>
            lane.statuses.includes(task.status),
          );
          return (
            <section
              className="object-task-lane"
              key={lane.title}
              aria-label={lane.title}
            >
              <h4>
                {lane.title} · {items.length}
              </h4>
              {!items.length && <p>暂无任务</p>}
              {items.map((task) => {
                const parent = task.parentTaskId
                  ? byId.get(task.parentTaskId)
                  : undefined;
                return (
                  <article
                    className="object-task-record"
                    key={task.id}
                    aria-label={`${levels[task.granularity]} ${task.title}`}
                  >
                    <span className="object-task-layer">
                      {levels[task.granularity]}
                    </span>
                    <h4>{task.title}</h4>
                    <small>{task.id}</small>
                    <p>{objectTaskStatusLabels[task.status]}</p>
                    <p>{objectTaskRequirementLabels[task.requirement]}</p>
                    {parent && (
                      <p>
                        责任父任务：{parent.title}（{parent.id}）
                      </p>
                    )}
                    {!parent && task.granularity === "medium" && (
                      <p>独立修改</p>
                    )}
                    {task.objectId && (
                      <p>
                        对象 {task.objectId} · 迭代 {task.runId}
                      </p>
                    )}
                    {objectTaskProgress(task, children).map((message) => (
                      <p key={message}>{message}</p>
                    ))}
                    {refresh &&
                      snapshot.planningStates
                        ?.filter((state) => state.taskId === task.id)
                        .map((state) => (
                          <ObjectTaskPlanningDeclaration
                            key={task.id}
                            projectId={task.projectId}
                            state={state}
                            planRevision={snapshot.planRevision}
                            disabled={disabled}
                            refresh={refresh}
                          />
                        ))}
                    <details>
                      <summary>需求与依赖</summary>
                      <p className="object-task-prompt">{task.prompt}</p>
                      <p>验收要求：{task.acceptance || "尚未填写"}</p>
                      <p>
                        依赖：
                        {task.dependsOn
                          .map((id) => byId.get(id)?.title ?? id)
                          .join("、") || "无"}
                      </p>
                    </details>
                    {history(task)}
                    {descendants(task)}
                  </article>
                );
              })}
            </section>
          );
        })}
      </div>
    </section>
  );
}
