import { z } from "zod";
import {
  publicationReviewSchema,
  type PublicationPreview,
} from "../../shared/object-publication";
import {
  followupBrowserStorage,
  type FollowupDraftStorage,
} from "./object-publication-followup-draft";

const fields = z.strictObject({
  note: z.string(),
  decisions: z.record(
    z.string(),
    z.strictObject({
      resolution: z.enum(["", "resolved", "waived", "deferred"]),
      note: z.string(),
    }),
  ),
});
const schema = z.strictObject({
  schemaVersion: z.literal(1),
  review: publicationReviewSchema,
  previewDigest: z.string(),
  draft: fields,
});
export type PublicationApprovalDraft = z.infer<typeof fields>;

export class ObjectPublicationApprovalDraft {
  private state = {
    draft: { note: "", decisions: {} } as PublicationApprovalDraft,
    restored: false,
    blocked: false,
    error: "",
  };
  private listeners = new Set<() => void>();
  private key: string;
  constructor(
    private preview: PublicationPreview,
    private storage: FollowupDraftStorage = followupBrowserStorage,
  ) {
    this.key =
      "beaver.publication-approval.v1:" +
      JSON.stringify([preview.review, preview.digest]);
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
        if (
          JSON.stringify(saved.review) !==
            JSON.stringify(this.preview.review) ||
          saved.previewDigest !== this.preview.digest ||
          Object.entries(saved.draft.decisions).some(([id, decision]) => {
            const feedback = this.preview.feedback.find(
              (item) => item.requestId === id,
            );
            return (
              !feedback ||
              (decision.resolution !== "" &&
                !!feedback.later !== (decision.resolution === "deferred"))
            );
          })
        )
          throw new Error("发布草稿与当前审阅或反馈不一致");
        this.set({ draft: saved.draft, restored: true });
      }
      this.set({ blocked: false, error: "" });
    } catch (error: unknown) {
      this.set({
        blocked: true,
        error: "无法恢复发布草稿，原记录保留：" + String(error),
      });
    }
  };
  edit = (patch: Partial<PublicationApprovalDraft>) => {
    if (this.state.blocked) return;
    this.set({ draft: { ...this.state.draft, ...patch } });
    this.persist();
  };
  persist = () => {
    if (this.state.blocked) return false;
    try {
      const saved = schema.parse({
        schemaVersion: 1,
        review: this.preview.review,
        previewDigest: this.preview.digest,
        draft: this.state.draft,
      });
      this.storage.setItem(this.key, JSON.stringify(saved));
      this.set({ error: "" });
      return true;
    } catch (error: unknown) {
      this.set({ error: "发布草稿尚未保存，请勿关闭页面：" + String(error) });
      return false;
    }
  };
}
