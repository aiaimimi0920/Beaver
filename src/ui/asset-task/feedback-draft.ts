import { z } from "zod";
import type { AssetReference, AssetSubmission } from "../../shared/asset-task";

const id = z
  .string()
  .min(1)
  .max(100)
  .regex(/^[a-zA-Z0-9_-]+$/);
const coordinate = z.number().min(0).max(1);
const point = z.tuple([coordinate, coordinate]);
const vector = z.tuple([z.number(), z.number(), z.number()]);
const annotation = z
  .object({
    kind: z.enum(["box", "arrow", "brush"]),
    points: z.array(point).min(2).max(4096),
  })
  .refine((mark) => mark.kind === "brush" || mark.points.length === 2);
const submission = z
  .object({
    id,
    feedbackId: id,
    referenceId: id,
    text: z.string().trim().min(1).max(12000),
    timing: z.enum(["now", "afterRound"]),
    annotations: z.array(annotation).max(64),
  })
  .refine(
    (value) =>
      value.annotations.reduce((n, mark) => n + mark.points.length, 0) <= 4096,
  );
const reference = z.object({
  id,
  taskId: id,
  projectId: id,
  sha256: z.string().regex(/^[a-f0-9]{64}$/),
  used: z.boolean(),
  frame: z.object({
    id,
    sessionId: id,
    generation: id,
    sceneRevision: z.number().int().nonnegative(),
    viewRevision: z.number().int().nonnegative(),
    capturedAt: z.number().int().nonnegative(),
    width: z.number().int().min(1).max(1280),
    height: z.number().int().min(1).max(960),
    viewMatrix: z.array(z.number()).length(16),
    projectionMatrix: z.array(z.number()).length(16),
  }),
  pick: z
    .object({
      frameId: id,
      objectId: z.string(),
      objectName: z.string(),
      instanceId: z.string(),
      local: vector,
      world: vector,
      normal: vector,
      face: z.number().int(),
      vertices: z.number().int().nonnegative(),
      polygons: z.number().int().nonnegative(),
      point,
    })
    .nullable(),
});
const schema = z.object({ version: z.literal(1), submission, reference });
export interface FeedbackDraft {
  version: 1;
  submission: AssetSubmission;
  reference: AssetReference;
}
export type DraftStorage = Pick<Storage, "getItem" | "setItem" | "removeItem">;
const key = (taskId: string) => `beaver.asset-feedback.v1.${taskId}`;

export function readDraft(
  storage: DraftStorage,
  taskId: string,
): FeedbackDraft | null {
  const raw = storage.getItem(key(taskId));
  if (!raw) return null;
  return parseDraft(raw, taskId);
}

function parseDraft(raw: string, taskId: string): FeedbackDraft {
  if (raw.length > 300000)
    throw new Error("待提交反馈记录过大，请核对任务反馈记录。");
  const draft = schema.parse(JSON.parse(raw) as unknown);
  if (
    draft.submission.id !== taskId ||
    draft.reference.taskId !== taskId ||
    draft.submission.referenceId !== draft.reference.id ||
    (draft.reference.pick &&
      draft.reference.pick.frameId !== draft.reference.frame.id)
  ) {
    throw new Error("待提交反馈的任务或参考画面不匹配。");
  }
  return draft;
}

export function writeDraft(
  storage: DraftStorage,
  draft: FeedbackDraft,
): FeedbackDraft {
  const fixed = parseDraft(JSON.stringify(draft), draft.submission.id);
  storage.setItem(key(fixed.submission.id), JSON.stringify(fixed));
  return fixed;
}

export function clearDraft(storage: DraftStorage, taskId: string): void {
  storage.removeItem(key(taskId));
}

// Explicit recovery preserves the damaged bytes before unlocking the form.
export function archiveDraft(storage: DraftStorage, taskId: string): void {
  const raw = storage.getItem(key(taskId));
  if (raw !== null) storage.setItem(`${key(taskId)}.recovery`, raw);
  clearDraft(storage, taskId);
}
