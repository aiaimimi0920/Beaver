import { useEffect, useRef, useState } from "react";
import { z } from "zod";
import type { Task, TaskEvent } from "../shared/types";
import { directions, taskDirection, taskAttention } from "../shared/task-board";
import { TaskResources } from "./TaskResources";
import { call, statusNames, type Run } from "./api";
import { Dialog } from "./components";
import { PagedText } from "./PagedText";
import { PagedEditor } from "./PagedEditor";
import { conversationText, executionStatus } from "../shared/task-conversation";
import { formatTaskBrief } from "../shared/task-brief";
import "./task-conversation.css";

export function TaskConversation({
  task,
  tasks,
  run,
  back,
  open,
}: {
  task: Task;
  tasks: Task[];
  run: Run;
  back: () => void;
  open: (id: string) => void;
}) {
  const draftKey = `beaver.task-draft.v1.${task.id}`;
  const [draft] = useState(() => {
    try {
      return z
        .object({
          text: z.string(),
          answers: z.record(z.string(), z.string()),
          pendingId: z.string().optional(),
          automatic: z.array(z.string()).optional(),
        })
        .parse(JSON.parse(localStorage.getItem(draftKey) ?? "{}"));
    } catch {
      return {
        text: "",
        answers: {} as Record<string, string>,
        pendingId: undefined,
        automatic: [],
      };
    }
  });
  const lastPending = useRef(draft.pendingId);
  const [tab, setTab] = useState("对话");
  const [events, setEvents] = useState<TaskEvent[]>([]);
  const [error, setError] = useState("");
  const [text, setText] = useState(draft.text);
  const [busy, setBusy] = useState(false);
  const [answers, setAnswers] = useState<Record<string, string>>(draft.answers);
  const [autoIds, setAutoIds] = useState<string[]>(draft.automatic ?? []);
  const [questionIndex, setQuestionIndex] = useState(0);
  const [filePage, setFilePage] = useState(0);
  const [keep, setKeep] = useState<string[]>([]);
  const [rollback, setRollback] = useState(false);
  const [mode, setMode] = useState("补充");
  const children = tasks.filter((item) => item.parentTaskId === task.id);
  const delegating = tab === "子任务";
  const pending = task.clarifications?.find((item) => !item.answers);
  const question =
    pending?.questions[Math.min(questionIndex, pending.questions.length - 1)];
  useEffect(() => {
    let live = true;
    let fetching = false;
    const refreshEvents = () => {
      if (fetching) return;
      fetching = true;
      void call<TaskEvent[]>("task.events", { id: task.id })
        .then((value) => {
          if (live) {
            setEvents(value);
            setError("");
          }
        })
        .catch(() => {
          if (live) setError("对话记录读取失败，请重新进入任务。");
        })
        .finally(() => {
          fetching = false;
        });
    };
    refreshEvents();
    const timer =
      task.status === "running" ? setInterval(refreshEvents, 1500) : undefined;
    return () => {
      live = false;
      clearInterval(timer);
    };
  }, [task.id, task.updatedAt, task.status]);
  useEffect(() => {
    if (lastPending.current !== pending?.id) {
      setAnswers({});
      setAutoIds([]);
    }
    lastPending.current = pending?.id;
    setQuestionIndex(0);
  }, [pending?.id]);
  useEffect(() => {
    try {
      localStorage.setItem(
        draftKey,
        JSON.stringify({
          text,
          answers,
          automatic: autoIds,
          pendingId: pending?.id,
        }),
      );
    } catch {
      setError("草稿缓存失败，请先提交或复制内容再离开。");
    }
  }, [draftKey, text, answers, autoIds, pending?.id]);
  const terminal = ["completed", "rolledBack", "conflict"].includes(
    task.status,
  );
  const conversation = conversationText(events, task.report);
  const content =
    tab === "决策记录"
      ? (task.clarifications ?? [])
          .flatMap((item) =>
            (item.automaticQuestions ?? []).map(
              (q) =>
                `${q.question}\n自动采用：${item.autoAnswers?.[q.id] ?? q.recommended}\n推荐理由：${q.reason ?? ""}\n重要性：${q.importance ?? "未提供"} · ${autonomyLabel(item.answerSources?.[q.id] === "automatic" ? (item.appliedAskRatio ?? item.askRatio ?? 100) : (item.askRatio ?? 100))}`,
            ),
          )
          .join("\n\n") || "暂无自动决策记录"
      : tab === "执行记录"
        ? events
            .filter((e) => !["user", "assistant"].includes(e.kind))
            .map((e) => `${e.time} · ${e.kind}\n${e.text}`)
            .join("\n\n")
        : tab === "要求"
          ? `${task.prompt}\n\n${formatTaskBrief(task)}\n\n${task.references.map((r) => `${r.path} · ${r.note}`).join("\n")}`
          : `${error || task.error || ""}\n${conversation || task.prompt}`;
  const label = delegating
    ? "委派子任务"
    : pending
      ? "提交回答并继续"
      : mode === "对话回退"
        ? "发起对话回退"
        : task.status === "conflict"
          ? "发起整合任务"
          : terminal
            ? "继续创作"
            : task.status === "running"
              ? "发送补充"
              : task.status === "queued"
                ? "保存补充"
                : "补充并继续";
  const fileCount = Math.max(1, Math.ceil(task.changes.length / 5));
  const currentFilePage = Math.min(filePage, fileCount - 1);
  function send() {
    if (busy) return;
    setBusy(true);
    void run(async () => {
      if (delegating) {
        const next = await call<Task>("task.delegate", { id: task.id, text });
        open(next.id);
      } else if (pending)
        await call("task.answer", {
          id: task.id,
          questionId: pending.id,
          answers,
          automatic: autoIds.filter((id) =>
            pending.questions.some(
              (q) =>
                q.id === id &&
                q.recommended === answers[id] &&
                automaticChoice(task.effectiveAskRatio ?? 100, q) ===
                  answers[id],
            ),
          ),
        });
      else if (mode === "对话回退" || task.status === "conflict") {
        const next = await call<Task>("task.dialogueRollback", {
          id: task.id,
          text,
        });
        open(next.id);
      } else if (terminal) {
        const next = await call<Task>("task.followup", { id: task.id, text });
        open(next.id);
      } else await call("task.continue", { id: task.id, text });
      setText("");
    }).finally(() => setBusy(false));
  }
  function restartContext() {
    if (busy) return;
    setBusy(true);
    void run(async () => {
      await call("task.continue", { id: task.id, text, freshContext: true });
      setText("");
    }).finally(() => setBusy(false));
  }
  return (
    <section className="task-conversation">
      <header className="conversation-heading">
        <button onClick={back}>‹ 看板</button>
        <h1 title={task.title}>{task.title}</h1>
        {taskAttention(task) && (
          <b className="task-attention">! {taskAttention(task)}</b>
        )}
        <span
          className={`badge ${task.status} ${task.status === "running" ? "is-running" : ""}`}
        >
          {statusNames[task.status]}
        </span>
        <select
          aria-label="任务方向"
          value={taskDirection(task)}
          onChange={(e) =>
            void run(() =>
              call("task.direction", {
                id: task.id,
                direction: e.target.value,
              }),
            )
          }
        >
          {Object.entries(directions).map(([id, info]) => (
            <option key={id} value={id}>
              {info.label}
            </option>
          ))}
        </select>
      </header>
      <div className="question-navigation">
        {(task.decompose || task.relation === "child") && (
          <select
            aria-label="子任务审批策略"
            value={task.autoAccept === false ? "manual" : "automatic"}
            disabled={task.accepted || busy}
            onChange={(e) =>
              void run(() =>
                call("task.approval", {
                  id: task.id,
                  autoAccept: e.target.value === "automatic",
                }),
              )
            }
          >
            <option value="automatic">完成后自动审批</option>
            <option value="manual">完成后手动审批</option>
          </select>
        )}
        <span role="status">
          {task.status === "waitingChildren" &&
            `${children.filter((t) => t.accepted).length} / ${children.length} 个子任务已审批；打开子任务查看进度。`}
          {task.status === "running"
            ? executionStatus(events, Date.now(), Date.parse(task.updatedAt)) ||
              "正在准备执行 · 等待期间不会自动重放操作"
            : "AI 任务完成与用户认可不代表导出验收通过"}
          {task.delivery
            ? `；最近导出快照：${task.delivery.status === "verified" ? "包完整性通过，运行/玩法未验收" : "验证失败"}（${task.delivery.checkedAt}，当前工程改动需重新导出）`
            : "；尚无标准导出验证记录"}
        </span>
        <AutonomySelect
          label="任务自动决策"
          inherit
          disabled={busy}
          value={task.askRatio}
          change={(value) =>
            void run(() =>
              call("task.autonomy", { id: task.id, askRatio: value }),
            )
          }
        />
        <span>
          {task.askRatio == null ? "全局" : "任务覆盖"} · 当前
          {autonomyLabel(task.effectiveAskRatio ?? task.askRatio ?? 100)}
        </span>
      </div>
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
        {["running", "queued", "waitingChildren"].includes(task.status) && (
          <button
            onClick={() =>
              void run(() => call("task.interrupt", { id: task.id }))
            }
          >
            中止
          </button>
        )}
        {(["interrupted", "failed"].includes(task.status) ||
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
      <main className="conversation-content">
        {tab === "资料与素材" ? (
          <TaskResources
            task={task}
            run={run}
            feedback={(path) => {
              if (pending && question)
                setAnswers((old) => ({
                  ...old,
                  [question.id]: `${old[question.id] ?? ""}\n针对文件 ${path}：`,
                }));
              else
                setText((old) => `${old}${old ? "\n" : ""}针对文件 ${path}：`);
              setTab("对话");
            }}
          />
        ) : delegating ? (
          <div className="child-tasks">
            {children.map((child) => (
              <button key={child.id} onClick={() => open(child.id)}>
                <strong title={child.title}>{child.title}</strong>
                <span>
                  {taskAttention(child) ||
                    (child.accepted ? "已认可" : statusNames[child.status])}
                </span>
              </button>
            ))}
            {!children.length && <small>暂无子任务</small>}
          </div>
        ) : tab === "文件" ? (
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
                {currentFilePage + 1} / {fileCount} · {task.changes.length}{" "}
                个文件
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
        ) : (
          <PagedText
            key={`${task.id}-${tab}-${question?.id ?? "history"}`}
            latest={tab === "对话" && !pending}
            text={
              pending && tab === "对话"
                ? `Codex 需要你补充\n\n${question?.question ?? ""}${question?.recommended ? `\n\n推荐：${question.recommended}\n理由：${question.reason ?? ""}\n重要性：${question.importance ?? "未提供"}` : ""}`
                : content
            }
            label={tab}
          />
        )}
      </main>
      <section className="conversation-composer">
        {pending && !delegating && (
          <div className="question-navigation">
            <button
              disabled={busy}
              onClick={() => {
                const next = { ...answers };
                const ids = [...autoIds];
                for (const q of pending.questions) {
                  if (next[q.id]?.trim()) continue;
                  try {
                    const value = automaticChoice(
                      task.effectiveAskRatio ?? task.askRatio ?? 100,
                      q,
                    );
                    if (value !== undefined) {
                      next[q.id] = value;
                      ids.push(q.id);
                    }
                  } catch {
                    /* Legacy questions remain manual. */
                  }
                }
                setAnswers(next);
                setAutoIds([...new Set(ids)]);
              }}
            >
              按当前档位填写推荐答案
            </button>
            <strong>
              问题 {questionIndex + 1} / {pending.questions.length}
            </strong>
            {pending.questions.map((q, i) => (
              <button
                key={q.id}
                aria-pressed={questionIndex === i}
                onClick={() => setQuestionIndex(i)}
              >
                {i + 1}
                {answers[q.id]?.trim() ? " ✓" : ""}
              </button>
            ))}
            {!!question?.options?.length && (
              <select
                aria-label="回答建议"
                disabled={busy}
                value={
                  question.options.some(
                    (option) => option.label === answers[question.id],
                  )
                    ? answers[question.id]
                    : ""
                }
                onChange={(e) => {
                  setAutoIds((ids) => ids.filter((id) => id !== question.id));
                  setAnswers((old) => ({
                    ...old,
                    [question.id]: e.target.value,
                  }));
                }}
              >
                <option value="">选择建议或自行填写</option>
                {question.options.map((option) => (
                  <option key={option.label} value={option.label}>
                    {option.label}
                  </option>
                ))}
              </select>
            )}
            {question?.options?.find(
              (option) => option.label === answers[question.id],
            )?.description && (
              <span role="status">
                {
                  question.options.find(
                    (option) => option.label === answers[question.id],
                  )?.description
                }
              </span>
            )}
          </div>
        )}
        <PagedEditor
          key={question?.id ?? "follow"}
          label={
            delegating
              ? "子任务目标"
              : pending
                ? "回答当前问题"
                : "补充任务对话"
          }
          value={
            pending && question && !delegating
              ? (answers[question.id] ?? "")
              : text
          }
          disabled={busy || (task.status === "waitingChildren" && !delegating)}
          change={(value) => {
            if (pending && question && !delegating) {
              setAutoIds((ids) => ids.filter((id) => id !== question.id));
              setAnswers((old) => ({ ...old, [question.id]: value }));
            } else setText(value);
          }}
        />
        <footer>
          <span>
            {delegating
              ? "独立副本 · 从项目已合入版本开始"
              : terminal
                ? "后续修改保留独立回退记录"
                : task.status === "running"
                  ? "直接补充到正在执行的任务"
                  : ""}
          </span>
          {!pending && !delegating && task.changes.length > 0 && (
            <select
              aria-label="对话操作"
              value={mode}
              onChange={(e) => setMode(e.target.value)}
            >
              <option>补充</option>
              <option>对话回退</option>
            </select>
          )}
          <button
            className="primary"
            disabled={
              busy ||
              (task.status === "waitingChildren" && !delegating) ||
              (pending && !delegating
                ? pending.questions.some((q) => !answers[q.id]?.trim())
                : !text.trim())
            }
            onClick={send}
          >
            {busy ? "正在提交…" : label}
          </button>
        </footer>
      </section>
      {rollback && (
        <Dialog title="标准回退" close={() => setRollback(false)}>
          <p>
            撤销此任务的修改，保留勾选的 {keep.length}{" "}
            个文件。遇到后续修改冲突时停止，不强行覆盖。
          </p>
          <footer>
            <button onClick={() => setRollback(false)}>取消</button>
            <button
              className="danger"
              onClick={() =>
                void run(async () => {
                  await call("task.rollback", { id: task.id, keep });
                  setRollback(false);
                })
              }
            >
              确认回退
            </button>
          </footer>
        </Dialog>
      )}
    </section>
  );
}
import { AutonomySelect } from "./AutonomySelect";
import { automaticChoice, autonomyLabel } from "../shared/autonomy";
