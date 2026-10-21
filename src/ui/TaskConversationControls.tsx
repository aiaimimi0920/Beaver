import type { Dispatch, SetStateAction } from "react";
import type { Task } from "../shared/types";
import { call, type Run } from "./api";
import { Dialog } from "./components";
import { SupplementWithdrawal } from "./SupplementWithdrawal";

export function TaskConversationTabs({
  task,
  run,
  open,
  tab,
  setTab,
  busy,
  pending,
  restartContext,
}: {
  task: Task;
  run: Run;
  open: (id: string) => void;
  tab: string;
  setTab: (tab: string) => void;
  busy: boolean;
  pending: boolean;
  restartContext: () => void;
}) {
  return (
    <nav className="conversation-tabs" aria-label="任务视图">
      {task.parentTaskId && (
        <button onClick={() => open(task.parentTaskId!)}>原任务</button>
      )}
      {[
        "对话",
        "资料与素材",
        "子任务",
        "文件",
        "要求",
        "执行记录",
        "决策记录",
      ].map((name) => (
        <button
          key={name}
          aria-pressed={tab === name}
          onClick={() => setTab(name)}
        >
          {name}
        </button>
      ))}
      <span />
      <SupplementWithdrawal task={task} run={run} />
      {["running", "queued", "waitingChildren"].includes(task.status) && (
        <button
          onClick={() =>
            void run(() => call("task.interrupt", { id: task.id }))
          }
        >
          中止
        </button>
      )}
      {!task.supplementWithdrawal &&
        (["interrupted", "failed"].includes(task.status) ||
          task.status === "waitingChildren") && (
          <button
            onClick={() =>
              void run(() => call("task.continue", { id: task.id, text: "" }))
            }
          >
            {task.status === "waitingChildren" ? "继续子任务" : "继续任务"}
          </button>
        )}
      {typeof window !== "undefined" &&
        "__TAURI__" in window &&
        task.threadId &&
        ["interrupted", "failed"].includes(task.status) && (
          <button
            disabled={busy || !!pending}
            title="保留工作副本、原会话历史和回退基线，用新 AI 会话检查已有进度后继续。"
            onClick={restartContext}
          >
            用新会话继续
          </button>
        )}
      <button
        disabled={task.workspacePrepared === false}
        title={
          task.workspacePrepared === false
            ? "前置子任务获批后创建工作副本"
            : undefined
        }
        onClick={() => void run(() => call("task.reveal", { id: task.id }))}
      >
        工作副本
      </button>
      {task.status === "completed" && (
        <button
          disabled={task.accepted}
          onClick={() => void run(() => call("task.accept", { id: task.id }))}
        >
          {task.accepted
            ? task.approvalSource === "automatic"
              ? "已自动审批"
              : "已认可"
            : "认可结果"}
        </button>
      )}
    </nav>
  );
}

export function TaskConversationFiles({
  task,
  keep,
  setKeep,
  currentFilePage,
  fileCount,
  setFilePage,
  setRollback,
}: {
  task: Task;
  keep: string[];
  setKeep: Dispatch<SetStateAction<string[]>>;
  currentFilePage: number;
  fileCount: number;
  setFilePage: (page: number) => void;
  setRollback: (value: boolean) => void;
}) {
  return (
    <div className="conversation-files">
      {task.changes
        .slice(currentFilePage * 5, (currentFilePage + 1) * 5)
        .map((change) => (
          <label key={change.path} title={change.path}>
            <input
              type="checkbox"
              checked={keep.includes(change.path)}
              onChange={(e) =>
                setKeep((old) =>
                  e.target.checked
                    ? [...old, change.path]
                    : old.filter((p) => p !== change.path),
                )
              }
            />
            <span>{change.path}</span>
            <small>
              {task.conflicts.includes(change.path)
                ? "冲突"
                : !change.before
                  ? "新增"
                  : !change.after
                    ? "删除"
                    : "修改"}
            </small>
          </label>
        ))}
      <footer className="page-controls">
        <button
          disabled={!currentFilePage}
          onClick={() => setFilePage(currentFilePage - 1)}
        >
          ‹
        </button>
        <span>
          {currentFilePage + 1} / {fileCount} · {task.changes.length} 个文件
        </span>
        <button
          disabled={currentFilePage + 1 >= fileCount}
          onClick={() => setFilePage(currentFilePage + 1)}
        >
          ›
        </button>
        {task.status === "completed" && (
          <button className="danger" onClick={() => setRollback(true)}>
            标准回退
          </button>
        )}
      </footer>
    </div>
  );
}

export function TaskRollbackDialog({
  task,
  keep,
  run,
  close,
}: {
  task: Task;
  keep: string[];
  run: Run;
  close: () => void;
}) {
  return (
    <Dialog title="标准回退" close={close}>
      <p>
        撤销此任务的修改，保留勾选的 {keep.length}{" "}
        个文件。遇到后续修改冲突时停止，不强行覆盖。
      </p>
      <footer>
        <button onClick={close}>取消</button>
        <button
          className="danger"
          onClick={() =>
            void run(async () => {
              await call("task.rollback", { id: task.id, keep });
              close();
            })
          }
        >
          确认回退
        </button>
      </footer>
    </Dialog>
  );
}
