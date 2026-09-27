import type { ReactNode } from "react";
import { requireObjectTaskRevisionEligibility } from "../../shared/object-task-revision-plan";
import type {
  ObjectTaskRecord,
  ObjectTaskSnapshot,
} from "../../shared/object-tasks";
import { objectTaskStatusLabels } from "./object-task-status";

const labels = { coarse: "粗修", medium: "中修", fine: "精修" } as const;

function canRevise(snapshot: ObjectTaskSnapshot, taskId: string): boolean {
  try {
    requireObjectTaskRevisionEligibility(snapshot, taskId);
    return true;
  } catch {
    return false;
  }
}

export function ObjectTaskList({
  snapshot,
  onCancel,
  cancelDisabled = false,
  onRevise,
  revisionDisabled = false,
  onExecution,
  executionDisabled = false,
  onDispatch,
  dispatchDisabled = false,
}: {
  snapshot: ObjectTaskSnapshot;
  onCancel?: (task: ObjectTaskRecord) => void;
  cancelDisabled?: boolean;
  onRevise?: (task: ObjectTaskRecord) => void;
  revisionDisabled?: boolean;
  onExecution?: (task: ObjectTaskRecord) => void;
  executionDisabled?: boolean;
  onDispatch?: (task: ObjectTaskRecord) => void;
  dispatchDisabled?: boolean;
}) {
  const children = new Map<string | null, ObjectTaskRecord[]>();
  for (const task of snapshot.tasks) {
    const siblings = children.get(task.parentTaskId) ?? [];
    siblings.push(task);
    children.set(task.parentTaskId, siblings);
  }
  const runs = new Map(snapshot.runs.map((run) => [run.id, run]));
  const records = new Map(snapshot.tasks.map((task) => [task.id, task]));
  const dispatch = new Map(
    [...snapshot.dispatchControls, ...snapshot.coarseDispatchControls].map(
      (control) => [control.taskId, control],
    ),
  );
  function rows(parentId: string | null): ReactNode {
    const tasks = children.get(parentId);
    if (!tasks?.length) return null;
    return (
      <ol className="object-task-tree">
        {tasks.map((task) => {
          const run = task.runId ? runs.get(task.runId) : undefined;
          const editable = onRevise && canRevise(snapshot, task.id);
          return (
            <li key={task.id}>
              <article className="object-task-record">
                <header>
                  <span className="object-task-layer">
                    {labels[task.granularity]}
                  </span>
                  <h3>{task.title}</h3>
                  <span className="object-task-status">
                    {objectTaskStatusLabels[task.status]}
                  </span>
                  {dispatch.get(task.id)?.paused && (
                    <span className="object-task-status">派发已暂停</span>
                  )}
                  {task.granularity === "medium" &&
                    task.parentTaskId &&
                    dispatch.get(task.parentTaskId)?.paused && (
                      <span className="object-task-status">
                        所属粗修已暂停派发
                      </span>
                    )}
                  {task.granularity !== "fine" &&
                    task.status !== "cancelled" &&
                    onDispatch && (
                      <button
                        aria-label={`管理任务 ${task.title} 的派发`}
                        disabled={dispatchDisabled}
                        onClick={() => onDispatch(task)}
                      >
                        派发控制
                      </button>
                    )}
                  {task.granularity === "medium" &&
                    task.runId &&
                    onExecution && (
                      <button
                        aria-label={`查看任务 ${task.title} 的执行记录`}
                        disabled={executionDisabled}
                        onClick={() => onExecution(task)}
                      >
                        执行记录
                      </button>
                    )}
                  {onRevise && (
                    <button
                      aria-label={
                        editable
                          ? `修订任务 ${task.title} 的定义`
                          : `查看任务 ${task.title} 的修订历史`
                      }
                      disabled={revisionDisabled}
                      onClick={() => onRevise(task)}
                    >
                      {editable ? "修订定义" : "修订历史"}
                    </button>
                  )}
                  {task.status === "planned" && onCancel && (
                    <button
                      aria-label={`撤销任务 ${task.title}`}
                      disabled={cancelDisabled}
                      onClick={() => onCancel(task)}
                    >
                      撤销规划
                    </button>
                  )}
                </header>
                <dl>
                  <div>
                    <dt>任务 ID</dt>
                    <dd>
                      <code>{task.id}</code>
                    </dd>
                  </div>
                  <div>
                    <dt>任务版本</dt>
                    <dd>{task.revision}</dd>
                  </div>
                  <div>
                    <dt>对象</dt>
                    <dd>
                      {task.objectId ? <code>{task.objectId}</code> : "项目级"}
                    </dd>
                  </div>
                  <div>
                    <dt>责任父任务</dt>
                    <dd>
                      {task.parentTaskId ? (
                        <code>{task.parentTaskId}</code>
                      ) : (
                        "无"
                      )}
                    </dd>
                  </div>
                  {task.runId && (
                    <div>
                      <dt>Run ID</dt>
                      <dd>
                        <code>{task.runId}</code>
                      </dd>
                    </div>
                  )}
                  {task.stageId && (
                    <div>
                      <dt>阶段 ID</dt>
                      <dd>
                        <code>{task.stageId}</code>
                      </dd>
                    </div>
                  )}
                  {run && (
                    <div>
                      <dt>迭代状态</dt>
                      <dd>{objectTaskStatusLabels[run.status]}</dd>
                    </div>
                  )}
                  {run && (
                    <div>
                      <dt>基准版本</dt>
                      <dd>
                        {run.baselineVersionId ? (
                          <code>{run.baselineVersionId}</code>
                        ) : run.status === "cancelled" ? (
                          "未记录基准（已撤销）"
                        ) : (
                          "待执行时确定"
                        )}
                      </dd>
                    </div>
                  )}
                </dl>
                <p className="object-task-prompt">{task.prompt}</p>
                <p>验收条件：{task.acceptance || "尚未填写"}</p>
                <p>
                  依赖：
                  {task.dependsOn.length
                    ? task.dependsOn.map((id) => (
                        <code className="object-task-dependency" key={id}>
                          {id}
                          {records.get(id)?.status === "cancelled" &&
                            "（已撤销）"}
                        </code>
                      ))
                    : "无显式依赖"}
                </p>
              </article>
              {rows(task.id)}
            </li>
          );
        })}
      </ol>
    );
  }
  return rows(null);
}
