import type { PublicationOperation } from "../../shared/object-publication";
import {
  followupBrowserStorage,
  loadFollowupDraft,
  saveFollowupDraft,
  type FollowupDraftStorage,
} from "./object-publication-followup-draft";
import {
  publicationFollowupRequestSchema,
  publicationFollowupReceiptSchema,
  type PublicationFollowupRequest,
  type PublicationFollowupReceipt,
} from "../../shared/object-publication-followup";

type Draft = Pick<
  PublicationFollowupRequest,
  "title" | "feedback" | "acceptance" | "previewFrame"
>;
interface State {
  draft: Draft;
  busy: boolean;
  error: string;
  retry: boolean;
  storageError: string;
  recoveryBlocked: boolean;
  restored: boolean;
  receipts: PublicationFollowupReceipt[];
}
export class ObjectPublicationFollowup {
  private state: State = {
    draft: { title: "", feedback: "", acceptance: "" },
    busy: false,
    error: "",
    retry: false,
    storageError: "",
    recoveryBlocked: false,
    restored: false,
    receipts: [],
  };
  private request: PublicationFollowupRequest | null = null;
  private generation = 0;
  private listeners = new Set<() => void>();
  constructor(
    readonly publication: PublicationOperation,
    private api: (method: string, input: unknown) => Promise<unknown>,
    private changed: () => Promise<unknown>,
    private requestId: () => string = () => crypto.randomUUID(),
    private storage: FollowupDraftStorage = followupBrowserStorage,
  ) {
    this.restore();
  }
  restore = () => {
    if (this.state.busy) return;
    try {
      const saved = loadFollowupDraft(this.publication, this.storage);
      this.request = saved?.request ?? null;
      this.set({
        draft: saved?.draft ?? { title: "", feedback: "", acceptance: "" },
        retry: !!this.request,
        restored: !!saved,
        recoveryBlocked: false,
        storageError: "",
      });
    } catch {
      this.set({
        recoveryBlocked: true,
        storageError:
          "无法恢复本地后续任务草稿；原记录已保留，请恢复存储后重新读取，避免重复创建。",
      });
    }
  };
  private persist() {
    try {
      saveFollowupDraft(
        this.publication,
        this.storage,
        this.state.draft,
        this.request,
      );
      this.set({ storageError: "" });
      return true;
    } catch {
      this.set({
        storageError:
          "无法保存本地后续任务草稿；请恢复存储后重试，关闭窗口可能丢失输入。",
      });
      return false;
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
  edit = (draft: Partial<Draft>) => {
    if (!this.state.busy && !this.request && !this.state.recoveryBlocked) {
      this.set({ draft: { ...this.state.draft, ...draft } });
      this.persist();
    }
  };
  cancel = () => {
    this.generation++;
    this.set({ busy: false, retry: !!this.request });
  };
  frames = () =>
    this.api("objectTask.publicationFrames", {
      projectId: this.publication.request.projectId,
      publicationRequestId: this.publication.request.requestId,
    });
  private matches(receipt: PublicationFollowupReceipt) {
    const op = this.publication;
    if (
      receipt.request.projectId !== op.request.projectId ||
      receipt.request.publicationRequestId !== op.request.requestId ||
      receipt.request.versionId !== op.versionId ||
      JSON.stringify(receipt.source) !== JSON.stringify(op.request.target)
    )
      throw new Error("后续任务回执与发布来源不一致");
  }
  refresh = async () => {
    if (this.state.busy) return;
    const generation = ++this.generation;
    this.set({ busy: true, error: "" });
    try {
      const raw = await this.api("objectTask.publicationFollowups", {
        projectId: this.publication.request.projectId,
        publicationRequestId: this.publication.request.requestId,
      });
      if (generation !== this.generation) return;
      const receipts = publicationFollowupReceiptSchema.array().parse(raw);
      for (const receipt of receipts) this.matches(receipt);
      const saved = receipts.find(
        (item) => item.request.requestId === this.request?.requestId,
      );
      if (saved) {
        if (JSON.stringify(saved.request) !== JSON.stringify(this.request))
          throw new Error("后续任务回执与原请求不一致");
        this.request = null;
        this.set({ draft: { title: "", feedback: "", acceptance: "" } });
        this.persist();
      }
      this.set({ receipts, busy: false, retry: !!this.request });
      if (saved) await this.refreshConsumers(generation);
    } catch (error: unknown) {
      if (generation === this.generation)
        this.set({ busy: false, error: String(error) });
    }
  };
  create = async () => {
    if (
      this.state.busy ||
      this.state.recoveryBlocked ||
      this.publication.state !== "published"
    )
      return;
    const generation = ++this.generation;
    try {
      if (!this.request)
        this.request = publicationFollowupRequestSchema.parse({
          projectId: this.publication.request.projectId,
          requestId: this.requestId(),
          publicationRequestId: this.publication.request.requestId,
          versionId: this.publication.versionId,
          ...this.state.draft,
        });
      const request = this.request;
      if (!this.persist()) {
        this.set({ retry: true });
        return;
      }
      this.set({ busy: true, error: "", retry: false });
      const raw = await this.api(
        "objectTask.createPublicationFollowup",
        structuredClone(request),
      );
      if (generation !== this.generation) return;
      const receipt = publicationFollowupReceiptSchema.parse(raw);
      this.matches(receipt);
      if (JSON.stringify(receipt.request) !== JSON.stringify(request))
        throw new Error("后续任务回执与原请求不一致");
      this.request = null;
      this.set({
        busy: false,
        retry: false,
        draft: { title: "", feedback: "", acceptance: "" },
        receipts: [
          ...this.state.receipts.filter(
            (item) => item.request.requestId !== request.requestId,
          ),
          receipt,
        ],
      });
      this.persist();
      await this.refreshConsumers(generation);
    } catch (error: unknown) {
      if (generation === this.generation)
        this.set({ busy: false, retry: !!this.request, error: String(error) });
    }
  };
  private async refreshConsumers(generation: number) {
    try {
      if ((await this.changed()) === false) throw new Error("任务列表刷新失败");
    } catch (error: unknown) {
      if (generation === this.generation)
        this.set({ error: "后续任务已创建，刷新失败：" + String(error) });
    }
  }
}
