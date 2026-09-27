import { z } from "zod";

const id = z
  .string()
  .min(1)
  .max(128)
  .regex(/^[A-Za-z0-9_.:-]+$/);
const digest = z.string().regex(/^[a-f0-9]{64}$/);
const pinnedBaseline = z.strictObject({
  kind: z.literal("pinnedVersion"),
  versionId: id,
});
export const frozenVersion = z.object({
  schemaVersion: z.literal(1),
  projectId: id,
  objectId: id,
  versionId: id,
  status: z.literal("accepted"),
  name: z.string().min(1),
  files: z.array(
    z.object({
      path: z.string().min(1),
      role: z.string().min(1),
      bytes: z.number().int().nonnegative(),
      sha256: digest,
    }),
  ),
});
const inspectionSchema = z.object({
  project: z.object({ id }),
  objects: z.array(
    z.object({
      id,
      projectId: id,
      name: z.string().min(1),
      versions: z.array(z.object({ versionId: id, manifest: z.unknown() })),
    }),
  ),
  importVersions: z.array(
    z.object({
      objectId: id,
      versionId: id,
      sourceDigest: digest.nullable(),
      blocker: z.string().min(1).nullable(),
    }),
  ),
});
const fileSourceSnapshotSchema = z.strictObject({
  source: z.strictObject({
    kind: z.literal("files"),
    paths: z.array(z.string().min(1)).min(1).max(100),
  }),
  files: z
    .array(
      z.strictObject({
        sourcePath: z.string().min(1),
        relativePath: z.string().min(1),
        path: z.string().min(1),
        kind: z.string().min(1),
        bytes: z.number().int().nonnegative(),
        sha256: digest,
      }),
    )
    .max(100_000),
  digest,
});

export type FileSourceSnapshot = z.infer<typeof fileSourceSnapshotSchema>;
export type FileSnapshot = FileSourceSnapshot["files"][number];

export function parseFileSourceSnapshot(input: unknown): FileSourceSnapshot {
  return fileSourceSnapshotSchema.parse(input);
}

export interface ImportSourceVersion {
  versionId: string;
  sourceDigest: string | null;
  blocker: string | null;
  manifest: z.infer<typeof frozenVersion> | null;
}

export interface ImportSourceObject {
  id: string;
  name: string;
  versions: ImportSourceVersion[];
}

export function parseImportInspection(
  input: unknown,
  expectedProjectId?: string,
): { projectId: string; objects: ImportSourceObject[] } {
  const snapshot = inspectionSchema.parse(input);
  const projectId = snapshot.project.id;
  const invalid = () => new Error("源对象目录与接受版本响应不一致");
  if (expectedProjectId !== undefined && projectId !== expectedProjectId)
    throw invalid();
  const key = (objectId: string, versionId: string) =>
    JSON.stringify([objectId, versionId]);
  const options = new Map(
    snapshot.importVersions.map((version) => [
      key(version.objectId, version.versionId),
      version,
    ]),
  );
  if (options.size !== snapshot.importVersions.length) throw invalid();
  const seen = new Set<string>();
  const objects = snapshot.objects.map((object) => {
    if (object.projectId !== projectId || seen.has(object.id)) throw invalid();
    seen.add(object.id);
    const versions = object.versions.map((version) => {
      const option = options.get(key(object.id, version.versionId));
      if (!option || Boolean(option.sourceDigest) === Boolean(option.blocker))
        throw invalid();
      options.delete(key(object.id, version.versionId));
      const manifest = option.sourceDigest
        ? frozenVersion.parse(version.manifest)
        : null;
      if (
        manifest &&
        (manifest.projectId !== projectId ||
          manifest.objectId !== object.id ||
          manifest.versionId !== version.versionId)
      )
        throw invalid();
      return { ...option, manifest };
    });
    return { id: object.id, name: object.name, versions };
  });
  if (options.size) throw invalid();
  return { projectId, objects };
}

export interface ImportPreparationRequest {
  requestId: string;
  targetProjectId: string;
  source: { path: string; projectId: string };
  objectId: string;
  baseline: z.infer<typeof pinnedBaseline>;
  sourceDigest: string;
}

export interface FileImportGroup {
  id: string;
  name: string;
  paths: string[];
}

export interface FileImportPreparationRequest {
  requestId: string;
  targetProjectId: string;
  snapshot: FileSourceSnapshot;
  groups: FileImportGroup[];
}

const receiptSchema = z.object({
  schemaVersion: z.literal(1),
  preparationId: id,
  requestId: id,
  targetProjectId: id,
  sourceProjectId: id,
  sourceObjectId: id,
  baseline: pinnedBaseline,
  acceptedVersionId: id,
  sourceDigest: digest,
  readyToCommit: z.literal(false),
});
export type ImportPreparationReceipt = z.infer<typeof receiptSchema>;

export function parseImportReceipt(
  input: unknown,
  request: ImportPreparationRequest,
): ImportPreparationReceipt {
  const receipt = receiptSchema.parse(input);
  if (
    receipt.requestId !== request.requestId ||
    receipt.targetProjectId !== request.targetProjectId ||
    receipt.sourceProjectId !== request.source.projectId ||
    receipt.sourceObjectId !== request.objectId ||
    receipt.baseline.versionId !== request.baseline.versionId ||
    receipt.acceptedVersionId !== request.baseline.versionId ||
    receipt.sourceDigest !== request.sourceDigest
  ) {
    throw new Error("导入准备回执与当前请求不一致");
  }
  return receipt;
}

export const fileReceiptSchema = z.object({
  schemaVersion: z.literal(1),
  preparationId: id,
  requestId: id,
  requestDigest: digest,
  targetProjectId: id,
  source: fileSourceSnapshotSchema,
  groups: z.array(
    z.object({
      id,
      name: z.string().min(1),
      paths: z.array(z.string().min(1)),
    }),
  ),
  identityMap: z.object({
    files: z.record(z.string(), id),
    groups: z.record(z.string(), id),
  }),
  readyToCommit: z.literal(false),
});
export type FileImportPreparationReceipt = z.infer<typeof fileReceiptSchema>;

export function parseFileImportReceipt(
  input: unknown,
  request: FileImportPreparationRequest,
): FileImportPreparationReceipt {
  const receipt = fileReceiptSchema.parse(input);
  if (
    receipt.requestId !== request.requestId ||
    receipt.targetProjectId !== request.targetProjectId ||
    receipt.source.digest !== request.snapshot.digest ||
    JSON.stringify(receipt.groups) !== JSON.stringify(request.groups)
  )
    throw new Error("文件导入准备回执与当前请求不一致");
  return receipt;
}
