import {
  deferredFeedbackSchema,
  type DeferredFeedback,
} from "../../shared/object-publication-deferred";
import type { PublicationReview } from "../../shared/object-publication";

type StoragePort = Pick<Storage, "getItem" | "setItem" | "removeItem">;
type Input = Pick<
  DeferredFeedback,
  "feedback" | "later" | "image" | "previewFrame" | "relocation"
>;
interface State {
  busy: boolean;
  error: string;
  blocked: boolean;
  pending: DeferredFeedback | null;
  saved: DeferredFeedback | null;
}

export class ObjectPublicationDeferred {
  private state: State = {
    busy: false,
    error: "",
    blocked: false,
    pending: null,
    saved: null,
  };
  private listeners = new Set<() => void>();
  private generation = 0;
  private storage: StoragePort | undefined;
  private key: string;
  constructor(
    readonly review: PublicationReview,
    private api: (method: string, input: unknown) => Promise<unknown>,
    private changed: () => Promise<unknown>,
    private requestId: () => string = () => crypto.randomUUID(),
    storage?: StoragePort,
  ) {
    this.key = "beaver.deferred-feedback.v1:" + JSON.stringify(review);
    try {
      this.storage =
        storage ??
        (typeof window === "undefined" ? undefined : window.localStorage);
      const raw = this.storage?.getItem(this.key);
      if (raw) {
        const pending = deferredFeedbackSchema.parse(JSON.parse(raw));
        if (JSON.stringify(pending.review) !== JSON.stringify(review))
          throw new Error("反馈来源不一致");
        this.state.pending = pending;
      }
    } catch (error: unknown) {
      this.state = {
        ...this.state,
        blocked: true,
        error: "无法恢复原反馈请求，已阻止新提交：" + String(error),
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
    this.set({ busy: false });
  };
  save = async (input?: Input) => {
    if (this.state.busy || this.state.blocked) return;
    const generation = ++this.generation;
    try {
      const request =
        this.state.pending ??
        deferredFeedbackSchema.parse({
          ...input,
          projectId: this.review.projectId,
          review: this.review,
          requestId: this.requestId(),
        });
      this.set({ pending: request, saved: null, error: "" });
      // Persist the immutable request before any side effect, including retries.
      this.storage?.setItem(this.key, JSON.stringify(request));
      this.set({ busy: true });
      const response = deferredFeedbackSchema.parse(
        await this.api(
          "objectTask.deferCandidateFeedback",
          structuredClone(request),
        ),
      );
      if (generation !== this.generation) return;
      if (JSON.stringify(response) !== JSON.stringify(request))
        throw new Error("反馈回执与原请求不一致");
      this.storage?.removeItem(this.key);
      this.set({ pending: null, saved: response, busy: false });
      await this.changed();
    } catch (error: unknown) {
      if (generation === this.generation)
        this.set({ busy: false, error: String(error) });
    }
  };
}
