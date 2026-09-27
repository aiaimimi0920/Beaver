import {
  objectRecoveryResumeOperationSchema,
  objectRecoveryResumeRequestSchema,
  type ObjectRecoveryResumeOperation,
  type ObjectRecoveryResumeRequest,
  type ObjectRecoveryView,
  type ObjectStageApproval,
  type ObjectCandidateRework,
} from "../../shared/object-recovery";

interface Host {
  projectId: string;
  view: () => ObjectRecoveryView | null;
  available: () => boolean;
  changed: () => void;
  refresh: () => Promise<boolean>;
  started: (attemptId: string) => void;
}
interface State {
  confirmation: ObjectRecoveryResumeRequest | null;
  receipt: ObjectRecoveryResumeOperation | null;
  submitting: boolean;
  retryAvailable: boolean;
  error: string;
}

export class ObjectTaskResume {
  private generation = 0;
  private request: ObjectRecoveryResumeRequest | null = null;
  private seenAttempt: string | null = null;
  private state: State = {
    confirmation: null,
    receipt: null,
    submitting: false,
    retryAvailable: false,
    error: "",
  };
  constructor(
    private readonly host: Host,
    private readonly api: (method: string, input: unknown) => Promise<unknown>,
    private readonly requestId: () => string,
  ) {}
  getSnapshot = () => this.state;
  private set(patch: Partial<State>) {
    this.state = { ...this.state, ...patch };
    this.host.changed();
  }
  blocks = () =>
    !!this.request || !!this.state.confirmation || this.state.submitting;
  canPrepare = () =>
    !this.blocks() && this.host.available() && !!this.host.view()?.canResume;
  prepare = (): boolean => {
    const view = this.host.view();
    if (!this.canPrepare() || !view?.operation) return false;
    this.set({
      confirmation: objectRecoveryResumeRequestSchema.parse({
        projectId: this.host.projectId,
        requestId: this.requestId(),
        target: view.target,
        verificationRequestId: view.operation.request.requestId,
      }),
      error: "",
    });
    return true;
  };
  prepareAdvance = (advance: ObjectStageApproval): boolean => {
    return this.preparePurpose({ advance });
  };
  prepareRework = (rework: ObjectCandidateRework): boolean => {
    return this.preparePurpose({ rework });
  };
  private preparePurpose = (purpose: {
    advance?: ObjectStageApproval;
    rework?: ObjectCandidateRework;
  }): boolean => {
    const view = this.host.view();
    if (
      this.blocks() ||
      !this.host.available() ||
      !view?.canDispose ||
      !view.operation?.result
    )
      return false;
    const parsed = objectRecoveryResumeRequestSchema.safeParse({
      projectId: this.host.projectId,
      requestId: this.requestId(),
      target: view.target,
      verificationRequestId: view.operation.request.requestId,
      ...purpose,
    });
    if (!parsed.success) return false;
    this.set({ confirmation: parsed.data, error: "" });
    return true;
  };
  cancel = () => {
    this.generation++;
    if (
      !this.state.submitting &&
      !this.state.confirmation &&
      this.state.retryAvailable === !!this.request
    )
      return;
    this.set({
      submitting: false,
      confirmation: null,
      retryAvailable: !!this.request,
    });
  };
  dismiss = () => {
    if (!this.state.submitting) this.set({ confirmation: null });
  };
  observe(view: ObjectRecoveryView | null) {
    const operation = view?.resume;
    if (!operation) return;
    if (operation.request.projectId !== this.host.projectId)
      throw new Error("恢复执行记录与当前项目不一致");
    if (this.request?.requestId === operation.request.requestId) {
      if (JSON.stringify(operation.request) !== JSON.stringify(this.request))
        throw new Error("恢复执行记录与原请求不一致");
      if (operation.result) this.request = null;
    }
    if (!this.request && !operation.result)
      this.request = structuredClone(operation.request);
    this.set({
      receipt: operation.result ? operation : this.state.receipt,
      retryAvailable: !!this.request,
      confirmation:
        operation.result?.status === "started" ? null : this.state.confirmation,
    });
    this.started(operation);
  }
  private started(operation: ObjectRecoveryResumeOperation) {
    if (
      operation.result?.status !== "started" ||
      this.seenAttempt === operation.result.attemptId
    )
      return;
    this.seenAttempt = operation.result.attemptId;
    this.host.started(operation.result.attemptId);
  }
  async submit(): Promise<ObjectRecoveryResumeOperation | null> {
    if (this.state.submitting || !this.host.available()) return null;
    if (!this.request) {
      if (!this.state.confirmation) return null;
      this.request = structuredClone(this.state.confirmation);
    }
    const request = this.request;
    const generation = ++this.generation;
    this.set({
      submitting: true,
      confirmation: null,
      retryAvailable: false,
      error: "",
    });
    try {
      if (generation !== this.generation) return null;
      const response = await this.api(
        request.advance
          ? "objectTask.advanceAttempt"
          : request.rework
            ? "objectTask.reworkCandidate"
            : "objectTask.resumeRecovery",
        structuredClone(request),
      );
      if (generation !== this.generation) return null;
      const receipt = objectRecoveryResumeOperationSchema.parse(response);
      if (
        !receipt.result ||
        JSON.stringify(receipt.request) !== JSON.stringify(request)
      )
        throw new Error("恢复执行回执与原请求不一致或尚未完成");
      this.request = null;
      this.set({ receipt, submitting: false, retryAvailable: false });
      if (generation !== this.generation) return receipt;
      this.started(receipt);
      if (!(await this.host.refresh()) && generation === this.generation)
        this.set({
          error: "恢复执行回执已保留，当前记录刷新失败，请刷新核验记录。",
        });
      return receipt;
    } catch (error: unknown) {
      if (generation !== this.generation) return null;
      const message = error instanceof Error ? error.message : String(error);
      if (
        [
          "OBJECT_RECOVERY_STALE_TARGET",
          "OBJECT_RECOVERY_RESUME_REVERIFY_REQUIRED",
          "OBJECT_RECOVERY_RESUME_TERMINAL_REQUIRED",
          "OBJECT_RECOVERY_RESUME_REQUEST_CONFLICT",
          "OBJECT_RECOVERY_ALREADY_DISPOSED",
          "INVALID_OBJECT_STAGE_APPROVAL",
          "OBJECT_STAGE_SUCCESS_REQUIRED",
          "OBJECT_STAGE_PASSING_REPORT_REQUIRED",
          "OBJECT_STAGE_CURRENT_MISSING",
          "OBJECT_STAGE_ORDER_MISMATCH",
          "OBJECT_STAGE_NO_SUCCESSOR",
          "OBJECT_STAGE_SUCCESSOR_MISMATCH",
          "OBJECT_STAGE_DEPENDENCY_NOT_ACCEPTED",
          "INVALID_OBJECT_CANDIDATE_REWORK",
          "INVALID_OBJECT_REWORK_IMAGE",
          "INVALID_OBJECT_REWORK_IMAGE_REGION",
          "OBJECT_REWORK_IMAGE_SOURCE_MISMATCH",
          "OBJECT_CANDIDATE_REVIEW_REQUIRED",
          "OBJECT_CANDIDATE_REWORK_SOURCE_MISMATCH",
          "OBJECT_CANDIDATE_RECORD_CHANGED",
          "OBJECT_CANDIDATE_REWORK_PROMPT_TOO_LONG",
        ].includes(message)
      )
        this.request = null;
      // Ambiguous responses retain the exact authorization, never mint a new request.
      this.set({
        submitting: false,
        retryAvailable: !!this.request,
        error: message,
      });
      if (!this.request) await this.host.refresh();
      return null;
    }
  }
}
