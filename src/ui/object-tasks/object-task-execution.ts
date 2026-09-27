import {
  objectAttemptInterruptReceiptSchema,
  objectAttemptInterruptSchema,
  objectExecutionSchema,
  type ObjectAttemptInterrupt,
  type ObjectAttemptInterruptReceipt,
  type ObjectExecution,
} from "../../shared/object-attempts";
import { parseObjectTaskSnapshot } from "../../shared/object-task-snapshot";
import { ObjectTaskRecovery } from "./object-task-recovery";
import { ObjectAttemptChecks } from "./object-attempt-checks";
import { ObjectCandidateReview } from "./object-candidate-review";
import { ObjectPublication } from "./object-publication";
import { ObjectAttemptFile } from "./object-attempt-file";
import type { ObjectCandidateReport } from "../../shared/object-candidate-review";
import type {
  ObjectTaskRecord,
  ObjectTaskSnapshot,
} from "../../shared/object-tasks";

type Api = (method: string, input: unknown) => Promise<unknown>;
interface State {
  phase: "loading" | "ready" | "submitting" | "failed" | "succeeded";
  executions: ObjectExecution[];
  refreshing: boolean;
  retryAvailable: boolean;
  error: string;
  receipt: ObjectAttemptInterruptReceipt | null;
}

import { ObjectAttemptTrace } from "./object-attempt-trace";

export class ObjectTaskExecution {
  readonly task: ObjectTaskRecord;
  readonly fineTasks: readonly ObjectTaskRecord[];
  readonly recovery: ObjectTaskRecovery;
  private request: ObjectAttemptInterrupt | null = null;
  private generation = 0;
  private checks = new Map<string, ObjectAttemptChecks>();
  private candidates = new Map<string, ObjectCandidateReview>();
  private publications = new Map<string, ObjectPublication>();
  private files = new Map<string, ObjectAttemptFile>();
  private traces = new Map<string, ObjectAttemptTrace>();
  private listeners = new Set<() => void>();
  private state: State = {
    phase: "loading",
    executions: [],
    refreshing: false,
    retryAvailable: false,
    error: "",
    receipt: null,
  };

  constructor(
    readonly projectId: string,
    snapshot: ObjectTaskSnapshot,
    taskId: string,
    private readonly api: Api,
    private readonly refreshWorkspace: () => Promise<boolean>,
    private readonly requestId: () => string = () => crypto.randomUUID(),
  ) {
    const reviewed = parseObjectTaskSnapshot(snapshot, projectId);
    const task = reviewed.tasks.find((task) => task.id === taskId);
    if (!task || task.granularity !== "medium" || !task.runId || !task.objectId)
      throw new Error("只能查看当前项目中修任务的执行记录");
    this.task = task;
    this.fineTasks = reviewed.tasks.filter(
      (fine) => fine.parentTaskId === taskId && fine.granularity === "fine",
    );
    this.recovery = new ObjectTaskRecovery(
      projectId,
      task.id,
      task.objectId,
      task.runId,
      api,
      this.requestId,
      this.resumed,
    );
  }

  private resumed = (attemptId: string) => {
    if (this.state.receipt?.result.target.attemptId !== attemptId) {
      this.generation++;
      this.request = null;
      this.set({
        receipt: null,
        retryAvailable: false,
        refreshing: false,
        phase: "ready",
      });
    }
    void this.refresh();
    void this.refreshWorkspace().catch((error: unknown) => {
      this.set({ error: "执行已重试，但任务列表刷新失败：" + String(error) });
    });
  };

  getSnapshot = () => this.state;
  publicationFor(report: ObjectCandidateReport) {
    const key = report.request.requestId;
    let session = this.publications.get(key);
    if (!session) {
      session = new ObjectPublication(
        {
          projectId: this.projectId,
          target: report.request.target,
          reviewRequestId: key,
        },
        this.api,
        async () => {
          await this.refresh(true);
          return this.refreshWorkspace();
        },
        this.requestId,
      );
      this.publications.set(key, session);
    }
    return session;
  }
  candidateFor(attempt: ObjectExecution["attempt"]) {
    const key = JSON.stringify(attempt.target);
    let session = this.candidates.get(key);
    if (!session) {
      session = new ObjectCandidateReview(
        this.projectId,
        attempt,
        this.api,
        this.requestId,
      );
      this.candidates.set(key, session);
    }
    return session;
  }
  filesFor(
    attempt: ObjectExecution["attempt"],
    purpose: "preview" | `feedback:${string}` = "preview",
  ) {
    const { runId, attemptId } = attempt.target;
    const key = `${attemptId}:${purpose}`;
    let session = this.files.get(key);
    if (!session) {
      session = new ObjectAttemptFile(
        { projectId: this.projectId, runId, attemptId },
        this.api,
      );
      this.files.set(key, session);
    }
    return session;
  }
  traceFor(attempt: ObjectExecution["attempt"]) {
    const { runId, attemptId } = attempt.target;
    let session = this.traces.get(attemptId);
    if (!session) {
      session = new ObjectAttemptTrace(
        { projectId: this.projectId, runId, attemptId },
        this.api,
      );
      this.traces.set(attemptId, session);
    }
    return session;
  }
  checksFor(attempt: ObjectExecution["attempt"]) {
    const key = JSON.stringify(attempt.target);
    let session = this.checks.get(key);
    if (!session) {
      session = new ObjectAttemptChecks(
        this.projectId,
        attempt,
        this.api,
        this.requestId,
      );
      this.checks.set(key, session);
    }
    return session;
  }
  subscribe = (listener: () => void) => {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  };
  private set(patch: Partial<State>) {
    this.state = { ...this.state, ...patch };
    for (const listener of this.listeners) listener();
  }
  cancel = () => {
    this.generation++;
    this.recovery.cancel();
    for (const session of this.files.values()) session.cancel();
    for (const session of this.traces.values()) session.cancel();
    for (const session of this.checks.values()) session.cancel();
    for (const session of this.candidates.values()) session.cancel();
    for (const session of this.publications.values()) session.cancel();
    if (this.state.phase === "submitting" || this.state.refreshing) {
      this.set({
        phase: this.state.receipt ? "succeeded" : "failed",
        refreshing: false,
        retryAvailable: !!this.request && !this.state.receipt,
        error: "已停止等待，可重新查询。已发出的中断请求仍可能完成。",
      });
    }
  };

  refresh = async (queryAfterReceipt = false): Promise<boolean> => {
    if (this.state.phase === "submitting" || this.state.refreshing)
      return false;
    const generation = ++this.generation;
    if (this.state.receipt && !queryAfterReceipt)
      return this.refreshWorkspaceFor(generation);
    this.set({ phase: "loading", refreshing: true, error: "" });
    if (generation !== this.generation) return false;
    try {
      const response = await this.api("objectTask.attempts", {
        projectId: this.projectId,
        runId: this.task.runId,
      });
      if (generation !== this.generation) return false;
      const executions = objectExecutionSchema.array().parse(response);
      const ids = new Set<string>();
      for (const { attempt } of executions) {
        const target = attempt.target;
        if (
          attempt.projectId !== this.projectId ||
          target.runId !== this.task.runId ||
          target.taskId !== this.task.id ||
          target.objectId !== this.task.objectId ||
          !this.fineTasks.some((fine) => fine.id === target.fineTaskId) ||
          ids.has(target.attemptId)
        )
          throw new Error("执行记录与当前项目或任务不一致");
        ids.add(target.attemptId);
      }
      this.set({ phase: "ready", executions, refreshing: false });
      return true;
    } catch (error: unknown) {
      if (generation !== this.generation) return false;
      this.set({ phase: "failed", refreshing: false, error: String(error) });
      return false;
    }
  };

  async interrupt(
    attemptId?: string,
  ): Promise<ObjectAttemptInterruptReceipt | null> {
    if (this.state.receipt) return this.state.receipt;
    if (
      this.state.phase === "submitting" ||
      this.state.refreshing ||
      this.recovery.blocksInterrupt()
    )
      return null;
    if (!this.request) {
      const execution = this.state.executions.find(
        ({ attempt }) => attempt.target.attemptId === attemptId,
      );
      if (
        this.state.phase !== "ready" ||
        !execution ||
        execution.availability !== "active"
      )
        return null;
      this.request = objectAttemptInterruptSchema.parse({
        projectId: this.projectId,
        requestId: this.requestId(),
        target: execution.attempt.target,
        expectedTaskRevision: execution.attempt.taskRevision,
      });
    }
    const generation = ++this.generation;
    this.set({ phase: "submitting", error: "", retryAvailable: false });
    if (generation !== this.generation) return null;
    try {
      // Retries keep the full reviewed target, even if a later query has changed.
      const response = await this.api(
        "objectTask.interrupt",
        structuredClone(this.request),
      );
      if (generation !== this.generation) return null;
      const receipt = objectAttemptInterruptReceiptSchema.parse(response);
      if (JSON.stringify(receipt.request) !== JSON.stringify(this.request))
        throw new Error("中断回执与当前请求不一致");
      this.set({
        phase: "succeeded",
        receipt,
        executions: this.state.executions.map((execution) =>
          execution.attempt.target.attemptId === receipt.result.target.attemptId
            ? {
                ...execution,
                attempt: receipt.result,
                availability: "finished",
              }
            : execution,
        ),
      });
      await this.refreshWorkspaceFor(generation);
      return generation === this.generation ? receipt : null;
    } catch (error: unknown) {
      if (generation !== this.generation) return null;
      const message = error instanceof Error ? error.message : String(error);
      const stale =
        message === "OBJECT_ATTEMPT_STALE_TARGET" ||
        message === "OBJECT_ATTEMPT_REVISION_CONFLICT";
      // Only an explicit rejection proves this request was not accepted.
      if (stale) this.request = null;
      this.set({
        phase: "failed",
        retryAvailable: !stale,
        error: stale
          ? "执行目标已变化，请刷新执行记录后重新确认中断。"
          : message,
      });
      return null;
    }
  }

  private async refreshWorkspaceFor(generation: number): Promise<boolean> {
    if (generation !== this.generation) return false;
    this.set({ refreshing: true, error: "" });
    if (generation !== this.generation) return false;
    try {
      if (!(await this.refreshWorkspace()))
        throw new Error("任务列表尚未完成刷新");
      if (generation !== this.generation) return false;
      this.set({ refreshing: false });
      return true;
    } catch (error: unknown) {
      if (generation !== this.generation) return false;
      this.set({
        refreshing: false,
        error: `中断回执已确认，但刷新失败：${String(error)}。请重试刷新。`,
      });
      return false;
    }
  }
}
