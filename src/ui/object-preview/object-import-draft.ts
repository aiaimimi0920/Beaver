import { z } from "zod";
import type { ObjectImportState } from "./object-import-session";

const text = z.string().max(32768);
const fields = z
  .object({
    sourceMode: z.enum(["project", "external", "files"]),
    path: text,
    sourceProjectId: text,
    selectedPaths: z.array(text).max(4096),
    fileGroups: z
      .array(
        z
          .object({ id: text, name: text, paths: z.array(text).max(65536) })
          .strict(),
      )
      .max(1024),
    objectId: text,
    versionId: text,
    groupName: text,
  })
  .strict();
const schema = z
  .object({ schemaVersion: z.literal(1), targetProjectId: text, fields })
  .strict();
export type ImportDraft = z.infer<typeof fields>;
export type DraftStorage = Pick<Storage, "getItem" | "setItem">;
const key = (target: string) => `beaver.object-import-draft.v1.${target}`;

export function loadImportDraft(
  target: string | undefined,
  storage: DraftStorage,
) {
  if (!target) return { draft: undefined, error: "" };
  try {
    const raw = storage.getItem(key(target));
    if (!raw) return { draft: undefined, error: "" };
    const value = schema.parse(JSON.parse(raw));
    if (value.targetProjectId !== target) throw new Error("target mismatch");
    const ids = value.fields.fileGroups.map((group) => group.id);
    if (new Set(ids).size !== ids.length) throw new Error("duplicate group");
    return { draft: value.fields, error: "" };
  } catch {
    return {
      draft: undefined,
      error: "无法恢复本地导入草稿；原记录未删除，可重新填写。",
    };
  }
}

export function saveImportDraft(
  target: string | undefined,
  storage: DraftStorage,
  state: Readonly<ObjectImportState>,
  groupName: string,
) {
  if (!target) return "";
  try {
    const {
      sourceMode,
      path,
      sourceProjectId,
      selectedPaths,
      fileGroups,
      objectId,
      versionId,
    } = state;
    const value = schema.parse({
      schemaVersion: 1,
      targetProjectId: target,
      fields: {
        sourceMode,
        path,
        sourceProjectId,
        selectedPaths,
        fileGroups,
        objectId,
        versionId,
        groupName,
      },
    });
    storage.setItem(key(target), JSON.stringify(value));
    return "";
  } catch {
    return "无法保存本地导入草稿；关闭窗口可能丢失未保存输入。";
  }
}

// Access may itself throw when WebView storage is disabled.
export const browserDraftStorage: DraftStorage = {
  getItem: (key) =>
    typeof window === "undefined" ? null : window.localStorage.getItem(key),
  setItem: (key, value) => {
    if (typeof window !== "undefined") window.localStorage.setItem(key, value);
  },
};
