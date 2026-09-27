import { z } from "zod";
import {
  publicationRequestSchema,
  type PublicationReview,
  type PublicationRequest,
} from "../../shared/object-publication";
import type { FollowupDraftStorage } from "./object-publication-followup-draft";

const scope = (review: PublicationReview) => ({
  projectId: review.projectId,
  taskId: review.target.taskId,
  runId: review.target.runId,
  objectId: review.target.objectId,
});
const schema = z
  .strictObject({
    schemaVersion: z.literal(1),
    scope: z.strictObject({
      projectId: z.string(),
      taskId: z.string(),
      runId: z.string(),
      objectId: z.string(),
    }),
    request: publicationRequestSchema.nullable(),
    abort: z.boolean(),
    revision: z.string().uuid().optional(),
  })
  .refine((saved) => !saved.abort || saved.request !== null);
const key = (review: PublicationReview) =>
  `beaver.publication-intent.v1:${JSON.stringify(scope(review))}`;

export function loadPublicationIntent(
  review: PublicationReview,
  storage: FollowupDraftStorage,
) {
  const raw = storage.getItem(key(review));
  return parse(review, raw);
}

function parse(review: PublicationReview, raw: string | null) {
  if (raw === null) return null;
  const saved = schema.parse(JSON.parse(raw));
  const expected = JSON.stringify(scope(review));
  if (
    JSON.stringify(saved.scope) !== expected ||
    (saved.request && JSON.stringify(scope(saved.request)) !== expected)
  )
    throw new Error("发布恢复记录与当前任务不一致");
  return saved;
}

export class PublicationIntentConflict extends Error {
  constructor() {
    super("另一窗口已更新原发布请求，请重新读取并刷新发布状态");
  }
}

export class PublicationIntentStore {
  private observed: string | null | undefined;
  constructor(
    private review: PublicationReview,
    private storage: FollowupDraftStorage,
  ) {}
  read() {
    const raw = this.storage.getItem(key(this.review));
    const saved = parse(this.review, raw);
    this.observed = raw;
    return saved;
  }
  async save(request: PublicationRequest | null, abort: boolean) {
    const expected = this.observed;
    const write = () => {
      const name = key(this.review);
      if (expected === undefined || this.storage.getItem(name) !== expected)
        throw new PublicationIntentConflict();
      const saved = schema.parse({
        schemaVersion: 1,
        scope: scope(this.review),
        request,
        abort,
        revision: crypto.randomUUID(),
      });
      this.storage.setItem(name, JSON.stringify(saved));
      this.observed = this.storage.getItem(name);
    };
    if (typeof window === "undefined") return write();
    if (!window.navigator.locks)
      throw new Error("当前窗口不支持跨窗口发布锁，请使用安全连接或桌面应用");
    // Keep the comparison and write in one origin-wide exclusive critical section.
    await window.navigator.locks.request(key(this.review), write);
  }
}
