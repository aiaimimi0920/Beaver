import {
  parseQueueView,
  previewQueueMove,
  queueMoveSchema,
  queueReceiptSchema,
  type QueueView,
  type QueueMove,
  type QueueReceipt,
} from "../../shared/object-task-queue";

type Api = (method: string, input: unknown) => Promise<unknown>;
interface State {
  phase: "loading" | "ready" | "submitting" | "failed";
  view: QueueView | null;
  proposal: QueueMove | null;
  receipt: QueueReceipt | null;
  error: string;
  retry: boolean;
}
type Pending =
  | { method: "objectTask.reorder"; input: QueueMove }
  | {
      method: "objectTask.enqueue";
      input: { projectId: string; taskIds: string[] };
    };

export class ObjectTaskQueueSession {
  private state: State = {
    phase: "loading",
    view: null,
    proposal: null,
    receipt: null,
    error: "",
    retry: false,
  };
  private listeners = new Set<() => void>();
  private generation = 0;
  private disposed = false;
  private pending: Pending | null = null;
  constructor(
    readonly projectId: string,
    private api: Api,
    private requestId: () => string = () => crypto.randomUUID(),
  ) {}
  getSnapshot = () => this.state;
  subscribe = (listener: () => void) => {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  };
  private set(next: Partial<State>) {
    this.state = { ...this.state, ...next };
    for (const listener of this.listeners) listener();
  }
  cancel = () => {
    this.generation++;
    this.disposed = true;
    this.pending = null;
  };
  refresh = async () => {
    if (this.disposed || this.pending || this.state.phase === "submitting")
      return;
    const generation = ++this.generation;
    this.set({ phase: "loading", proposal: null, error: "", retry: false });
    try {
      const view = parseQueueView(
        await this.api("objectTask.queueView", { projectId: this.projectId }),
        this.projectId,
      );
      if (generation === this.generation) this.set({ phase: "ready", view });
    } catch (error) {
      if (generation === this.generation)
        this.set({ phase: "failed", error: String(error) });
    }
  };
  notified = () => {
    if (!this.state.proposal && !this.pending && this.state.phase !== "loading")
      void this.refresh();
  };
  preview = (taskId: string, index: number) => {
    if (
      this.disposed ||
      this.state.phase !== "ready" ||
      !this.state.view ||
      this.pending
    )
      return;
    try {
      const move = previewQueueMove(this.state.view, taskId, index);
      const proposal = queueMoveSchema.parse({
        projectId: this.projectId,
        taskId,
        expectedVersion: this.state.view.version,
        requestId: this.requestId(),
        previousTaskId: move.previousTaskId,
        nextTaskId: move.nextTaskId,
      });
      this.set({ proposal, error: "" });
    } catch (error) {
      this.set({ proposal: null, error: String(error) });
    }
  };
  discard = () => {
    if (!this.pending && this.state.phase === "ready")
      this.set({ proposal: null, error: "" });
  };
  confirm = async () => {
    if (
      this.disposed ||
      this.state.phase !== "ready" ||
      !this.state.proposal ||
      this.pending
    )
      return;
    this.pending = {
      method: "objectTask.reorder",
      input: structuredClone(this.state.proposal),
    };
    await this.submit();
  };
  enqueue = async (taskId: string) => {
    if (
      this.disposed ||
      this.state.phase !== "ready" ||
      this.state.proposal ||
      this.pending
    )
      return;
    this.pending = {
      method: "objectTask.enqueue",
      input: { projectId: this.projectId, taskIds: [taskId] },
    };
    await this.submit();
  };
  retry = async () => {
    if (this.state.retry && this.pending) await this.submit();
  };
  private async submit() {
    const pending = this.pending;
    if (!pending || this.state.phase === "submitting") return;
    const generation = ++this.generation;
    this.set({ phase: "submitting", error: "", retry: false });
    try {
      const response = await this.api(
        pending.method,
        structuredClone(pending.input),
      );
      if (generation !== this.generation) return;
      let receipt = this.state.receipt;
      if (pending.method === "objectTask.reorder") {
        receipt = queueReceiptSchema.parse(response);
        const parsed = queueMoveSchema.parse(pending.input);
        if (JSON.stringify(receipt.request) !== JSON.stringify(parsed))
          throw new Error("Queue receipt request mismatch");
      }
      this.pending = null;
      this.set({ phase: "ready", proposal: null, receipt });
      await this.refresh();
    } catch (error) {
      if (generation !== this.generation) return;
      const message = String(error);
      const rejected =
        /OBJECT_TASK_QUEUE_(VERSION_CONFLICT|ANCHOR_CONFLICT|BLOCKED_HEAD|NOT_REORDERABLE|REQUEST_CONFLICT)/.test(
          message,
        );
      if (rejected) this.pending = null;
      this.set({
        phase: "failed",
        retry: !rejected,
        proposal: null,
        error: rejected
          ? "队列已变化或移动被阻止，请刷新后重新确认。 " + message
          : "结果未确认，请按原请求重试。 " + message,
      });
    }
  }
}
