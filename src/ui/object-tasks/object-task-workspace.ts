import {
  commitObjectTaskSchema,
  objectTaskCommitReceiptSchema,
  objectTaskDraftSchema,
  objectTaskPlanSchema,
  saveObjectTaskDraftSchema,
  unlockObjectTaskDraftSchema,
  type ObjectTaskCommitReceipt,
  type ObjectTaskPlan,
} from "../../shared/object-tasks";
import { parseObjectTaskSnapshot } from "../../shared/object-task-snapshot";

export const OBJECT_TASK_DRAFT_ID = "object-task-plan";

export type { ObjectTaskWorkspaceState } from "./object-task-workspace-state";
import {
  type ObjectTaskWorkspaceState,
  emptyPlan,
  samePlan,
  errorMessage,
  conflictKind,
  workspaceFromRemote,
} from "./object-task-workspace-state";
type Api = (method: string, input: unknown) => Promise<unknown>;

export class ObjectTaskWorkspace {
  private state: ObjectTaskWorkspaceState = { kind: "loading" };
  private generation = 0;
  private editVersion = 0;
  private listeners = new Set<() => void>();

  constructor(
    readonly projectId: string,
    private readonly api: Api,
  ) {}

  getSnapshot = () => this.state;
  suggestTitle = (prompt: string, acceptance: string) =>
    this.api("objectTask.suggestTitle", {
      projectId: this.projectId,
      prompt,
      acceptance,
    });
  subscribe = (listener: () => void) => {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  };
  cancel = () => {
    this.generation++;
  };

  private set(state: ObjectTaskWorkspaceState) {
    this.state = state;
    for (const listener of this.listeners) listener();
  }

  private async readRemote() {
    const [rawSnapshot, rawDraft] = await Promise.all([
      this.api("objectTask.snapshot", { projectId: this.projectId }),
      this.api("objectTask.getDraft", {
        projectId: this.projectId,
        draftId: OBJECT_TASK_DRAFT_ID,
      }),
    ]);
    const snapshot = parseObjectTaskSnapshot(rawSnapshot, this.projectId);
    const draft =
      rawDraft === null ? null : objectTaskDraftSchema.parse(rawDraft);
    if (
      draft &&
      (draft.projectId !== this.projectId || draft.id !== OBJECT_TASK_DRAFT_ID)
    ) {
      throw new Error("对象任务草稿身份与当前项目不一致");
    }
    return { snapshot, draft };
  }

  refresh = async (): Promise<boolean> => {
    if (
      this.state.kind === "ready" &&
      (this.state.operation === "saving" ||
        this.state.operation === "committing" ||
        this.state.operation === "unlocking")
    ) {
      return false;
    }
    const generation = ++this.generation;
    const previous = this.state;
    const editVersion = this.editVersion;
    this.set(
      previous.kind === "ready"
        ? { ...previous, operation: "refreshing", error: null }
        : { kind: "loading" },
    );
    try {
      const remote = await this.readRemote();
      if (generation !== this.generation) return false;
      const latest = this.state;
      if (latest.kind !== "ready") {
        this.set(workspaceFromRemote(remote.snapshot, remote.draft));
        return true;
      }
      const keepLocal =
        (previous.kind === "ready" && previous.dirty) ||
        latest.dirty ||
        editVersion !== this.editVersion;
      if (keepLocal) {
        const planConflict =
          remote.snapshot.planRevision !== latest.planRevision;
        const draftConflict =
          !planConflict &&
          (remote.draft?.revision ?? 0) !== latest.draftRevision;
        this.set({
          ...latest,
          snapshot: remote.snapshot,
          operation: "idle",
          conflict: planConflict ? "plan" : draftConflict ? "draft" : null,
          error:
            planConflict || draftConflict
              ? "远端草稿或计划版本已变化；本地编辑仍保留。"
              : null,
        });
        return true;
      }
      this.set(workspaceFromRemote(remote.snapshot, remote.draft));
      return true;
    } catch (error: unknown) {
      if (generation !== this.generation) return false;
      const latest = this.state;
      if (
        latest.kind === "ready" &&
        ((previous.kind === "ready" && previous.dirty) ||
          latest.dirty ||
          editVersion !== this.editVersion)
      ) {
        this.set({ ...latest, operation: "idle", error: errorMessage(error) });
      } else {
        this.set({ kind: "error", message: errorMessage(error) });
      }
      return false;
    }
  };

  async reloadDiscardingLocal(): Promise<void> {
    if (
      this.state.kind === "ready" &&
      (this.state.operation === "saving" ||
        this.state.operation === "committing" ||
        this.state.operation === "unlocking")
    ) {
      return;
    }
    const generation = ++this.generation;
    const previous = this.state;
    if (previous.kind === "ready") {
      this.set({ ...previous, operation: "refreshing", error: null });
    }
    try {
      const remote = await this.readRemote();
      if (generation === this.generation) {
        this.set(workspaceFromRemote(remote.snapshot, remote.draft));
      }
    } catch (error: unknown) {
      if (generation !== this.generation) return;
      if (previous.kind === "ready") {
        this.set({
          ...previous,
          operation: "idle",
          error: errorMessage(error),
        });
      } else {
        this.set({ kind: "error", message: errorMessage(error) });
      }
    }
  }

  updatePlan(
    update: ObjectTaskPlan | ((plan: ObjectTaskPlan) => ObjectTaskPlan),
  ): void {
    if (
      this.state.kind !== "ready" ||
      this.state.operation === "committing" ||
      this.state.operation === "unlocking" ||
      this.state.committedRequestId
    ) {
      return;
    }
    let plan: ObjectTaskPlan;
    try {
      plan = typeof update === "function" ? update(this.state.plan) : update;
    } catch (error: unknown) {
      this.set({ ...this.state, error: errorMessage(error) });
      return;
    }
    this.editVersion++;
    this.set({
      ...this.state,
      plan,
      dirty:
        !samePlan(plan, this.state.savedPlan) ||
        this.state.planRevision !== this.state.savedPlanRevision,
      error: null,
      receipt: null,
    });
  }

  async save(): Promise<boolean> {
    const current = this.state;
    if (
      current.kind !== "ready" ||
      current.operation !== "idle" ||
      current.committedRequestId ||
      current.conflict
    ) {
      return false;
    }
    if (!current.dirty) return true;
    const generation = ++this.generation;
    const editVersion = this.editVersion;
    this.set({ ...current, operation: "saving", error: null });
    try {
      const plan = objectTaskPlanSchema.parse(current.plan);
      const request = saveObjectTaskDraftSchema.parse({
        projectId: this.projectId,
        draftId: OBJECT_TASK_DRAFT_ID,
        expectedRevision: current.draftRevision,
        expectedPlanRevision: current.planRevision,
        plan,
      });
      const draft = objectTaskDraftSchema.parse(
        await this.api("objectTask.saveDraft", request),
      );
      if (
        draft.projectId !== this.projectId ||
        draft.id !== OBJECT_TASK_DRAFT_ID
      ) {
        throw new Error("对象任务草稿保存回执身份不一致");
      }
      if (
        draft.revision <= current.draftRevision ||
        draft.committedRequestId ||
        draft.planRevision !== current.planRevision ||
        !samePlan(draft.plan, plan)
      ) {
        throw new Error("对象任务草稿保存回执与提交内容不一致");
      }
      if (generation !== this.generation) return false;
      const latest = this.state;
      if (latest.kind !== "ready") return false;
      const changedDuringSave = editVersion !== this.editVersion;
      this.set({
        ...latest,
        savedPlan: draft.plan,
        savedPlanRevision: draft.planRevision,
        draftRevision: draft.revision,
        planRevision: draft.planRevision,
        dirty: changedDuringSave || !samePlan(latest.plan, draft.plan),
        operation: "idle",
        conflict: null,
        error: null,
      });
      return !changedDuringSave;
    } catch (error: unknown) {
      if (generation !== this.generation) return false;
      const message = errorMessage(error);
      const latest = this.state;
      if (latest.kind === "ready") {
        this.set({
          ...latest,
          operation: "idle",
          conflict: conflictKind(message) ?? latest.conflict,
          error: message,
        });
      }
      return false;
    }
  }

  async keepLocalDraftAndRebaseRevision(): Promise<boolean> {
    const current = this.state;
    if (
      current.kind !== "ready" ||
      current.operation !== "idle" ||
      current.committedRequestId ||
      !current.conflict
    ) {
      return false;
    }
    const generation = ++this.generation;
    const editVersion = this.editVersion;
    this.set({ ...current, operation: "refreshing", error: null });
    try {
      const remote = await this.readRemote();
      if (generation !== this.generation) return false;
      const latest = this.state;
      if (latest.kind !== "ready") return false;
      if (remote.draft?.committedRequestId) {
        throw new Error(
          "远端草稿已提交并锁定；本地内容仍保留，请重新加载后开始下一份草稿。",
        );
      }
      const planChanged =
        remote.snapshot.planRevision !== current.snapshot.planRevision;
      if (planChanged || editVersion !== this.editVersion) {
        this.set({
          ...latest,
          snapshot: remote.snapshot,
          operation: "idle",
          conflict: planChanged ? "plan" : current.conflict,
          error: planChanged
            ? "计划版本再次变化；本地内容仍保留，请核对最新任务后再次确认。"
            : "本地草稿在重新检查时又发生了变化，请再次处理版本冲突。",
        });
        return false;
      }
      const savedPlan = remote.draft?.plan ?? emptyPlan();
      const savedPlanRevision =
        remote.draft?.planRevision ?? remote.snapshot.planRevision;
      this.set({
        ...latest,
        snapshot: remote.snapshot,
        draftRevision: remote.draft?.revision ?? 0,
        planRevision: remote.snapshot.planRevision,
        savedPlan,
        savedPlanRevision,
        dirty:
          !samePlan(latest.plan, savedPlan) ||
          savedPlanRevision !== remote.snapshot.planRevision,
        conflict: null,
        operation: "idle",
        error: null,
      });
      return true;
    } catch (error: unknown) {
      if (generation !== this.generation) return false;
      const latest = this.state;
      if (latest.kind === "ready") {
        this.set({ ...latest, operation: "idle", error: errorMessage(error) });
      }
      return false;
    }
  }

  startNewPlan(): void {
    if (
      this.state.kind !== "ready" ||
      this.state.operation !== "idle" ||
      this.state.committedRequestId ||
      this.state.conflict === "draft"
    ) {
      return;
    }
    this.editVersion++;
    const plan = emptyPlan();
    this.set({
      ...this.state,
      plan,
      planRevision: this.state.snapshot.planRevision,
      dirty:
        !samePlan(plan, this.state.savedPlan) ||
        this.state.snapshot.planRevision !== this.state.savedPlanRevision,
      conflict: null,
      error: null,
    });
  }

  async commit(): Promise<boolean> {
    let state = this.state;
    if (
      state.kind !== "ready" ||
      state.operation !== "idle" ||
      state.committedRequestId ||
      state.conflict ||
      state.receipt ||
      state.plan.tasks.length === 0
    )
      return false;
    if (state.dirty && !(await this.save())) return false;
    state = this.state;
    if (
      state.kind !== "ready" ||
      state.dirty ||
      state.conflict ||
      state.committedRequestId
    )
      return false;
    const generation = ++this.generation;
    const request = commitObjectTaskSchema.parse({
      projectId: this.projectId,
      requestId:
        OBJECT_TASK_DRAFT_ID +
        "." +
        state.draftRevision +
        "." +
        state.planRevision,
      draftId: OBJECT_TASK_DRAFT_ID,
      expectedDraftRevision: state.draftRevision,
      expectedPlanRevision: state.planRevision,
    });
    this.set({ ...state, operation: "committing", error: null });
    let committedReceipt: ObjectTaskCommitReceipt | null = null;
    try {
      const receipt = objectTaskCommitReceiptSchema.parse(
        await this.api("objectTask.commit", request),
      );
      if (
        receipt.projectId !== this.projectId ||
        receipt.requestId !== request.requestId ||
        receipt.draftId !== request.draftId ||
        receipt.draftRevision !== request.expectedDraftRevision ||
        receipt.previousPlanRevision !== request.expectedPlanRevision
      ) {
        throw new Error("对象任务提交回执与当前请求不一致");
      }
      committedReceipt = receipt;
      const remote = await this.readRemote();
      if (
        !remote.draft ||
        remote.draft.revision <= state.draftRevision ||
        remote.snapshot.planRevision < receipt.planRevision
      ) {
        throw new Error("提交后的草稿或计划快照未同步");
      }
      if (generation !== this.generation) return false;
      this.set({
        ...workspaceFromRemote(remote.snapshot, remote.draft),
        ...(remote.draft.committedRequestId === receipt.requestId
          ? { receipt }
          : {}),
      });
      return true;
    } catch (error: unknown) {
      if (generation !== this.generation) return false;
      const message = errorMessage(error);
      const latest = this.state;
      if (latest.kind === "ready") {
        this.set({
          ...latest,
          ...(committedReceipt
            ? {
                committedRequestId: committedReceipt.requestId,
                draftRevision: state.draftRevision + 1,
              }
            : {}),
          operation: "idle",
          conflict: committedReceipt
            ? null
            : (conflictKind(message) ?? latest.conflict),
          receipt: committedReceipt ?? latest.receipt,
          error: committedReceipt
            ? "任务已提交并锁定，但刷新失败，请重新加载：" + message
            : message,
        });
      }
      return false;
    }
  }

  async unlockDraft(): Promise<boolean> {
    const current = this.state;
    if (
      current.kind !== "ready" ||
      current.operation !== "idle" ||
      !current.committedRequestId
    )
      return false;
    const generation = ++this.generation;
    const request = unlockObjectTaskDraftSchema.parse({
      projectId: this.projectId,
      draftId: OBJECT_TASK_DRAFT_ID,
      requestId:
        "unlock." +
        OBJECT_TASK_DRAFT_ID +
        "." +
        current.draftRevision +
        "." +
        current.snapshot.planRevision,
      expectedRevision: current.draftRevision,
      expectedPlanRevision: current.snapshot.planRevision,
    });
    this.set({ ...current, operation: "unlocking", error: null });
    try {
      const draft = objectTaskDraftSchema.parse(
        await this.api("objectTask.unlockDraft", request),
      );
      if (
        draft.projectId !== this.projectId ||
        draft.id !== OBJECT_TASK_DRAFT_ID ||
        draft.revision !== current.draftRevision + 1 ||
        draft.planRevision !== request.expectedPlanRevision ||
        draft.committedRequestId ||
        !samePlan(draft.plan, emptyPlan())
      ) {
        throw new Error("下一份草稿回执与当前请求不一致");
      }
      // A replay must not hide later edits made by another window.
      const remote = await this.readRemote();
      if (!remote.draft || remote.draft.revision < draft.revision)
        throw new Error("下一份草稿尚未同步，请重新加载");
      if (generation !== this.generation) return false;
      this.set(workspaceFromRemote(remote.snapshot, remote.draft));
      return true;
    } catch (error: unknown) {
      if (generation === this.generation)
        this.set({ ...current, operation: "idle", error: errorMessage(error) });
      return false;
    }
  }
}
