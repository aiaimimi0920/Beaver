import { useEffect, useSyncExternalStore } from "react";
import {
  objectTaskRevisionImpact,
  revisionTarget,
} from "../../shared/object-task-revision-plan";
import { objectTaskDefinition } from "../../shared/object-task-revisions";
import { Dialog } from "../components";
import {
  ObjectTaskDefinitionComparison,
  ObjectTaskDefinitionView,
} from "./ObjectTaskDefinitionComparison";
import { ObjectTaskDefinitionEditor } from "./ObjectTaskDefinitionEditor";
import { ObjectTaskRevisionHistory } from "./ObjectTaskRevisionHistory";
import { ObjectTaskPromptHistory } from "./ObjectTaskPromptHistory";
import type { ObjectTaskRevision } from "./object-task-revision";
import { objectTaskStatusLabels } from "./object-task-status";

export function ObjectTaskRevisionDialog({
  session,
  close,
}: {
  session: ObjectTaskRevision;
  close: () => void;
}) {
  const state = useSyncExternalStore(
    session.subscribe,
    session.getSnapshot,
    session.getSnapshot,
  );
  useEffect(() => {
    void session.history.load();
    return () => session.cancel();
  }, [session]);
  const dismiss = () => {
    session.cancel();
    close();
  };
  const task = revisionTarget(state.snapshot, session.taskId);
  const latestTask = state.latest?.tasks.find(
    (candidate) => candidate.id === session.taskId,
  );
  const affected = new Set(
    objectTaskRevisionImpact(state.snapshot, session.taskId),
  );
  return (
    <Dialog
      title="任务定义与修订历史"
      close={dismiss}
      className="object-task-revision-dialog"
    >
      <p>
        目标：{task.title}（<code>{task.id}</code>）
      </p>
      <p>
        已核对计划版本 {state.snapshot.planRevision} · 任务版本 {task.revision}
      </p>
      {state.phase === "editing" ? (
        <>
          {task.granularity === "fine" && (
            <ObjectTaskPromptHistory session={session} />
          )}
          <ObjectTaskDefinitionEditor
            snapshot={state.snapshot}
            taskId={task.id}
            definition={state.definition}
            reason={state.reason}
            onDefinition={(patch) => session.updateDefinition(patch)}
            onReason={(reason) => session.updateReason(reason)}
          />
        </>
      ) : state.phase === "readonly" ? (
        <ObjectTaskDefinitionView
          label="当前任务定义"
          definition={objectTaskDefinition(task)}
        />
      ) : (
        <>
          <ObjectTaskDefinitionComparison
            before={objectTaskDefinition(task)}
            after={state.definition}
          />
          <p className="object-task-revision-reason">
            修订原因：{state.reason}
          </p>
          <p>采纳者：项目所有者（owner）</p>
        </>
      )}
      {state.phase !== "readonly" && (
        <section
          className="object-task-revision-impact"
          aria-label="本次修订影响范围"
        >
          <h3>影响范围：{affected.size} 项任务</h3>
          <p>
            包括本任务、责任子任务，以及沿依赖关系受影响的任务。提交后请重新核对这些任务的规划。
          </p>
          <ul>
            {state.snapshot.tasks
              .filter((candidate) => affected.has(candidate.id))
              .map((candidate) => (
                <li key={candidate.id}>
                  {candidate.title}（<code>{candidate.id}</code>）
                </li>
              ))}
          </ul>
        </section>
      )}
      {state.error && <p role="alert">{state.error}</p>}
      {state.promptSource && (
        <p role="status">
          {state.promptSource}。内容可继续编辑，来源将随修订原因保存。
        </p>
      )}
      {state.phase === "reviewing" && (
        <p role="status">请核对修订前后定义、原因和影响范围，再确认保存。</p>
      )}
      {state.phase === "submitting" && (
        <p role="status">正在提交修订；关闭窗口后请求仍可能完成。</p>
      )}
      {state.phase === "failed" && (
        <p>尚未确认提交结果。重试会使用完整的原请求；关闭后可刷新列表核对。</p>
      )}
      {state.phase === "conflict" && (
        <section className="object-task-conflict" aria-label="任务修订版本冲突">
          <h3>远端计划已变化，本地输入已保留</h3>
          <p>
            请比较上方原定义、本地修订和下方最新定义。采用新版本后还需重新核对并确认。
          </p>
          {state.latestLoading && <p role="status">正在读取最新任务定义…</p>}
          {state.latestError && <p role="alert">{state.latestError}</p>}
          {state.latest && (
            <>
              <p>
                最新计划版本 {state.latest.planRevision} · 任务版本{" "}
                {latestTask?.revision ?? "任务已不存在"}
              </p>
              {latestTask && (
                <>
                  <p>
                    任务状态：
                    {objectTaskStatusLabels[latestTask.status]}
                  </p>
                  <ObjectTaskDefinitionView
                    label="最新远端定义"
                    definition={objectTaskDefinition(latestTask)}
                  />
                </>
              )}
            </>
          )}
          <div className="object-task-editor-actions">
            <button
              disabled={state.latestLoading}
              onClick={() => {
                void session.loadLatest();
              }}
            >
              重新读取最新计划
            </button>
            <button
              disabled={state.latestLoading || !state.latest}
              onClick={() => session.rebase()}
            >
              保留本地修改并采用最新版本
            </button>
          </div>
        </section>
      )}
      {state.refreshing && <p role="status">正在刷新任务列表…</p>}
      {state.receipt && (
        <p role="status">
          修订已确认，任务版本 {state.receipt.taskRevision} · 计划版本{" "}
          {state.receipt.planRevision}
          。本地规划草稿保留；如有冲突，请在草稿编辑器中核对。
        </p>
      )}
      <footer className="object-task-editor-actions">
        {state.phase === "editing" && (
          <button onClick={() => session.review()}>核对修订</button>
        )}
        {state.phase === "reviewing" && (
          <>
            <button
              onClick={() => {
                void session.submit();
              }}
            >
              确认修订
            </button>
            <button onClick={session.edit}>返回编辑</button>
          </>
        )}
        {state.phase === "failed" && (
          <button
            onClick={() => {
              void session.submit();
            }}
          >
            使用原请求重试修订
          </button>
        )}
        {state.receipt && (
          <button
            disabled={state.refreshing}
            onClick={() => {
              void session.refresh();
            }}
          >
            重试刷新任务列表
          </button>
        )}
        <button onClick={dismiss}>{state.receipt ? "完成" : "关闭"}</button>
      </footer>
      <ObjectTaskRevisionHistory store={session.history} />
    </Dialog>
  );
}
