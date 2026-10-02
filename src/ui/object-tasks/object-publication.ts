import {
  publicationOperationSchema,
  publicationPreviewSchema,
  publicationRequestSchema,
  type PublicationReview,
  type PublicationPreview,
  type PublicationRequest,
  type PublicationOperation,
  type PublicationDecision,
} from "../../shared/object-publication";

import { ObjectAttemptFile } from "./object-attempt-file";
import { ObjectPublicationFollowup } from "./object-publication-followup";
import { ObjectPublicationDeferred } from "./object-publication-deferred";
import { ObjectCandidateFeedbackDraft } from "./object-candidate-feedback-draft";
import { ObjectPublicationApprovalDraft } from "./object-publication-approval-draft";
import { validateFinalRelocations } from "./final-relocation-draft";
import {
  followupBrowserStorage,
  type FollowupDraftStorage,
} from "./object-publication-followup-draft";
import {
  PublicationIntentStore,
  PublicationIntentConflict,
} from "./object-publication-intent";

interface State {
  busy: boolean;
  error: string;
  preview: PublicationPreview | null;
  operations: PublicationOperation[];
  retry: boolean;
  storageError: string;
  recoveryBlocked: boolean;
  restored: boolean;
  reconciled: boolean;
}
export class ObjectPublication {
  readonly feedbackDraft: ObjectCandidateFeedbackDraft;
  readonly deferred: ObjectPublicationDeferred;
  private approvals = new Map<string, ObjectPublicationApprovalDraft>();
  approvalFor(preview: PublicationPreview) {
    let draft = this.approvals.get(preview.digest);
    if (!draft) {
      draft = new ObjectPublicationApprovalDraft(preview, this.approvalStorage);
      this.approvals.set(preview.digest, draft);
    }
    return draft;
  }
  private followups = new Map<string, ObjectPublicationFollowup>();
  followupFor(op: PublicationOperation) {
    let session = this.followups.get(op.request.requestId);
    if (!session) {
      session = new ObjectPublicationFollowup(
        op,
        this.api,
        this.changed,
        this.requestId,
      );
      this.followups.set(op.request.requestId, session);
    }
    return session;
  }
  private generation = 0;
  private intent: PublicationIntentStore;
  private request: PublicationRequest | null = null;
  private abortId: string | null = null;
  private listeners = new Set<() => void>();
  private state: State = {
    busy: false,
    error: "",
    preview: null,
    operations: [],
    retry: false,
    storageError: "",
    recoveryBlocked: false,
    restored: false,
    reconciled: true,
  };
  constructor(
    readonly review: PublicationReview,
    private api: (method: string, input: unknown) => Promise<unknown>,
    private changed: () => Promise<unknown>,
    private requestId: () => string = () => crypto.randomUUID(),
    private approvalStorage?: FollowupDraftStorage,
  ) {
    this.intent = new PublicationIntentStore(
      review,
      approvalStorage ?? followupBrowserStorage,
    );
    this.feedbackDraft = new ObjectCandidateFeedbackDraft(review);
    this.deferred = new ObjectPublicationDeferred(
      review,
      api,
      this.refresh,
      requestId,
    );
    this.restore();
  }
  restore = () => {
    if (this.state.busy) return;
    try {
      const saved = this.intent.read();
      this.request = saved?.request ?? null;
      this.abortId = saved?.abort ? this.request!.requestId : null;
      this.set({
        storageError: "",
        recoveryBlocked: false,
        restored: !!this.request,
        retry: !!this.request,
        reconciled: !this.request,
        preview: null,
      });
    } catch (error: unknown) {
      this.set({
        storageError: "无法读取原发布请求：" + String(error),
        recoveryBlocked: true,
        reconciled: false,
      });
    }
  };
  private async persist(request = this.request, abortId = this.abortId) {
    if (this.state.recoveryBlocked) return false;
    try {
      await this.intent.save(request, !!abortId);
      this.set({ storageError: "" });
      return true;
    } catch (error: unknown) {
      this.set({ storageError: "无法保存原发布请求：" + String(error) });
      if (error instanceof PublicationIntentConflict)
        this.set({ recoveryBlocked: true, reconciled: false });
      return false;
    }
  }
  getSnapshot = () => this.state;
  attemptFrames(
    attemptId: string,
    previewFrame?: import("../../shared/object-publication").PreviewFrameReference,
  ) {
    return this.api("objectTask.attemptFrames", {
      projectId: this.review.projectId,
      attemptId,
      ...(previewFrame ? { previewFrame } : {}),
    });
  }
  createFeedbackFile(feedback: PublicationPreview["feedback"][number]) {
    return new ObjectAttemptFile(
      {
        projectId: this.review.projectId,
        runId: feedback.origin?.runId ?? this.review.target.runId,
        attemptId: feedback.attemptId,
      },
      this.api,
    );
  }
  publicationFrames(
    publicationRequestId: string,
    previewFrame: import("../../shared/object-publication").PreviewFrameReference,
  ) {
    return this.api("objectTask.publicationFrames", {
      projectId: this.review.projectId,
      publicationRequestId,
      previewFrame,
    });
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
    this.deferred.cancel();
    this.generation++;
    this.set({ busy: false, retry: !!this.request || !!this.abortId });
  };
  private matches(op: PublicationOperation) {
    if (
      op.request.projectId !== this.review.projectId ||
      op.request.target.taskId !== this.review.target.taskId ||
      op.request.target.runId !== this.review.target.runId ||
      op.request.target.objectId !== this.review.target.objectId
    )
      throw new Error("发布记录与当前任务不一致");
  }
  refresh = async () => {
    if (this.state.busy || this.state.recoveryBlocked) return;
    const generation = ++this.generation;
    this.set({ busy: true, error: "", reconciled: false });
    if (generation !== this.generation) return;
    try {
      const raw = await this.api("objectTask.publications", {
        projectId: this.review.projectId,
        taskId: this.review.target.taskId,
      });
      if (generation !== this.generation) return;
      const operations = publicationOperationSchema.array().parse(raw);
      const ids = new Set<string>();
      for (const op of operations) {
        this.matches(op);
        if (ids.has(op.request.requestId)) throw new Error("发布请求 ID 重复");
        ids.add(op.request.requestId);
      }
      const pending = operations.find(
        (op) => op.state === "applying" || op.state === "aborting",
      );
      if (
        operations.filter((op) => ["applying", "aborting"].includes(op.state))
          .length > 1
      )
        throw new Error("存在多个未完成发布，无法安全恢复");
      if (this.request) {
        const saved = operations.find(
          (op) => op.request.requestId === this.request?.requestId,
        );
        if (
          saved &&
          JSON.stringify(saved.request) !== JSON.stringify(this.request)
        )
          throw new Error("发布回执与请求不一致");
        if (saved && ["published", "aborted"].includes(saved.state)) {
          if (!(await this.persist(null, null)))
            throw new Error("回执已确认，请重试刷新以保存完成状态");
          if (generation !== this.generation) return;
          this.request = null;
          this.abortId = null;
        }
      }
      if (
        this.request &&
        operations.some(
          (op) =>
            op.request.requestId !== this.request!.requestId &&
            op.state !== "aborted",
        )
      )
        throw new Error("本机原发布请求与服务端发布记录冲突，请保留记录并核对");
      if (pending) {
        const aborting =
          pending.state === "aborting" ||
          this.abortId === pending.request.requestId;
        this.request = pending.request;
        this.abortId = aborting ? pending.request.requestId : null;
      }
      this.set({
        operations,
        preview: null,
        retry: !!this.request || !!this.abortId,
        reconciled: true,
      });
      if (generation !== this.generation) return;
      if (
        !pending &&
        !this.request &&
        !this.abortId &&
        !operations.some((op) => op.state === "published")
      ) {
        const rawPreview = await this.api(
          "objectTask.publicationPreview",
          this.review,
        );
        if (generation !== this.generation) return;
        const preview = publicationPreviewSchema.parse(rawPreview);
        if (JSON.stringify(preview.review) !== JSON.stringify(this.review))
          throw new Error("发布预览与审阅不一致");
        this.set({ preview });
      }
      if (generation === this.generation) this.set({ busy: false });
      if (
        generation === this.generation &&
        operations.some((op) => op.state === "published")
      ) {
        await this.refreshConsumers(generation);
      }
    } catch (error: unknown) {
      if (generation === this.generation)
        this.set({ busy: false, error: String(error) });
    }
  };
  publish = async (approval?: {
    acceptanceNote: string;
    confirmFiles: boolean;
    confirmReplacement: boolean;
    feedback: PublicationDecision[];
  }) => {
    if (
      this.state.busy ||
      this.abortId ||
      this.state.recoveryBlocked ||
      !this.state.reconciled
    )
      return;
    try {
      if (!this.request) {
        if (!approval || !this.state.preview) return;
        const preview = this.state.preview;
        if (preview.replacementRequired && !approval.confirmReplacement)
          throw new Error("请确认替换当前接受版本");
        if (
          approval.feedback.length !== preview.feedback.length ||
          preview.feedback.some(
            (item) =>
              approval.feedback.filter((d) => d.requestId === item.requestId)
                .length !== 1,
          )
        )
          throw new Error("请处置每条返工反馈");
        validateFinalRelocations(preview, approval.feedback);
        this.request = publicationRequestSchema.parse({
          ...this.review,
          ...approval,
          requestId: this.requestId(),
          previewDigest: preview.digest,
        });
      }
      await this.submit(false);
    } catch (error: unknown) {
      this.set({ error: String(error) });
    }
  };
  abort = async (requestId: string, confirmed: boolean) => {
    if (
      this.state.busy ||
      !confirmed ||
      this.state.recoveryBlocked ||
      !this.state.reconciled
    )
      return;
    if (
      !this.state.operations.some(
        (op) =>
          op.request.requestId === requestId &&
          ["applying", "aborting"].includes(op.state),
      )
    )
      return;
    this.abortId = requestId;
    this.request = this.state.operations.find(
      (op) => op.request.requestId === requestId,
    )!.request;
    await this.submit(true);
  };
  retry = async () => {
    if (this.abortId) await this.submit(true);
    else await this.publish();
  };
  private async submit(abort: boolean) {
    if (
      this.state.busy ||
      this.state.recoveryBlocked ||
      !this.state.reconciled ||
      !this.request
    )
      return;
    const request = this.request;
    const abortId = this.abortId;
    const generation = ++this.generation;
    this.set({ busy: true, error: "", retry: false });
    if (generation !== this.generation) return;
    try {
      if (!(await this.persist())) {
        if (generation === this.generation)
          this.set({ busy: false, retry: true });
        return;
      }
      if (generation !== this.generation) return;
      const raw = await this.api(
        abort ? "objectTask.abortPublication" : "objectTask.publishCandidate",
        abort
          ? {
              projectId: this.review.projectId,
              requestId: abortId,
              confirmAbort: true,
            }
          : structuredClone(request),
      );
      if (generation !== this.generation) return;
      const op = publicationOperationSchema.parse(raw);
      this.matches(op);
      if (
        abort
          ? op.request.requestId !== abortId
          : JSON.stringify(op.request) !== JSON.stringify(request)
      )
        throw new Error("发布回执与请求不一致");
      if (abort) {
        if (
          JSON.stringify(request) !== JSON.stringify(op.request) ||
          !["aborting", "aborted"].includes(op.state)
        )
          throw new Error("中止回执与请求不一致");
      }
      if (op.state === "published" || op.state === "aborted") {
        if (!(await this.persist(null, null))) {
          this.set({ reconciled: false });
          throw new Error("回执已确认，请刷新发布状态以保存完成状态");
        }
        if (generation !== this.generation) return;
        this.request = null;
        this.abortId = null;
      } else if (op.state === "aborting") {
        this.abortId = op.request.requestId;
        await this.persist();
      }
      if (generation !== this.generation) return;
      this.set({
        busy: false,
        preview: null,
        retry: !!this.request || !!this.abortId,
        error: op.error ?? "",
        operations: [
          ...this.state.operations.filter(
            (old) => old.request.requestId !== op.request.requestId,
          ),
          op,
        ],
      });
      if (op.state === "published") await this.refreshConsumers(generation);
    } catch (error: unknown) {
      if (generation === this.generation)
        this.set({ busy: false, retry: true, error: String(error) });
    }
  }
  private async refreshConsumers(generation: number) {
    try {
      if ((await this.changed()) === false) throw new Error("任务列表刷新失败");
    } catch (error: unknown) {
      if (generation === this.generation)
        this.set({ error: "发布已完成，刷新失败：" + String(error) });
    }
  }
}
