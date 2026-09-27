import {
  coarseDispatchReceiptSchema,
  setCoarsePausedSchema,
  type CoarseDispatchControl,
  type CoarseDispatchReceipt,
  type SetCoarsePausedRequest,
  objectTaskDispatchReceiptSchema,
  setObjectTaskPausedSchema,
  type ObjectTaskDispatchControl,
  type ObjectTaskDispatchReceipt,
  type SetObjectTaskPausedRequest,
} from "../../shared/object-task-dispatch";
import { parseObjectTaskSnapshot } from "../../shared/object-task-snapshot";
import type {
  ObjectTaskRecord,
  ObjectTaskSnapshot,
} from "../../shared/object-tasks";

type Api = (method: string, input: unknown) => Promise<unknown>;
interface State {
  phase: "loading" | "ready" | "submitting" | "failed" | "succeeded";
  task: ObjectTaskRecord;
  control: ObjectTaskDispatchControl | CoarseDispatchControl;
  parentPaused: boolean;
  refreshing: boolean;
  retryAvailable: boolean;
  error: string;
  receipt: ObjectTaskDispatchReceipt | CoarseDispatchReceipt | null;
}

function review(input: unknown, projectId: string, taskId: string) {
  const snapshot = parseObjectTaskSnapshot(input, projectId);
  const task = snapshot.tasks.find((task) => task.id === taskId);
  if (!task || task.granularity === "fine")
    throw new Error("只能控制当前项目粗修或中修任务的派发");
  if (task.granularity === "coarse") {
    const control = snapshot.coarseDispatchControls.find(
      (item) => item.taskId === task.id,
    ) ?? {
      schemaVersion: 1 as const,
      projectId,
      taskId,
      paused: false,
      revision: 0,
    };
    return { task, control, parentPaused: false };
  }
  if (!task.runId || !task.objectId) throw new Error("中修派发身份不完整");
  const control = snapshot.dispatchControls.find(
    (item) => item.taskId === task.id,
  ) ?? {
    schemaVersion: 1 as const,
    projectId,
    taskId,
    objectId: task.objectId,
    runId: task.runId,
    paused: false,
    revision: 0,
  };
  const parentPaused = snapshot.coarseDispatchControls.some(
    (item) => item.taskId === task.parentTaskId && item.paused,
  );
  return { task, control, parentPaused };
}

export class ObjectTaskDispatch {
  private readonly identity: ObjectTaskRecord;
  private request: SetObjectTaskPausedRequest | SetCoarsePausedRequest | null =
    null;
  private generation = 0;
  private listeners = new Set<() => void>();
  private state: State;

  constructor(
    readonly projectId: string,
    snapshot: ObjectTaskSnapshot,
    taskId: string,
    private readonly api: Api,
    private readonly refreshWorkspace: () => Promise<boolean>,
    private readonly requestId: () => string = () => crypto.randomUUID(),
  ) {
    const reviewed = review(snapshot, projectId, taskId);
    this.identity = reviewed.task;
    this.state = {
      ...reviewed,
      phase: "loading",
      refreshing: false,
      retryAvailable: false,
      error: "",
      receipt: null,
    };
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
        retryAvailable: !!this.request && !this.state.receipt,
        error: "已停止等待。已发出的派发控制请求仍可能完成。",
      });
    }
  };

  refresh = async (): Promise<boolean> => {
    if (this.state.phase === "submitting" || this.state.refreshing)
      return false;
    const generation = ++this.generation;
    if (this.state.receipt) return this.refreshWorkspaceFor(generation);
    this.set({ phase: "loading", refreshing: true, error: "" });
    if (generation !== this.generation) return false;
    try {
      const response = await this.api("objectTask.snapshot", {
        projectId: this.projectId,
      });
      if (generation !== this.generation) return false;
      const reviewed = review(response, this.projectId, this.identity.id);
      if (
        reviewed.task.runId !== this.identity.runId ||
        reviewed.task.granularity !== this.identity.granularity ||
        reviewed.task.parentTaskId !== this.identity.parentTaskId ||
        reviewed.task.objectId !== this.identity.objectId
      )
        throw new Error("派发控制目标已变化，请关闭窗口后重新选择任务");
      this.set({ ...reviewed, phase: "ready", refreshing: false });
      return true;
    } catch (error: unknown) {
      if (generation !== this.generation) return false;
      this.set({ phase: "failed", refreshing: false, error: String(error) });
      return false;
    }
  };

  confirm = async (): Promise<
    ObjectTaskDispatchReceipt | CoarseDispatchReceipt | null
  > => {
    if (this.state.receipt) return this.state.receipt;
    if (this.state.phase === "submitting" || this.state.refreshing) return null;
    if (!this.request) {
      if (
        this.state.phase !== "ready" ||
        this.state.task.status === "cancelled"
      )
        return null;
      const { task, control } = this.state;
      if (control.revision === Number.MAX_SAFE_INTEGER) {
        this.set({
          phase: "failed",
          error: "派发控制版本已达到上限，无法继续更改。",
        });
        return null;
      }
      const input = {
        projectId: this.projectId,
        taskId: task.id,
        requestId: this.requestId(),
        expectedTaskRevision: task.revision,
        expectedControlRevision: control.revision,
        paused: !control.paused,
      };
      this.request =
        task.granularity === "coarse"
          ? setCoarsePausedSchema.parse(input)
          : setObjectTaskPausedSchema.parse({
              ...input,
              objectId: task.objectId,
              runId: task.runId,
            });
    }
    const generation = ++this.generation;
    this.set({ phase: "submitting", error: "", retryAvailable: false });
    if (generation !== this.generation) return null;
    try {
      // An ambiguous response keeps this exact request even after a newer snapshot.
      const response = await this.api(
        this.identity.granularity === "coarse"
          ? "objectTask.setCoarsePaused"
          : "objectTask.setPaused",
        structuredClone(this.request),
      );
      if (generation !== this.generation) return null;
      const receipt =
        this.identity.granularity === "coarse"
          ? coarseDispatchReceiptSchema.parse(response)
          : objectTaskDispatchReceiptSchema.parse(response);
      if (JSON.stringify(receipt.request) !== JSON.stringify(this.request))
        throw new Error("派发控制回执与当前请求不一致");
      this.set({ phase: "succeeded", receipt });
      await this.refreshWorkspaceFor(generation);
      return generation === this.generation ? receipt : null;
    } catch (error: unknown) {
      if (generation !== this.generation) return null;
      const message = error instanceof Error ? error.message : String(error);
      const stale =
        message === "OBJECT_TASK_REVISION_CONFLICT" ||
        message === "OBJECT_TASK_DISPATCH_REVISION_CONFLICT" ||
        message === "OBJECT_TASK_DISPATCH_NOT_CONTROLLABLE";
      if (stale) this.request = null;
      this.set({
        phase: "failed",
        retryAvailable: !stale,
        error: stale
          ? "任务或派发状态已变化，请刷新派发状态后重新确认。"
          : message,
      });
      return null;
    }
  };

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
        error: `派发控制回执已确认，但刷新失败：${String(error)}。请重试刷新。`,
      });
      return false;
    }
  }
}
