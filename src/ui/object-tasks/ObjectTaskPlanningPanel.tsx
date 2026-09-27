import { useEffect, useMemo, useState } from "react";
import { call } from "../api";
import {
  OBJECT_TASK_DRAFT_ID,
  type ObjectTaskWorkspaceState,
} from "./object-task-workspace";

type PlanningStatus =
  | "running"
  | "awaitingInput"
  | "proposed"
  | "adopted"
  | "cancelled"
  | "failed"
  | "interrupted";
type Question = {
  id: string;
  question?: string;
  recommended?: string;
  reason?: string;
};
type Session = {
  id: string;
  revision: number;
  status: PlanningStatus;
  questions: Question[];
  proposal?: {
    objects: unknown[];
    tasks: unknown[];
    assumptions: unknown[];
  } | null;
  error?: string | null;
};

const id = () => crypto.randomUUID();

export function ObjectTaskPlanningPanel({
  projectId,
  state,
}: {
  projectId: string;
  state: Extract<ObjectTaskWorkspaceState, { kind: "ready" }>;
}) {
  const [goal, setGoal] = useState("");
  const [acceptance, setAcceptance] = useState("");
  const [answers, setAnswers] = useState<Record<string, string>>({});
  const [session, setSession] = useState<Session | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const locked =
    Boolean(state.committedRequestId) ||
    state.operation !== "idle" ||
    Boolean(state.conflict);
  const active =
    session?.status === "running" || session?.status === "awaitingInput";
  const refresh = useMemo(
    () => async () => {
      const result = await call<Session | null>("objectTaskPlanning.get", {
        projectId,
        draftId: OBJECT_TASK_DRAFT_ID,
      });
      setSession(result);
    },
    [projectId],
  );
  useEffect(() => {
    void refresh();
  }, [refresh]);
  useEffect(() => {
    if (!active) return;
    const timer = window.setInterval(() => void refresh(), 1200);
    return () => window.clearInterval(timer);
  }, [active, refresh]);
  const run = async (operation: () => Promise<Session>) => {
    try {
      setMessage(null);
      setSession(await operation());
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
    }
  };
  const start = () =>
    run(() =>
      call<Session>("objectTaskPlanning.start", {
        projectId,
        requestId: id(),
        draftId: OBJECT_TASK_DRAFT_ID,
        expectedDraftRevision: state.draftRevision,
        expectedPlanRevision: state.planRevision,
        goal,
        acceptance,
        askRatio: 30,
      }),
    );
  const answer = () => {
    if (!session) return;
    return run(() =>
      call<Session>("objectTaskPlanning.answer", {
        projectId,
        sessionId: session.id,
        requestId: id(),
        expectedRevision: session.revision,
        answers,
      }),
    );
  };
  const mutate = (method: "cancel" | "adopt") => {
    if (!session) return;
    return run(() =>
      call<Session>(`objectTaskPlanning.${method}`, {
        projectId,
        sessionId: session.id,
        requestId: id(),
        expectedRevision: session.revision,
      }),
    );
  };
  return (
    <section className="object-task-planning" aria-label="Codex 对象任务规划">
      <h3>Codex 规划</h3>
      <p>Codex 只能提出问题或草稿提案；采纳后仍需人工提交任务。</p>
      {!active && session?.status !== "proposed" && (
        <>
          <label>
            目标
            <input
              value={goal}
              onChange={(event) => setGoal(event.target.value)}
            />
          </label>
          <label>
            验收要求
            <textarea
              value={acceptance}
              onChange={(event) => setAcceptance(event.target.value)}
            />
          </label>
          <button
            disabled={!goal.trim() || state.dirty || locked}
            onClick={() => void start()}
          >
            开始规划
          </button>
        </>
      )}
      {session && <p role="status">状态：{session.status}</p>}
      {session?.questions.map((question) => (
        <label key={question.id}>
          {question.question ?? question.id}
          <textarea
            value={answers[question.id] ?? ""}
            onChange={(event) =>
              setAnswers((current) => ({
                ...current,
                [question.id]: event.target.value,
              }))
            }
          />
        </label>
      ))}
      {session?.status === "awaitingInput" && (
        <button disabled={locked} onClick={() => void answer()}>
          提交回答
        </button>
      )}
      {active && (
        <button onClick={() => void mutate("cancel")}>取消规划</button>
      )}
      {session?.status === "proposed" && (
        <button
          disabled={locked || state.dirty}
          onClick={() => void mutate("adopt")}
        >
          采纳提案到草稿
        </button>
      )}
      {session?.error && <p role="alert">{session.error}</p>}
      {message && <p role="alert">{message}</p>}
    </section>
  );
}
