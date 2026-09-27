import { z } from "zod";
import {
  publicationReviewSchema,
  previewFrameReferenceSchema,
  type PublicationReview,
} from "../../shared/object-publication";
import { imageRegionSchema } from "../../shared/object-rework-image";
import { relocationDraftSchema } from "./feedback-relocation-draft";

const selection = z.strictObject({
  path: z.string().min(1),
  sha256: z.string().regex(/^[a-f0-9]{64}$/),
  regions: z
    .array(
      z
        .strictObject({
          x: z.number(),
          y: z.number(),
          width: z.number(),
          height: z.number(),
          prompt: z.string(),
        })
        .refine(
          (region) =>
            imageRegionSchema.safeParse({ ...region, prompt: "" }).success,
        ),
    )
    .max(8),
});
const fields = z
  .strictObject({
    route: z.enum(["current", "later"]),
    feedback: z.string(),
    title: z.string(),
    acceptance: z.string(),
    image: selection.optional(),
    previewFrame: previewFrameReferenceSchema.optional(),
    relocation: relocationDraftSchema.optional(),
  })
  .refine(
    (draft) => !draft.image || !draft.previewFrame,
    "FEEDBACK_IMAGE_CONFLICT",
  );
const schema = z.strictObject({
  schemaVersion: z.literal(1),
  review: publicationReviewSchema,
  draft: fields,
});
export type CandidateFeedbackDraft = z.infer<typeof fields>;
export type FeedbackImageDraft = z.infer<typeof selection>;
type StoragePort = Pick<Storage, "getItem" | "setItem">;
const browserStorage: StoragePort = {
  getItem: (key) =>
    typeof window === "undefined" ? null : window.localStorage.getItem(key),
  setItem: (key, value) => {
    if (typeof window !== "undefined") window.localStorage.setItem(key, value);
  },
};

export class ObjectCandidateFeedbackDraft {
  private state = {
    draft: {
      route: "current",
      feedback: "",
      title: "",
      acceptance: "",
    } as CandidateFeedbackDraft,
    restored: false,
    blocked: false,
    error: "",
    restoreVersion: 0,
  };
  private listeners = new Set<() => void>();
  private key: string;
  constructor(
    readonly review: PublicationReview,
    private storage: StoragePort = browserStorage,
  ) {
    this.key = "beaver.candidate-feedback.v1:" + JSON.stringify(review);
    this.restore();
  }
  getSnapshot = () => this.state;
  subscribe = (listener: () => void) => {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  };
  private set(patch: Partial<typeof this.state>) {
    this.state = { ...this.state, ...patch };
    for (const listener of this.listeners) listener();
  }
  restore = () => {
    try {
      const raw = this.storage.getItem(this.key);
      if (raw) {
        const saved = schema.parse(JSON.parse(raw));
        if (JSON.stringify(saved.review) !== JSON.stringify(this.review))
          throw Error("反馈草稿来源不一致");
        this.set({
          draft: saved.draft,
          restored: true,
          restoreVersion: this.state.restoreVersion + 1,
        });
      }
      this.set({ blocked: false, error: "" });
    } catch (error: unknown) {
      this.set({
        blocked: true,
        error: "无法恢复反馈草稿，原记录保留：" + String(error),
      });
    }
  };
  edit = (patch: Partial<CandidateFeedbackDraft>) => {
    if (this.state.blocked) return;
    const current = this.state.draft;
    const next = { ...current, ...patch };
    if (
      next.relocation &&
      (JSON.stringify(next.previewFrame) !==
        JSON.stringify(current.previewFrame) ||
        next.relocation.sourceAttemptId !==
          current.relocation?.sourceAttemptId ||
        JSON.stringify(next.relocation.sourceFrame) !==
          JSON.stringify(current.relocation?.sourceFrame))
    ) {
      next.relocation = { ...next.relocation, regions: [], confirmed: false };
    }
    this.set({ draft: next });
    this.persist();
  };
  persist = () => {
    if (this.state.blocked) return false;
    try {
      const saved = schema.parse({
        schemaVersion: 1,
        review: this.review,
        draft: this.state.draft,
      });
      this.storage.setItem(this.key, JSON.stringify(saved));
      this.set({ error: "" });
      return true;
    } catch (error: unknown) {
      this.set({ error: "反馈草稿尚未保存，请勿关闭页面：" + String(error) });
      return false;
    }
  };
}
