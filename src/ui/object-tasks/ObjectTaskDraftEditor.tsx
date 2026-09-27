import type {
  ObjectTaskPlan,
  ObjectTaskProposal,
} from "../../shared/object-tasks";
import type { ObjectTaskWorkspace } from "./object-task-workspace";
import type { ObjectTaskWorkspaceState } from "./object-task-workspace";
import { ObjectTaskAssumptionsEditor } from "./ObjectTaskAssumptionsEditor";
import { ObjectTaskBaselineEditor } from "./ObjectTaskBaselineEditor";
import { objectChoices, parentTasks } from "./object-task-draft-choices";
import { ObjectTaskTitleSuggestion } from "./ObjectTaskTitleSuggestion";
import { ObjectTaskRequirementField } from "./ObjectTaskRequirementField";
import { ObjectTaskPlanningField } from "./ObjectTaskPlanningField";
import {
  insertDraftTask,
  newDraftItemId,
  reindexTasks,
} from "./object-task-draft-insertion";

type ReadyState = Extract<ObjectTaskWorkspaceState, { kind: "ready" }>;

function updateTask(
  plan: ObjectTaskPlan,
  taskId: string,
  changes: Partial<ObjectTaskProposal>,
): ObjectTaskPlan {
  return {
    ...plan,
    tasks: plan.tasks.map((task) =>
      task.id === taskId ? { ...task, ...changes } : task,
    ),
  };
}

export function ObjectTaskDraftEditor({
  state,
  workspace,
}: {
  state: ReadyState;
  workspace: ObjectTaskWorkspace;
}) {
  const busy = state.operation !== "idle" || Boolean(state.committedRequestId);
  const objects = objectChoices(state);
  return (
    <section
      className="object-task-editor"
      aria-label="对象任务草稿编辑器"
      aria-busy={state.operation !== "idle"}
    >
      <header className="object-task-editor-heading">
        <div>
          <h3>规划草稿</h3>
          <p>
            计划版本 {state.planRevision} · 草稿版本 {state.draftRevision}
          </p>
        </div>
        {state.operation !== "idle" && (
          <span role="status">
            {state.operation === "saving"
              ? "正在保存…"
              : state.operation === "committing"
                ? "正在提交规划…"
                : "正在同步…"}
          </span>
        )}
      </header>
      {state.committedRequestId && (
        <div role="status">
          <p>
            草稿已提交并锁定，重开后仍为只读。已提交任务的修改请使用任务列表中的定义修订。
          </p>
          <p>开始下一份草稿将清空编辑区；已提交任务及历史记录会保留。</p>
          <button
            disabled={state.operation !== "idle"}
            onClick={() => void workspace.unlockDraft()}
          >
            开始下一份草稿
          </button>
          <button
            disabled={state.operation !== "idle"}
            onClick={() => void workspace.reloadDiscardingLocal()}
          >
            重新加载提交状态
          </button>
        </div>
      )}
      {state.conflict && (
        <div className="object-task-conflict" role="alert">
          <p>
            {state.conflict === "draft"
              ? "远端草稿版本已变化；本地编辑仍保留。可基于最新草稿版本重试，或丢弃本地更改并重新加载。"
              : `计划版本已变化。请核对下方计划版本 ${state.snapshot.planRevision} 的最新任务，再保留本地内容并采用此版本；已撤销的父任务或依赖需在草稿中调整。也可丢弃当前草稿。`}
          </p>
          <button
            disabled={busy}
            onClick={() => {
              void workspace.keepLocalDraftAndRebaseRevision();
            }}
          >
            {state.conflict === "plan"
              ? `保留本地内容并采用计划版本 ${state.snapshot.planRevision}`
              : "保留本地内容并重试"}
          </button>
          {state.conflict === "plan" && (
            <button disabled={busy} onClick={() => workspace.startNewPlan()}>
              丢弃当前草稿并从最新计划开始
            </button>
          )}
          <button
            disabled={busy}
            onClick={() => {
              void workspace.reloadDiscardingLocal();
            }}
          >
            丢弃本地更改并重新加载
          </button>
        </div>
      )}
      {state.error && <p role="alert">{state.error}</p>}
      <div className="object-task-editor-actions">
        <button
          disabled={busy}
          onClick={() => {
            workspace.updatePlan({
              ...state.plan,
              objects: [
                ...state.plan.objects,
                { id: newDraftItemId("object"), name: "", category: "其他" },
              ],
            });
          }}
        >
          添加对象
        </button>
        {(["coarse", "medium", "fine"] as const).map((granularity) => (
          <button
            key={granularity}
            disabled={busy}
            onClick={() =>
              workspace.updatePlan((plan) =>
                insertDraftTask(plan, { edge: "end" }, granularity),
              )
            }
          >
            添加
            {granularity === "coarse"
              ? "粗修"
              : granularity === "medium"
                ? "中修"
                : "精修"}
          </button>
        ))}
        <button
          disabled={busy}
          onClick={() =>
            workspace.updatePlan((plan) =>
              insertDraftTask(plan, { edge: "start" }),
            )
          }
        >
          任务首部插入
        </button>
        <button
          disabled={busy}
          onClick={() =>
            workspace.updatePlan((plan) =>
              insertDraftTask(plan, { edge: "end" }),
            )
          }
        >
          任务尾部插入
        </button>
      </div>
      <ObjectTaskAssumptionsEditor
        plan={state.plan}
        history={state.snapshot.assumptions}
        disabled={busy}
        onPlanChange={(plan) => workspace.updatePlan(plan)}
      />
      <fieldset disabled={busy} className="object-task-editor-fields">
        <legend>对象</legend>
        {state.plan.objects.map((object) => (
          <div className="object-task-editor-row" key={object.id}>
            <code>{object.id}</code>
            <label>
              名称
              <input
                value={object.name}
                onChange={(event) =>
                  workspace.updatePlan({
                    ...state.plan,
                    objects: state.plan.objects.map((candidate) =>
                      candidate.id === object.id
                        ? { ...candidate, name: event.currentTarget.value }
                        : candidate,
                    ),
                  })
                }
              />
            </label>
            <label>
              类别
              <input
                value={object.category}
                onChange={(event) =>
                  workspace.updatePlan({
                    ...state.plan,
                    objects: state.plan.objects.map((candidate) =>
                      candidate.id === object.id
                        ? { ...candidate, category: event.currentTarget.value }
                        : candidate,
                    ),
                  })
                }
              />
            </label>
            <button
              aria-label={`删除对象 ${object.name || object.id}`}
              onClick={() =>
                workspace.updatePlan({
                  ...state.plan,
                  objects: state.plan.objects.filter(
                    (candidate) => candidate.id !== object.id,
                  ),
                })
              }
            >
              删除对象
            </button>
          </div>
        ))}
      </fieldset>
      <fieldset disabled={busy} className="object-task-editor-fields">
        <legend>任务</legend>
        {state.plan.tasks.map((task, index) => (
          <article className="object-task-editor-task" key={task.id}>
            <header>
              <code>{task.id}</code>
              <button
                aria-label={`在任务 ${task.title || index + 1} 前插入`}
                onClick={() =>
                  workspace.updatePlan((plan) =>
                    insertDraftTask(plan, { taskId: task.id, side: "before" }),
                  )
                }
              >
                前插入
              </button>
              <button
                aria-label={`在任务 ${task.title || index + 1} 后插入`}
                onClick={() =>
                  workspace.updatePlan((plan) =>
                    insertDraftTask(plan, { taskId: task.id, side: "after" }),
                  )
                }
              >
                后插入
              </button>
              <label>
                层级
                <select
                  value={task.granularity}
                  onChange={(event) =>
                    workspace.updatePlan(
                      updateTask(state.plan, task.id, {
                        granularity: event.currentTarget
                          .value as ObjectTaskProposal["granularity"],
                        baseline:
                          event.currentTarget.value === "medium"
                            ? task.baseline
                            : undefined,
                      }),
                    )
                  }
                >
                  <option value="coarse">粗修</option>
                  <option value="medium">中修</option>
                  <option value="fine" disabled={Boolean(task.pendingPlanning)}>
                    精修{task.pendingPlanning ? "（先处理待规划事项）" : ""}
                  </option>
                </select>
              </label>
              <button
                aria-label={`删除任务 ${task.title || index + 1}`}
                onClick={() =>
                  workspace.updatePlan({
                    ...state.plan,
                    tasks: reindexTasks(
                      state.plan.tasks.filter(
                        (candidate) => candidate.id !== task.id,
                      ),
                    ),
                  })
                }
              >
                删除任务
              </button>
            </header>
            <label>
              标题
              <input
                value={task.title}
                onChange={(event) =>
                  workspace.updatePlan(
                    updateTask(state.plan, task.id, {
                      title: event.currentTarget.value,
                    }),
                  )
                }
              />
            </label>
            <ObjectTaskTitleSuggestion
              workspace={workspace}
              taskId={task.id}
              disabled={busy}
            />
            <ObjectTaskRequirementField
              value={task.requirement}
              onChange={(requirement) =>
                workspace.updatePlan(
                  updateTask(state.plan, task.id, { requirement }),
                )
              }
            />
            {task.granularity !== "fine" && (
              <ObjectTaskPlanningField
                value={task.pendingPlanning}
                onChange={(pendingPlanning) =>
                  workspace.updatePlan(
                    updateTask(state.plan, task.id, { pendingPlanning }),
                  )
                }
              />
            )}
            <label>
              任务说明
              <textarea
                value={task.prompt}
                onChange={(event) =>
                  workspace.updatePlan(
                    updateTask(state.plan, task.id, {
                      prompt: event.currentTarget.value,
                    }),
                  )
                }
              />
            </label>
            <label>
              验收条件
              <textarea
                value={task.acceptance}
                onChange={(event) =>
                  workspace.updatePlan(
                    updateTask(state.plan, task.id, {
                      acceptance: event.currentTarget.value,
                    }),
                  )
                }
              />
            </label>
            <label>
              对象 ID
              <select
                value={task.objectId ?? ""}
                onChange={(event) =>
                  workspace.updatePlan(
                    updateTask(state.plan, task.id, {
                      objectId: event.currentTarget.value || null,
                    }),
                  )
                }
              >
                <option value="">未绑定对象</option>
                {objects.map((object) => (
                  <option key={object.id} value={object.id}>
                    {object.name || object.id}
                  </option>
                ))}
              </select>
            </label>
            {task.granularity === "medium" && (
              <ObjectTaskBaselineEditor
                baseline={task.baseline}
                onChange={(baseline) =>
                  workspace.updatePlan(
                    updateTask(state.plan, task.id, { baseline }),
                  )
                }
              />
            )}
            <label>
              责任父任务
              <select
                value={task.parentTaskId ?? ""}
                onChange={(event) =>
                  workspace.updatePlan(
                    updateTask(state.plan, task.id, {
                      parentTaskId: event.currentTarget.value || null,
                    }),
                  )
                }
              >
                <option value="">无</option>
                {parentTasks(state, task).map((parent) => (
                  <option
                    key={parent.id}
                    value={parent.id}
                    disabled={parent.disabled}
                  >
                    {parent.title || parent.id}
                  </option>
                ))}
              </select>
            </label>
            {task.granularity === "fine" && (
              <label>
                阶段 ID
                <input
                  value={task.stageId ?? ""}
                  onChange={(event) =>
                    workspace.updatePlan(
                      updateTask(state.plan, task.id, {
                        stageId: event.currentTarget.value || null,
                      }),
                    )
                  }
                />
              </label>
            )}
            <label>
              依赖任务 ID（逗号分隔）
              <input
                value={task.dependsOn.join(", ")}
                onChange={(event) =>
                  workspace.updatePlan(
                    updateTask(state.plan, task.id, {
                      dependsOn: event.currentTarget.value
                        .split(",")
                        .map((value) => value.trim())
                        .filter(Boolean),
                    }),
                  )
                }
              />
            </label>
          </article>
        ))}
      </fieldset>
      <footer className="object-task-editor-actions">
        <button
          disabled={busy || !state.dirty || Boolean(state.conflict)}
          onClick={() => {
            void workspace.save();
          }}
        >
          保存草稿
        </button>
        <button
          disabled={
            busy ||
            state.dirty ||
            Boolean(state.conflict) ||
            Boolean(state.receipt) ||
            state.plan.tasks.length === 0
          }
          onClick={() => {
            void workspace.commit();
          }}
        >
          提交到任务列表（不执行）
        </button>
      </footer>
    </section>
  );
}
