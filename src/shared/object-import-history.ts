import { z } from "zod";
import { fileReceiptSchema, frozenVersion } from "./object-import";

const id = z
  .string()
  .min(1)
  .max(128)
  .regex(/^[A-Za-z0-9_.:-]+$/);
const digest = z.string().regex(/^[a-f0-9]{64}$/);
const entrySchema = z.object({
  kind: z.enum(["project", "files"]),
  preparationId: id,
  requestId: id,
  cursor: z.string().min(1),
});
const pageSchema = z.object({
  targetProjectId: id,
  entries: z.array(entrySchema).max(50),
  next: z.string().min(1).nullable(),
});
export type ImportHistoryEntry = z.infer<typeof entrySchema>;
const invalid = () => new Error("导入准备历史与目标项目或记录不一致");
const versionKey = (objectId: string, versionId: string) =>
  JSON.stringify([objectId, versionId]);

export function parseImportHistory(
  input: unknown,
  projectId: string,
  after?: string,
) {
  const page = pageSchema.parse(input);
  let previous = after ?? "";
  for (const entry of page.entries) {
    const kind =
      entry.kind === "files"
        ? "file_object_import_preparation"
        : "object_import_preparation";
    if (
      entry.cursor !== kind + ":" + entry.preparationId ||
      entry.cursor <= previous
    )
      throw invalid();
    previous = entry.cursor;
  }
  if (
    page.targetProjectId !== projectId ||
    (page.next !== null && (!page.entries.length || page.next !== previous))
  )
    throw invalid();
  return page;
}

const projectReceiptSchema = z.object({
  schemaVersion: z.literal(1),
  preparationId: id,
  requestId: id,
  requestDigest: digest,
  targetProjectId: id,
  sourcePath: z.string().min(1),
  sourceProjectId: id,
  sourceObjectId: id,
  acceptedVersionId: id,
  sourceDigest: digest,
  baseline: z.discriminatedUnion("kind", [
    z.object({ kind: z.literal("latestAccepted") }),
    z.object({ kind: z.literal("pinnedVersion"), versionId: id }),
  ]),
  versions: z
    .array(
      frozenVersion.extend({
        components: z.array(
          z.object({ id, name: z.string(), kind: z.string() }),
        ),
        references: z.array(
          z.object({ projectId: id, objectId: id, versionId: id }),
        ),
      }),
    )
    .min(1),
  identityMap: z.object({
    objects: z.record(z.string(), id),
    components: z.record(z.string(), id),
    versions: z.record(z.string(), id),
  }),
  readyToCommit: z.literal(false),
});

export function parseImportHistoryReceipt(
  input: unknown,
  projectId: string,
  entry: ImportHistoryEntry,
) {
  const receipt =
    entry.kind === "files"
      ? { kind: "files" as const, ...fileReceiptSchema.parse(input) }
      : { kind: "project" as const, ...projectReceiptSchema.parse(input) };
  if (
    receipt.targetProjectId !== projectId ||
    receipt.preparationId !== entry.preparationId ||
    receipt.requestId !== entry.requestId
  )
    throw invalid();
  if (receipt.kind === "project") {
    const keys = new Set(
      receipt.versions.map((v) => versionKey(v.objectId, v.versionId)),
    );
    if (
      keys.size !== receipt.versions.length ||
      !receipt.versions.some(
        (v) =>
          v.objectId === receipt.sourceObjectId &&
          v.versionId === receipt.acceptedVersionId,
      )
    )
      throw invalid();
    if (
      receipt.baseline.kind === "pinnedVersion" &&
      receipt.baseline.versionId !== receipt.acceptedVersionId
    )
      throw invalid();
    for (const version of receipt.versions) {
      if (
        version.projectId !== receipt.sourceProjectId ||
        !receipt.identityMap.objects[version.objectId] ||
        !receipt.identityMap.versions[version.versionId] ||
        version.components.some((c) => !receipt.identityMap.components[c.id]) ||
        version.references.some(
          (r) =>
            r.projectId !== receipt.sourceProjectId ||
            !keys.has(versionKey(r.objectId, r.versionId)),
        )
      )
        throw invalid();
    }
  } else {
    const paths = new Set(receipt.source.files.map((f) => f.path));
    const assigned = new Set<string>();
    if (
      paths.size !== receipt.source.files.length ||
      receipt.source.files.some((f) => !receipt.identityMap.files[f.path])
    )
      throw invalid();
    for (const group of receipt.groups) {
      if (!receipt.identityMap.groups[group.id] || !group.paths.length)
        throw invalid();
      for (const path of group.paths) {
        if (!paths.has(path) || assigned.has(path)) throw invalid();
        assigned.add(path);
      }
    }
  }
  return receipt;
}
export type ImportHistoryReceipt = ReturnType<typeof parseImportHistoryReceipt>;
