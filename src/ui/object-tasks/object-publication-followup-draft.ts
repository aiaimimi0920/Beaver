import { z } from "zod";
import type { PublicationOperation } from "../../shared/object-publication";
import { publicationFollowupRequestSchema } from "../../shared/object-publication-followup";

const fields = z.strictObject({
  title: z.string(),
  feedback: z.string(),
  acceptance: z.string(),
  previewFrame: publicationFollowupRequestSchema.shape.previewFrame,
});
const schema = z.strictObject({
  schemaVersion: z.literal(1),
  projectId: z.string(),
  publicationRequestId: z.string(),
  versionId: z.string(),
  draft: fields,
  request: publicationFollowupRequestSchema.nullable(),
});
export type FollowupDraft = z.infer<typeof fields>;
export type FollowupDraftStorage = Pick<Storage, "getItem" | "setItem">;
type Saved = z.infer<typeof schema>;
const identity = (op: PublicationOperation) => ({
  projectId: op.request.projectId,
  publicationRequestId: op.request.requestId,
  versionId: op.versionId,
});
const key = (op: PublicationOperation) =>
  `beaver.publication-followup.v1.${JSON.stringify(identity(op))}`;

export const followupBrowserStorage: FollowupDraftStorage = {
  getItem: (key) =>
    typeof window === "undefined" ? null : window.localStorage.getItem(key),
  setItem: (key, value) => {
    if (typeof window !== "undefined") window.localStorage.setItem(key, value);
  },
};

export function loadFollowupDraft(
  op: PublicationOperation,
  storage: FollowupDraftStorage,
) {
  const raw = storage.getItem(key(op));
  if (!raw) return null;
  const saved = schema.parse(JSON.parse(raw));
  const expected = identity(op);
  for (const field of [
    "projectId",
    "publicationRequestId",
    "versionId",
  ] as const) {
    if (
      saved[field] !== expected[field] ||
      (saved.request && saved.request[field] !== expected[field])
    )
      throw new Error("后续任务草稿来源不一致");
  }
  if (
    saved.request &&
    (["title", "feedback", "acceptance", "previewFrame"] as const).some(
      (field) =>
        JSON.stringify(saved.draft[field as keyof FollowupDraft]) !==
        JSON.stringify(saved.request![field as keyof FollowupDraft]),
    )
  )
    throw new Error("后续任务草稿与原请求不一致");
  return saved;
}

export function saveFollowupDraft(
  op: PublicationOperation,
  storage: FollowupDraftStorage,
  draft: FollowupDraft,
  request: Saved["request"],
) {
  const saved = schema.parse({
    schemaVersion: 1,
    ...identity(op),
    draft,
    request,
  });
  storage.setItem(key(op), JSON.stringify(saved));
}
