import { useState } from "react";
import { call } from "../api";
import { ObjectTaskList } from "./ObjectTaskList";
import { ObjectTaskBoard } from "./ObjectTaskBoard";
import "./object-task-board.css";
import { useObjectTasks } from "./use-object-tasks";
import { ObjectTaskDraftEditor } from "./ObjectTaskDraftEditor";
import { ObjectTaskCancellation } from "./object-task-cancellation";
import { ObjectTaskCancellationDialog } from "./ObjectTaskCancellationDialog";
import { ObjectTaskRevision } from "./object-task-revision";
import { ObjectTaskRevisionDialog } from "./ObjectTaskRevisionDialog";
import { ObjectTaskExecution } from "./object-task-execution";
import { ObjectTaskExecutionDialog } from "./ObjectTaskExecutionDialog";
import { ObjectTaskDispatch } from "./object-task-dispatch";
import { ObjectTaskDispatchDialog } from "./ObjectTaskDispatchDialog";
import "./object-tasks.css";
import "./object-task-revisions.css";
import { ObjectTaskPlanningPanel } from "./ObjectTaskPlanningPanel";
import { ObjectTaskQueuePanel } from "./ObjectTaskQueuePanel";

export function ObjectTasksPanel({ projectId }: { projectId: string }) {
  return <ProjectObjectTasksPanel key={projectId} projectId={projectId} />;
}

function ProjectObjectTasksPanel({ projectId }: { projectId: string }) {
  const { state, workspace, refresh } = useObjectTasks(projectId);
  const [cancellation, setCancellation] =
    useState<ObjectTaskCancellation | null>(null);
  const [revision, setRevision] = useState<ObjectTaskRevision | null>(null);
  const [execution, setExecution] = useState<ObjectTaskExecution | null>(null);
  const [dispatch, setDispatch] = useState<ObjectTaskDispatch | null>(null);
  const controlsDisabled =
    state.kind !== "ready" ||
    state.operation !== "idle" ||
    !!revision ||
    !!cancellation ||
    !!execution ||
    !!dispatch;
  return (
    <section
      className="object-tasks-panel"
      aria-label="对象任务记录"
      aria-busy={state.kind === "loading"}
    >
      <header className="object-tasks-toolbar">
        <div>
          <h2>对象任务</h2>
          <p>规划、队列与执行记录；输出保存后等待验收或后续处置。</p>
        </div>
        <button
          onClick={() => {
            void refresh();
          }}
          disabled={state.kind === "loading"}
        >
          刷新
        </button>
      </header>
      {state.kind === "loading" && <p role="status">正在读取对象任务…</p>}
      {state.kind === "error" && (
        <div role="alert">
          <h3>无法读取对象任务</h3>
          <p>{state.message}</p>
        </div>
      )}
      {state.kind === "ready" && (
        <>
          <ObjectTaskPlanningPanel projectId={projectId} state={state} />
          <ObjectTaskDraftEditor state={state} workspace={workspace} />
          <ObjectTaskQueuePanel
            projectId={projectId}
            tasks={state.snapshot.tasks}
            disabled={controlsDisabled}
          />
          <p className="object-tasks-summary">
            计划版本 {state.snapshot.planRevision} ·{" "}
            {state.snapshot.tasks.length} 项任务 · {state.snapshot.runs.length}{" "}
            次迭代
          </p>
          {state.snapshot.tasks.length ? (
            <>
              <ObjectTaskBoard
                snapshot={state.snapshot}
                refresh={refresh}
                disabled={controlsDisabled}
                execute={(task) =>
                  setExecution(
                    new ObjectTaskExecution(
                      projectId,
                      state.snapshot,
                      task.id,
                      call,
                      refresh,
                    ),
                  )
                }
              />
              <details>
                <summary>管理任务：派发、修订与取消</summary>
                <ObjectTaskList
                  snapshot={state.snapshot}
                  cancelDisabled={controlsDisabled}
                  revisionDisabled={controlsDisabled}
                  executionDisabled={controlsDisabled}
                  dispatchDisabled={controlsDisabled}
                  onDispatch={(task) =>
                    setDispatch(
                      new ObjectTaskDispatch(
                        projectId,
                        state.snapshot,
                        task.id,
                        call,
                        refresh,
                      ),
                    )
                  }
                  onExecution={(task) =>
                    setExecution(
                      new ObjectTaskExecution(
                        projectId,
                        state.snapshot,
                        task.id,
                        call,
                        refresh,
                      ),
                    )
                  }
                  onCancel={(task) =>
                    setCancellation(
                      new ObjectTaskCancellation(
                        projectId,
                        state.snapshot,
                        task.id,
                        call,
                        refresh,
                      ),
                    )
                  }
                  onRevise={(task) =>
                    setRevision(
                      new ObjectTaskRevision(
                        projectId,
                        state.snapshot,
                        task.id,
                        call,
                        refresh,
                      ),
                    )
                  }
                />
              </details>
            </>
          ) : (
            <p role="status">
              当前项目尚无已提交的对象任务。保存的草稿不会显示为已提交任务。
            </p>
          )}
        </>
      )}
      {cancellation && (
        <ObjectTaskCancellationDialog
          session={cancellation}
          close={() => setCancellation(null)}
          refresh={refresh}
        />
      )}
      {revision && (
        <ObjectTaskRevisionDialog
          session={revision}
          close={() => setRevision(null)}
        />
      )}
      {execution && (
        <ObjectTaskExecutionDialog
          session={execution}
          close={() => setExecution(null)}
        />
      )}
      {dispatch && (
        <ObjectTaskDispatchDialog
          session={dispatch}
          close={() => setDispatch(null)}
        />
      )}
    </section>
  );
}
