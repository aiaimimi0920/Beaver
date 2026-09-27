import {
  objectRecoveryOperationSchema,
  objectRecoveryRequestSchema,
  objectRecoveryViewSchema,
  objectRecoveryDispositionOperationSchema,
  objectRecoveryDispositionRequestSchema,
  type ObjectRecoveryDispositionOperation,
  type ObjectRecoveryDispositionRequest,
  type ObjectRecoveryOperation,
  type ObjectRecoveryRequest,
  type ObjectRecoveryView,
} from "../../shared/object-recovery";

type Api = (method: string, input: unknown) => Promise<unknown>;
import { ObjectTaskResume } from "./object-task-resume";
interface State {
  phase: "loading" | "ready" | "submitting" | "failed";
  view: ObjectRecoveryView | null;
  receipt: ObjectRecoveryOperation | null;
  confirmation: ObjectRecoveryDispositionRequest | null;
  dispositionReceipt: ObjectRecoveryDispositionOperation | null;
  dispositionRetryAvailable: boolean;
  action: "verify" | "dispose" | null;
  refreshing: boolean;
  retryAvailable: boolean;
  error: string;
}
const rejections: Record<string, string> = {
  OBJECT_RECOVERY_STALE_TARGET: "核验目标已变化，请刷新记录后重新确认。",
  OBJECT_RECOVERY_NOT_REQUIRED: "当前任务尚无准备记录，请刷新记录。",
  OBJECT_RECOVERY_PENDING: "已有未完成核验，请刷新后重试已保存的请求。",
  OBJECT_RECOVERY_REQUEST_CONFLICT: "核验请求标识冲突，请刷新记录后重新确认。",
  OBJECT_RECOVERY_DISPOSITION_PENDING:
    "已有未完成处置，请刷新后重试已保存的请求。",
  OBJECT_RECOVERY_ALREADY_DISPOSED: "任务已完成处置，请刷新历史记录。",
};
const dispositionRejections: Record<string, string> = {
  OBJECT_RECOVERY_STALE_TARGET: "处置目标已变化，请刷新并重新核验。",
  OBJECT_RECOVERY_DISPOSITION_REVERIFY_REQUIRED:
    "处置需要更新的核验，请刷新并重新核验。",
  OBJECT_RECOVERY_DISPOSITION_PENDING:
    "已有未完成处置，请刷新后重试已保存的请求。",
  OBJECT_RECOVERY_DISPOSITION_REQUEST_CONFLICT:
    "处置请求标识冲突，请刷新并重新确认。",
};

export class ObjectTaskRecovery {
  readonly resume: ObjectTaskResume;
  private request: ObjectRecoveryRequest | null = null;
  private dispositionRequest: ObjectRecoveryDispositionRequest | null = null;
  private confirmation: ObjectRecoveryDispositionRequest | null = null;
  private generation = 0;
  private listeners = new Set<() => void>();
  private state: State = {
    phase: "loading",
    view: null,
    receipt: null,
    confirmation: null,
    dispositionReceipt: null,
    dispositionRetryAvailable: false,
    action: null,
    refreshing: false,
    retryAvailable: false,
    error: "",
  };

  constructor(
    readonly projectId: string,
    readonly taskId: string,
    readonly objectId: string,
    readonly runId: string,
    private readonly api: Api,
    private readonly requestId: () => string = () => crypto.randomUUID(),
    onStarted: (attemptId: string) => void = () => {},
  ) {
    this.resume = new ObjectTaskResume(
      {
        projectId,
        view: () => this.state.view,
        available: this.resumeAvailable,
        changed: () => this.set({}),
        refresh: this.refresh,
        started: onStarted,
      },
      api,
      requestId,
    );
  }

  resumeAvailable = () =>
    this.state.phase !== "submitting" &&
    !this.state.refreshing &&
    !this.request &&
    !this.dispositionRequest &&
    !this.confirmation;

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
        phase: "failed",
        refreshing: false,
        action: null,
        view: null,
        retryAvailable: !!this.request,
        dispositionRetryAvailable: !!this.dispositionRequest,
        error: "已停止等待。已发出的请求仍可能完成，可重新查询或重试原请求。",
      });
    }
    this.resume.cancel();
  };

  refresh = async (): Promise<boolean> => {
    if (this.state.phase === "submitting" || this.state.refreshing)
      return false;
    return this.query(++this.generation);
  };

  private async query(generation: number): Promise<boolean> {
    if (generation !== this.generation) return false;
    this.set({
      phase: "loading",
      refreshing: true,
      action: null,
      view: null,
      error: "",
    });
    if (generation !== this.generation) return false;
    try {
      const response = await this.api("objectTask.recovery", {
        projectId: this.projectId,
        taskId: this.taskId,
      });
      if (generation !== this.generation) return false;
      const view = objectRecoveryViewSchema.nullable().parse(response);
      const operation = view?.operation;
      const disposition = view?.disposition;
      if (
        view &&
        (view.target.taskId !== this.taskId ||
          view.target.objectId !== this.objectId ||
          view.target.runId !== this.runId ||
          (operation && operation.request.projectId !== this.projectId) ||
          (disposition && disposition.request.projectId !== this.projectId))
      )
        throw new Error("核验记录与当前项目或任务不一致");
      if (
        operation &&
        this.request?.requestId === operation.request.requestId
      ) {
        if (JSON.stringify(operation.request) !== JSON.stringify(this.request))
          throw new Error("核验记录与待确认请求不一致");
        if (operation.result) this.request = null;
      }
      if (!this.request && operation && !operation.result)
        this.request = structuredClone(operation.request);
      if (
        disposition &&
        this.dispositionRequest?.requestId === disposition.request.requestId
      ) {
        if (
          JSON.stringify(disposition.request) !==
          JSON.stringify(this.dispositionRequest)
        )
          throw new Error("处置记录与待确认请求不一致");
        if (disposition.result) this.dispositionRequest = null;
      }
      if (!this.dispositionRequest && disposition && !disposition.result)
        this.dispositionRequest = structuredClone(disposition.request);
      if (disposition && disposition.result?.status !== "blocked")
        this.confirmation = null;
      this.resume.observe(view);
      this.set({
        phase: "ready",
        view,
        refreshing: false,
        retryAvailable: !!this.request,
        receipt: operation?.result ? operation : this.state.receipt,
        confirmation: structuredClone(this.confirmation),
        dispositionRetryAvailable: !!this.dispositionRequest,
        dispositionReceipt: disposition?.result
          ? disposition
          : this.state.dispositionReceipt,
      });
      return true;
    } catch (error: unknown) {
      if (generation !== this.generation) return false;
      this.set({
        phase: "failed",
        refreshing: false,
        retryAvailable: !!this.request,
        dispositionRetryAvailable: !!this.dispositionRequest,
        error: `${this.state.dispositionReceipt ? "处置回执已保留，当前记录刷新失败：" : this.state.receipt ? "核验回执已保留，当前记录刷新失败：" : ""}${String(error)}`,
      });
      return false;
    }
  }

  canVerify = (): boolean => {
    if (this.resume.blocks()) return false;
    const disposition = this.state.view?.disposition;
    if (
      this.state.phase === "submitting" ||
      this.state.refreshing ||
      this.confirmation ||
      this.dispositionRequest ||
      (disposition && disposition.result?.status !== "blocked") ||
      this.state.dispositionReceipt?.result?.status ===
        "cancelledAndRetained" ||
      this.state.dispositionReceipt?.result?.status ===
        "cancelledAndWorkspaceRemoved"
    )
      return false;
    return (
      !!this.request ||
      (this.state.phase === "ready" &&
        !!this.state.view &&
        this.state.view.target.recoveryGeneration < Number.MAX_SAFE_INTEGER)
    );
  };

  blocksInterrupt = (): boolean => {
    if (this.resume.blocks()) return true;
    const disposition = this.state.view?.disposition;
    return (
      !!this.confirmation ||
      !!this.dispositionRequest ||
      this.state.dispositionReceipt?.result?.status ===
        "cancelledAndRetained" ||
      (!!disposition && disposition.result?.status !== "blocked")
    );
  };

  async verify(): Promise<ObjectRecoveryOperation | null> {
    if (!this.canVerify()) return null;
    if (!this.request) {
      if (
        this.state.phase !== "ready" ||
        !this.state.view ||
        this.state.view.target.recoveryGeneration >= Number.MAX_SAFE_INTEGER
      )
        return null;
      this.request = objectRecoveryRequestSchema.parse({
        projectId: this.projectId,
        requestId: this.requestId(),
        target: this.state.view.target,
      });
    }
    const request = this.request;
    const generation = ++this.generation;
    this.set({
      phase: "submitting",
      action: "verify",
      retryAvailable: false,
      error: "",
    });
    if (generation !== this.generation) return null;
    try {
      const response = await this.api(
        "objectTask.verifyRecovery",
        structuredClone(request),
      );
      if (generation !== this.generation) return null;
      const receipt = objectRecoveryOperationSchema.parse(response);
      if (
        !receipt.result ||
        JSON.stringify(receipt.request) !== JSON.stringify(request)
      )
        throw new Error("核验回执与当前请求不一致或尚未完成");
      this.request = null;
      this.set({
        phase: "ready",
        action: null,
        receipt,
        view: null,
        retryAvailable: false,
      });
      await this.query(generation);
      return generation === this.generation ? receipt : null;
    } catch (error: unknown) {
      if (generation !== this.generation) return null;
      const message = error instanceof Error ? error.message : String(error);
      const rejection = Object.hasOwn(rejections, message)
        ? rejections[message]
        : undefined;
      if (rejection) this.request = null;
      this.set({
        phase: "failed",
        action: null,
        view: null,
        retryAvailable: !!this.request,
        error: rejection ?? message,
      });
      return null;
    }
  }

  private prepareDisposition = (
    choice: "cancelAndKeep" | "cancelAndRemoveWorkspace",
  ): boolean => {
    const view = this.state.view;
    if (
      this.state.phase !== "ready" ||
      this.resume.blocks() ||
      this.state.refreshing ||
      !view?.canDispose ||
      !view.operation?.result ||
      this.request ||
      this.dispositionRequest ||
      this.confirmation
    )
      return false;
    this.confirmation = objectRecoveryDispositionRequestSchema.parse({
      projectId: this.projectId,
      requestId: this.requestId(),
      target: view.target,
      verificationRequestId: view.operation.request.requestId,
      choice,
    });
    this.set({ confirmation: structuredClone(this.confirmation), error: "" });
    return true;
  };

  prepareCancelAndKeep = (): boolean =>
    this.prepareDisposition("cancelAndKeep");

  prepareCancelAndRemoveWorkspace = (): boolean =>
    this.prepareDisposition("cancelAndRemoveWorkspace");

  dismissDisposition = () => {
    if (this.state.phase === "submitting") return;
    this.confirmation = null;
    this.set({ confirmation: null });
  };

  async dispose(): Promise<ObjectRecoveryDispositionOperation | null> {
    if (
      this.state.phase === "submitting" ||
      this.state.refreshing ||
      this.request
    )
      return null;
    if (!this.dispositionRequest) {
      if (!this.confirmation || this.state.phase !== "ready") return null;
      this.dispositionRequest = this.confirmation;
      this.confirmation = null;
    }
    const request = this.dispositionRequest;
    const generation = ++this.generation;
    this.set({
      phase: "submitting",
      action: "dispose",
      confirmation: null,
      dispositionRetryAvailable: false,
      error: "",
    });
    if (generation !== this.generation) return null;
    try {
      const response = await this.api(
        "objectTask.disposeRecovery",
        structuredClone(request),
      );
      if (generation !== this.generation) return null;
      const receipt = objectRecoveryDispositionOperationSchema.parse(response);
      if (
        !receipt.result ||
        JSON.stringify(receipt.request) !== JSON.stringify(request)
      )
        throw new Error("处置回执与当前请求不一致或尚未完成");
      this.dispositionRequest = null;
      this.set({
        phase: "ready",
        action: null,
        dispositionReceipt: receipt,
        view: null,
        dispositionRetryAvailable: false,
      });
      await this.query(generation);
      return generation === this.generation ? receipt : null;
    } catch (error: unknown) {
      if (generation !== this.generation) return null;
      const message = error instanceof Error ? error.message : String(error);
      const rejection = Object.hasOwn(dispositionRejections, message)
        ? dispositionRejections[message]
        : undefined;
      if (rejection) this.dispositionRequest = null;
      this.set({
        phase: "failed",
        action: null,
        view: null,
        dispositionRetryAvailable: !!this.dispositionRequest,
        error: rejection ?? message,
      });
      return null;
    }
  }
}
