import { parseObjectTaskSnapshot } from "../../shared/object-task-snapshot";
import {
  cancelPlannedObjectTaskSchema,
  objectTaskCancellationReceiptSchema,
  type CancelPlannedObjectTaskRequest,
  type ObjectRun,
  type ObjectTaskCancellationReceipt,
  type ObjectTaskRecord,
  type ObjectTaskSnapshot,
} from "../../shared/object-tasks";

type Api = (method: string, input: unknown) => Promise<unknown>;
interface State {
  phase: "reviewing" | "blocked" | "submitting" | "failed" | "succeeded";
  refreshing: boolean;
  error: string;
  receipt: ObjectTaskCancellationReceipt | null;
}

export class ObjectTaskCancellation {
  readonly task: ObjectTaskRecord;
  readonly tasks: readonly ObjectTaskRecord[];
  readonly runs: readonly ObjectRun[];
  readonly planRevision: number;
  private request: CancelPlannedObjectTaskRequest | null = null;
  private generation = 0;
  private listeners = new Set<() => void>();
  private state: State = {
    phase: "reviewing",
    refreshing: false,
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
    const task = reviewed.tasks.find((candidate) => candidate.id === taskId);
    if (!task || task.status !== "planned")
      throw new Error("只能撤销当前项目中尚未执行的已规划任务");
    this.task = task;
    this.planRevision = reviewed.planRevision;
    const owned = new Set([task.id]);
    const pending = [task.id];
    for (const parentId of pending) {
      for (const child of reviewed.tasks) {
        if (child.parentTaskId === parentId && !owned.has(child.id)) {
          owned.add(child.id);
          pending.push(child.id);
        }
      }
    }
    const tasks = reviewed.tasks.filter((candidate) => owned.has(candidate.id));
    const runs = reviewed.runs.filter((run) => owned.has(run.mediumTaskId));
    this.tasks = tasks.filter((candidate) => candidate.status === "planned");
    this.runs = runs.filter((run) => run.status === "planned");
    if (
      tasks.some(
        (candidate) => !["planned", "cancelled"].includes(candidate.status),
      ) ||
      runs.some((run) => !["planned", "cancelled"].includes(run.status))
    ) {
      this.state = {
        ...this.state,
        phase: "blocked",
        error:
          "责任范围内已有任务或迭代进入执行流程，不能撤销规划。请刷新列表后处理该迭代。",
      };
    }
  }

  getSnapshot = () => this.state;
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
    if (this.state.phase === "submitting" || this.state.refreshing) {
      this.set({
        phase: this.state.receipt ? "succeeded" : "failed",
        refreshing: false,
        error: "已停止等待，原请求仍可能完成；可刷新任务列表核对。",
      });
    }
  };

  async submit(): Promise<ObjectTaskCancellationReceipt | null> {
    if (this.state.receipt) return this.state.receipt;
    if (this.state.phase === "submitting" || this.state.phase === "blocked")
      return null;
    const generation = ++this.generation;
    this.set({ phase: "submitting", error: "" });
    try {
      // A lost response may follow a commit; retries keep the reviewed payload and ID.
      this.request ??= cancelPlannedObjectTaskSchema.parse({
        projectId: this.projectId,
        taskId: this.task.id,
        requestId: this.requestId(),
        expectedTaskRevision: this.task.revision,
        expectedPlanRevision: this.planRevision,
      });
      const response = await this.api(
        "objectTask.cancelPlanned",
        structuredClone(this.request),
      );
      if (generation !== this.generation) return null;
      const receipt = objectTaskCancellationReceiptSchema.parse(response);
      if (
        receipt.projectId !== this.request.projectId ||
        receipt.taskId !== this.request.taskId ||
        receipt.requestId !== this.request.requestId ||
        receipt.previousTaskRevision !== this.request.expectedTaskRevision ||
        receipt.taskRevision !== this.request.expectedTaskRevision + 1 ||
        receipt.previousPlanRevision !== this.request.expectedPlanRevision ||
        receipt.planRevision !== this.request.expectedPlanRevision + 1
      ) {
        throw new Error("任务撤销回执与当前请求不一致");
      }
      this.set({ phase: "succeeded", receipt });
      await this.refreshFor(generation);
      return generation === this.generation ? receipt : null;
    } catch (error: unknown) {
      if (generation !== this.generation) return null;
      this.set({
        phase: "failed",
        error: error instanceof Error ? error.message : String(error),
      });
      return null;
    }
  }

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
      const message = error instanceof Error ? error.message : String(error);
      this.set({
        refreshing: false,
        error: `撤销回执已确认，但刷新失败：${message}。请重试刷新。`,
      });
      return false;
    }
  }
}
