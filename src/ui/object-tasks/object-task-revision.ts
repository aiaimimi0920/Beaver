import { ZodError } from "zod";
import { parseObjectTaskSnapshot } from "../../shared/object-task-snapshot";
import {
  requireObjectTaskRevisionEligibility,
  revisionTarget,
  validateObjectTaskRevisionPlan,
} from "../../shared/object-task-revision-plan";
import {
  objectTaskDefinition,
  parseObjectTaskRevisionReceipt,
  revisePlannedObjectTaskSchema,
  sameObjectTaskDefinition,
  sameObjectTaskIdentity,
  type ObjectTaskDefinition,
  type ObjectTaskDefinitionRevision,
  type RevisePlannedObjectTaskRequest,
} from "../../shared/object-task-revisions";
import type { ObjectTaskSnapshot } from "../../shared/object-tasks";
import { ObjectTaskRevisionHistoryStore } from "./object-task-revision-history";
import { ObjectTaskPromptHistory } from "./object-task-prompt-history";

export interface ObjectTaskRevisionState {
  phase:
    | "editing"
    | "reviewing"
    | "submitting"
    | "failed"
    | "conflict"
    | "succeeded"
    | "readonly";
  snapshot: ObjectTaskSnapshot;
  definition: ObjectTaskDefinition;
  reason: string;
  promptSource: string;
  error: string;
  receipt: ObjectTaskDefinitionRevision | null;
  refreshing: boolean;
  latest: ObjectTaskSnapshot | null;
  latestLoading: boolean;
  latestError: string;
}

function message(error: unknown): string {
  if (error instanceof ZodError)
    return error.issues.map((issue) => issue.message).join("；");
  return error instanceof Error ? error.message : String(error);
}

export class ObjectTaskRevision {
  readonly history: ObjectTaskRevisionHistoryStore;
  readonly promptHistory: ObjectTaskPromptHistory;
  private request: RevisePlannedObjectTaskRequest | null = null;
  private generation = 0;
  private latestGeneration = 0;
  private listeners = new Set<() => void>();
  private state: ObjectTaskRevisionState;

  constructor(
    readonly projectId: string,
    snapshot: ObjectTaskSnapshot,
    readonly taskId: string,
    private readonly api: (method: string, input: unknown) => Promise<unknown>,
    private readonly refreshWorkspace: () => Promise<boolean>,
    private readonly requestId: () => string = () => crypto.randomUUID(),
  ) {
    const reviewed = parseObjectTaskSnapshot(snapshot, projectId);
    const task = revisionTarget(reviewed, taskId);
    let error = "";
    try {
      requireObjectTaskRevisionEligibility(reviewed, taskId);
    } catch (cause: unknown) {
      error = message(cause);
    }
    this.state = {
      phase: error ? "readonly" : "editing",
      snapshot: reviewed,
      definition: objectTaskDefinition(task),
      reason: "",
      promptSource: "",
      error,
      receipt: null,
      refreshing: false,
      latest: null,
      latestLoading: false,
      latestError: "",
    };
    this.history = new ObjectTaskRevisionHistoryStore(projectId, taskId, api);
    this.promptHistory = new ObjectTaskPromptHistory(
      projectId,
      taskId,
      () => this.state.snapshot,
      api,
    );
  }

  getSnapshot = () => this.state;
  subscribe = (listener: () => void) => {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  };
  private set(patch: Partial<ObjectTaskRevisionState>) {
    this.state = { ...this.state, ...patch };
    for (const listener of this.listeners) listener();
  }

  updateDefinition(patch: Partial<ObjectTaskDefinition>): void {
    if (this.state.phase !== "editing") return;
    this.set({
      definition: { ...this.state.definition, ...structuredClone(patch) },
      error: "",
    });
  }

  updateReason(reason: string): void {
    if (this.state.phase === "editing") this.set({ reason, error: "" });
  }

  loadHistoricalPrompt(attemptId: string): boolean {
    if (this.state.phase !== "editing") return false;
    const history = this.promptHistory.getSnapshot();
    const entry = history.entries.find(
      (entry) => entry.attempt.target.attemptId === attemptId,
    );
    if (history.loading || !history.loaded || !entry) return false;
    try {
      requireObjectTaskRevisionEligibility(this.state.snapshot, this.taskId);
      if (
        !this.promptHistory
          .sources()
          .some((source) => source.id === history.sourceTaskId)
      )
        return false;
      this.set({
        definition: {
          ...this.state.definition,
          prompt: entry.definition.prompt,
          acceptance: entry.definition.acceptance,
        },
        promptSource:
          "历史提示词来源：run " +
          entry.attempt.target.runId +
          " / fine " +
          entry.attempt.target.fineTaskId +
          " / attempt " +
          attemptId +
          " / definition revision " +
          entry.definition.revision,
        error: "",
      });
      return true;
    } catch (error: unknown) {
      this.set({ error: message(error) });
      return false;
    }
  }

  review = (): boolean => {
    if (this.state.phase !== "editing") return false;
    try {
      const task = revisionTarget(this.state.snapshot, this.taskId);
      const request = revisePlannedObjectTaskSchema.parse({
        projectId: this.projectId,
        taskId: this.taskId,
        requestId: this.requestId(),
        expectedTaskRevision: task.revision,
        expectedPlanRevision: this.state.snapshot.planRevision,
        definition: this.state.definition,
        reason: this.state.reason,
      });
      if (
        sameObjectTaskDefinition(request.definition, objectTaskDefinition(task))
      ) {
        throw new Error("任务定义没有变化，无需提交修订");
      }
      validateObjectTaskRevisionPlan(
        this.state.snapshot,
        this.taskId,
        request.definition,
      );
      this.request = this.state.promptSource
        ? revisePlannedObjectTaskSchema.parse({
            ...request,
            reason: request.reason + "\n" + this.state.promptSource,
          })
        : request;
      this.set({ phase: "reviewing", error: "" });
      return true;
    } catch (error: unknown) {
      this.set({ error: message(error) });
      return false;
    }
  };

  edit = () => {
    if (this.state.phase !== "reviewing") return;
    this.request = null;
    this.set({ phase: "editing", error: "" });
  };

  async submit(): Promise<ObjectTaskDefinitionRevision | null> {
    if (this.state.receipt) return this.state.receipt;
    if (!this.request || !["reviewing", "failed"].includes(this.state.phase))
      return null;
    const request = this.request;
    const generation = ++this.generation;
    this.set({ phase: "submitting", error: "" });
    try {
      // The server may commit before a response is lost; retries keep the whole request.
      const response = await this.api(
        "objectTask.revisePlanned",
        structuredClone(request),
      );
      if (generation !== this.generation) return null;
      const receipt = parseObjectTaskRevisionReceipt(
        response,
        request,
        this.state.snapshot,
      );
      this.set({ phase: "succeeded", receipt });
      this.history.confirm(receipt);
      void this.history.load();
      await this.refreshFor(generation);
      return generation === this.generation ? receipt : null;
    } catch (error: unknown) {
      if (generation !== this.generation) return null;
      const detail = message(error);
      if (/\bOBJECT_TASK_(?:PLAN_REVISION|REVISION)_CONFLICT\b/.test(detail)) {
        this.set({ phase: "conflict", error: detail, latest: null });
        await this.loadLatest();
      } else {
        this.set({ phase: "failed", error: detail });
      }
      return null;
    }
  }

  loadLatest = async (): Promise<boolean> => {
    if (this.state.phase !== "conflict" || this.state.latestLoading)
      return false;
    const generation = ++this.latestGeneration;
    this.set({ latestLoading: true, latestError: "", latest: null });
    try {
      const response = await this.api("objectTask.snapshot", {
        projectId: this.projectId,
      });
      if (generation !== this.latestGeneration) return false;
      const latest = parseObjectTaskSnapshot(response, this.projectId);
      const previous = revisionTarget(this.state.snapshot, this.taskId);
      const target = latest.tasks.find((task) => task.id === this.taskId);
      if (
        latest.planRevision < this.state.snapshot.planRevision ||
        (target && target.revision < previous.revision) ||
        (latest.planRevision === this.state.snapshot.planRevision &&
          target?.revision === previous.revision)
      ) {
        throw new Error("最新快照未包含冲突后的版本，请重新读取");
      }
      this.set({ latest, latestLoading: false });
      return true;
    } catch (error: unknown) {
      if (generation !== this.latestGeneration) return false;
      this.set({ latestLoading: false, latestError: message(error) });
      return false;
    }
  };

  rebase = (): boolean => {
    const latest = this.state.latest;
    if (this.state.phase !== "conflict" || !latest || this.state.latestLoading)
      return false;
    try {
      const target = requireObjectTaskRevisionEligibility(latest, this.taskId);
      if (
        !sameObjectTaskIdentity(
          revisionTarget(this.state.snapshot, this.taskId),
          target,
        )
      ) {
        throw new Error(
          "任务的对象、责任、阶段或迭代身份已改变，请关闭并重新核对任务",
        );
      }
      this.request = null;
      this.set({
        phase: "editing",
        snapshot: latest,
        latest: null,
        error: "",
        latestError: "",
      });
      return true;
    } catch (error: unknown) {
      this.set({ latestError: message(error) });
      return false;
    }
  };

  refresh = async (): Promise<boolean> => {
    if (!this.state.receipt || this.state.refreshing) return false;
    return this.refreshFor(++this.generation);
  };

  private async refreshFor(generation: number): Promise<boolean> {
    if (generation !== this.generation) return false;
    this.set({ refreshing: true, error: "" });
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
        error: `修订回执已确认，但刷新失败：${message(error)}。请重试刷新。`,
      });
      return false;
    }
  }

  cancel = () => {
    this.generation++;
    this.latestGeneration++;
    this.history.cancel();
    this.promptHistory.cancel();
    const pending = this.state.phase === "submitting";
    this.set({
      refreshing: false,
      latestLoading: false,
      ...(pending
        ? ({
            phase: "failed",
            error:
              "已停止等待，原请求仍可能完成；请刷新列表核对或使用原请求重试。",
          } as const)
        : {}),
    });
  };
}
