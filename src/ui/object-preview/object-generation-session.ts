import { z } from "zod";
import {
  objectTaskPlanSchema,
  objectTaskDraftSchema,
  objectTaskCommitReceiptSchema,
  type ObjectTaskCommitReceipt,
} from "../../shared/object-tasks";
import { parseObjectTaskSnapshot } from "../../shared/object-task-snapshot";
import type { DraftStorage } from "./object-import-draft";

const fieldsSchema = z.object({
  name: z.string().max(256),
  category: z.string().min(1).max(128),
  prompt: z.string().max(20000),
  acceptance: z.string().max(10000),
});
const savedSchema = z.object({
  version: z.literal(1),
  projectId: z.string(),
  id: z
    .string()
    .regex(/^generation-[A-Za-z0-9-]+$/)
    .max(100),
  fields: fieldsSchema,
  pending: z
    .object({
      plan: objectTaskPlanSchema,
      planRevision: z.number().int().nonnegative().nullable(),
    })
    .nullable(),
  receipt: objectTaskCommitReceiptSchema.nullable(),
});
type Saved = z.infer<typeof savedSchema>;
type Fields = Saved["fields"];
type Api = (method: string, input: unknown) => Promise<unknown>;
const message = (error: unknown) =>
  error instanceof Error ? error.message : String(error);

export class ObjectGenerationSession {
  private saved: Saved;
  private listeners = new Set<() => void>();
  private state: { saved: Saved; busy: boolean; error: string };
  private loadFailed = false;
  private key: string;

  constructor(
    readonly projectId: string,
    private api: Api,
    private storage: DraftStorage,
    id = `generation-${crypto.randomUUID()}`,
  ) {
    this.key = `beaver.object-generation.v1.${projectId}`;
    this.saved = {
      version: 1,
      projectId,
      id,
      fields: { name: "", category: "模型", prompt: "", acceptance: "" },
      pending: null,
      receipt: null,
    };
    let error = "";
    try {
      const raw = storage.getItem(this.key);
      if (raw) {
        const saved = savedSchema.parse(JSON.parse(raw));
        if (saved.projectId !== projectId) throw new Error("草稿项目不匹配");
        this.saved = saved;
        if (saved.receipt) this.verifyReceipt(saved.receipt);
      }
    } catch (cause) {
      this.loadFailed = true;
      this.saved = { ...this.saved, receipt: null };
      error = `无法恢复草稿，请恢复本地存储后重新打开：${message(cause)}`;
    }
    this.state = { saved: this.saved, busy: false, error };
  }

  getSnapshot = () => this.state;
  subscribe = (listener: () => void) => {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  };
  private publish(busy = false, error = "") {
    this.state = { saved: this.saved, busy, error };
    for (const listener of this.listeners) listener();
  }
  private persist() {
    if (this.loadFailed) throw new Error("草稿尚未恢复，已阻止提交");
    this.storage.setItem(this.key, JSON.stringify(this.saved));
  }
  edit(fields: Partial<Fields>) {
    if (
      this.state.busy ||
      this.saved.pending ||
      this.saved.receipt ||
      this.loadFailed
    )
      return;
    this.saved = { ...this.saved, fields: { ...this.saved.fields, ...fields } };
    try {
      this.persist();
      this.publish();
    } catch (cause) {
      this.publish(false, `草稿未保存：${message(cause)}`);
    }
  }
  newDraft() {
    if (!this.saved.receipt || this.state.busy) return;
    const previous = this.saved;
    this.saved = {
      ...this.saved,
      id: `generation-${crypto.randomUUID()}`,
      fields: { name: "", category: "模型", prompt: "", acceptance: "" },
      pending: null,
      receipt: null,
    };
    try {
      this.persist();
      this.publish();
    } catch (cause) {
      this.saved = previous;
      this.publish(false, message(cause));
    }
  }

  private verifyReceipt(receipt: ObjectTaskCommitReceipt) {
    const { id, pending } = this.saved;
    if (
      !pending ||
      receipt.projectId !== this.projectId ||
      receipt.requestId !== id ||
      receipt.draftId !== id ||
      receipt.draftRevision !== 1 ||
      receipt.previousPlanRevision !== pending.planRevision ||
      receipt.objectIds.length !== 1 ||
      receipt.objectIds[0] !== `${id}.object` ||
      receipt.runs.length !== 1 ||
      receipt.runs[0]?.projectId !== this.projectId ||
      receipt.runs[0]?.objectId !== `${id}.object` ||
      receipt.runs[0]?.mediumTaskId !== `${id}.medium` ||
      JSON.stringify([...receipt.taskIds].sort()) !==
        JSON.stringify(pending.plan.tasks.map((task) => task.id).sort())
    ) {
      throw new Error("生成回执身份不匹配；保留原请求以便重试");
    }
  }

  submit = async () => {
    if (this.state.busy || this.saved.receipt || this.loadFailed) return;
    this.publish(true);
    try {
      this.persist();
      const { id, fields } = this.saved;
      if (!this.saved.pending) {
        if (!fields.prompt.trim()) throw new Error("请输入任务描述");
        const name = fields.name.trim() || fields.prompt.trim().slice(0, 256);
        const common = {
          title: name,
          prompt: fields.prompt.trim(),
          acceptance: fields.acceptance,
          objectId: `${id}.object`,
        };
        const plan = objectTaskPlanSchema.parse({
          objects: [{ id: common.objectId, name, category: fields.category }],
          tasks: [
            {
              ...common,
              id: `${id}.medium`,
              position: 0,
              granularity: "medium",
              baseline: { basePolicy: "empty" },
            },
            {
              ...common,
              id: `${id}.fine`,
              position: 1,
              granularity: "fine",
              parentTaskId: `${id}.medium`,
              stageId: `${id}.stage`,
            },
          ],
        });
        this.saved = {
          ...this.saved,
          pending: { plan, planRevision: null },
        };
        // Freeze the exact identity and payload before the first remote write.
        this.persist();
      }
      if (this.saved.pending!.planRevision === null) {
        const snapshot = parseObjectTaskSnapshot(
          await this.api("objectTask.snapshot", { projectId: this.projectId }),
          this.projectId,
        );
        this.saved = {
          ...this.saved,
          pending: {
            ...this.saved.pending!,
            planRevision: snapshot.planRevision,
          },
        };
        this.persist();
      }
      const pending = this.saved.pending!;
      const input = { projectId: this.projectId, draftId: id };
      const remote = await this.api("objectTask.getDraft", input);
      const draft = objectTaskDraftSchema.parse(
        remote ??
          (await this.api("objectTask.saveDraft", {
            ...input,
            expectedRevision: 0,
            expectedPlanRevision: pending.planRevision,
            plan: pending.plan,
          })),
      );
      if (
        draft.projectId !== this.projectId ||
        draft.id !== id ||
        (draft.committedRequestId
          ? draft.committedRequestId !== id || draft.revision !== 2
          : draft.revision !== 1) ||
        draft.planRevision !== pending.planRevision ||
        JSON.stringify(draft.plan) !== JSON.stringify(pending.plan)
      ) {
        throw new Error("准备记录与冻结草稿不一致；已停止提交");
      }
      const receipt = objectTaskCommitReceiptSchema.parse(
        await this.api("objectTask.commit", {
          ...input,
          requestId: id,
          expectedDraftRevision: 1,
          expectedPlanRevision: pending.planRevision,
        }),
      );
      this.verifyReceipt(receipt);
      this.saved = { ...this.saved, receipt };
      this.persist();
      this.publish();
    } catch (cause) {
      this.publish(
        false,
        `提交未确认：${message(cause)}。草稿及原请求保留；重试不会创建第二份对象。`,
      );
    }
  };
}
