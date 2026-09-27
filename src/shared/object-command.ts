import { z } from "zod";
import {
  objectCatalogRecordSchema,
  objectComponentSchema,
  objectFileSchema,
  objectIdSchema,
  pinnedObjectReferenceSchema,
  readObjectVersion,
  type ObjectCatalogRecord,
} from "./object-catalog";

const identity = { projectId: objectIdSchema, requestId: objectIdSchema };
const baseline = {
  ...identity,
  objectId: objectIdSchema,
  expectedRevision: z
    .number()
    .int()
    .nonnegative()
    .max(Number.MAX_SAFE_INTEGER - 1),
};
const metadata = {
  name: objectCatalogRecordSchema.shape.name,
  category: objectCatalogRecordSchema.shape.category,
  tags: objectCatalogRecordSchema.shape.tags,
  thumbnailPath: objectCatalogRecordSchema.shape.thumbnailPath,
  parentObjectId: objectCatalogRecordSchema.shape.parentObjectId,
  components: z.array(objectComponentSchema).max(256),
  files: z.array(objectFileSchema).max(1_024),
  references: z.array(pinnedObjectReferenceSchema).max(256),
};
const commandSchema = z.discriminatedUnion("method", [
  z.strictObject({
    method: z.literal("object.register"),
    input: z.strictObject({ ...identity, ...metadata }),
  }),
  z.strictObject({
    method: z.literal("object.updateRegistration"),
    input: z.strictObject({ ...baseline, ...metadata }),
  }),
  z.strictObject({
    method: z.literal("object.captureVersion"),
    input: z.strictObject(baseline),
  }),
  z.strictObject({
    method: z.literal("object.acceptVersion"),
    input: z.strictObject({ ...baseline, versionId: objectIdSchema }),
  }),
]);
const resultSchema = z.strictObject({
  projectId: objectIdSchema,
  requestId: objectIdSchema,
  object: objectCatalogRecordSchema,
  versionId: objectIdSchema.nullable(),
});
export type ObjectCommand = z.infer<typeof commandSchema>;
export type ObjectCommandResult = z.infer<typeof resultSchema>;

export function parseObjectCommand(input: unknown): ObjectCommand {
  return commandSchema.parse(input);
}

function same(left: unknown, right: unknown): boolean {
  if (left === right) return true;
  if (!left || !right || typeof left !== "object" || typeof right !== "object")
    return false;
  if (Array.isArray(left) || Array.isArray(right))
    return (
      Array.isArray(left) &&
      Array.isArray(right) &&
      left.length === right.length &&
      left.every((value, index) => same(value, right[index]))
    );
  const a = left as Record<string, unknown>;
  const b = right as Record<string, unknown>;
  return (
    Object.keys(a).length === Object.keys(b).length &&
    Object.keys(a).every((key) => Object.hasOwn(b, key) && same(a[key], b[key]))
  );
}

function registration(value: Pick<ObjectCatalogRecord, keyof typeof metadata>) {
  const {
    name,
    category,
    tags,
    thumbnailPath,
    parentObjectId,
    components,
    files,
    references,
  } = value;
  return {
    name,
    category,
    tags,
    thumbnailPath,
    parentObjectId,
    components,
    files,
    references,
  };
}

export function parseObjectCommandResult(
  value: unknown,
  command: ObjectCommand,
  original?: ObjectCatalogRecord,
): ObjectCommandResult {
  const result = resultSchema.parse(value);
  const { object } = result;
  const invalid = () => new Error("对象操作回执与原请求或版本清单不一致");
  if (
    result.projectId !== command.input.projectId ||
    result.requestId !== command.input.requestId ||
    object.projectId !== result.projectId
  )
    throw invalid();
  if (command.method === "object.register") {
    if (
      result.versionId !== null ||
      object.revision !== 0 ||
      object.versions.length ||
      !same(registration(object), registration(command.input))
    )
      throw invalid();
    return result;
  }
  if (
    !original ||
    original.id !== command.input.objectId ||
    original.projectId !== result.projectId ||
    original.revision !== command.input.expectedRevision ||
    object.id !== original.id ||
    object.revision !== original.revision + 1
  )
    throw invalid();
  if (command.method === "object.updateRegistration") {
    if (
      result.versionId !== null ||
      !same(object.versions, original.versions) ||
      !same(registration(object), registration(command.input))
    )
      throw invalid();
    return result;
  }
  if (command.method === "object.acceptVersion") {
    const originalVersions = original.versions.filter(
      (item) => item.versionId === command.input.versionId,
    );
    const acceptedVersions = object.versions.filter(
      (item) => item.versionId === command.input.versionId,
    );
    const originalVersion = originalVersions[0];
    const acceptedVersion = acceptedVersions[0];
    const originalManifest =
      originalVersion && readObjectVersion(original, originalVersion);
    const acceptedManifest =
      acceptedVersion && readObjectVersion(object, acceptedVersion);
    const acceptedComparable = acceptedManifest && {
      ...acceptedVersion,
      manifest: { ...acceptedManifest, status: "captured" as const },
    };
    const originalComparable = originalManifest && {
      ...originalVersion,
      manifest: originalManifest,
    };
    if (
      result.versionId !== command.input.versionId ||
      originalVersions.length !== 1 ||
      acceptedVersions.length !== 1 ||
      !originalManifest ||
      originalManifest.status !== "captured" ||
      !acceptedManifest ||
      acceptedManifest.status !== "accepted" ||
      object.versions.length !== original.versions.length ||
      !same(
        object.versions.filter(
          (item) => item.versionId !== command.input.versionId,
        ),
        original.versions.filter(
          (item) => item.versionId !== command.input.versionId,
        ),
      ) ||
      !same(acceptedComparable, originalComparable) ||
      !same(registration(object), registration(original))
    )
      throw invalid();
    return result;
  }
  const version = object.versions.at(-1);
  const manifest = version && readObjectVersion(object, version);
  if (
    !version ||
    !manifest ||
    manifest.status !== "captured" ||
    version.versionId !== result.versionId ||
    original.versions.some((item) => item.versionId === version.versionId) ||
    !same(object.versions.slice(0, -1), original.versions) ||
    !same(registration(object), registration(original)) ||
    !same(
      registration({
        ...manifest,
        files: manifest.files.map(({ path, role }) => ({ path, role })),
      }),
      registration(original),
    )
  )
    throw invalid();
  return result;
}
