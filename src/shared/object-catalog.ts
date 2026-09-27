import { z } from "zod";

export const objectIdSchema = z
  .string()
  .min(1)
  .max(128)
  .regex(/^[A-Za-z0-9_.:-]+$/);
const text = (limit: number) =>
  z
    .string()
    .min(1)
    .max(limit)
    .refine((value) => value.trim().length > 0);
const metadataText = (limit: number) =>
  z
    .string()
    .min(1)
    .max(limit)
    .refine((value) => value.trim() === value && value.length > 0);
const categorySchema = metadataText(128).default("其他");
const tagsSchema = z
  .array(metadataText(128))
  .max(64)
  .superRefine((tags, context) => {
    const seen = new Set<string>();
    tags.forEach((tag, index) => {
      const key = tag.toLocaleLowerCase();
      if (seen.has(key)) {
        context.addIssue({
          code: z.ZodIssueCode.custom,
          path: [index],
          message: "标签不能重复",
        });
      }
      seen.add(key);
    });
  })
  .default([]);
function validAssetPath(value: string) {
  if (value.length === 0 || value.length > 2_000 || value.trim() !== value)
    return false;
  if ([...value].some((char) => /[\u0000-\u001f\\:<>"|?*]/u.test(char)))
    return false;
  return value.split("/").every((part) => {
    if (
      !part ||
      part === "." ||
      part === ".." ||
      part.trim() !== part ||
      part.endsWith(".")
    )
      return false;
    const stem = (part.split(".", 1)[0] ?? "").toUpperCase();
    return (
      !new Set(["CON", "PRN", "AUX", "NUL"]).has(stem) &&
      !["COM", "LPT"].some(
        (prefix) =>
          stem.startsWith(prefix) && /^[1-9]$/u.test(stem.slice(prefix.length)),
      ) &&
      ![".beaver", ".beaver-context", ".git", ".godot"].includes(
        stem.toLowerCase(),
      )
    );
  });
}
const thumbnailPathSchema = z
  .string()
  .refine(validAssetPath, "缩略图路径必须是安全的项目相对路径")
  .nullable()
  .default(null);
const parentObjectIdSchema = objectIdSchema.nullable().default(null);
export const objectComponentSchema = z.strictObject({
  id: objectIdSchema,
  kind: text(128),
  name: text(800),
});
export const objectFileSchema = z.strictObject({
  path: text(2_000),
  role: text(128),
});
export const objectReferenceSchema = z.strictObject({
  projectId: objectIdSchema,
  objectId: objectIdSchema,
  versionId: objectIdSchema.nullable().default(null),
});
export const pinnedObjectReferenceSchema = objectReferenceSchema.extend({
  versionId: objectIdSchema,
});
export const objectCatalogRecordSchema = z.strictObject({
  id: objectIdSchema,
  projectId: objectIdSchema,
  name: text(800),
  category: categorySchema,
  tags: tagsSchema,
  thumbnailPath: thumbnailPathSchema,
  parentObjectId: parentObjectIdSchema,
  revision: z
    .number()
    .int()
    .nonnegative()
    .max(Number.MAX_SAFE_INTEGER)
    .default(0),
  components: z.array(objectComponentSchema).max(256),
  files: z.array(objectFileSchema).max(1_024),
  references: z.array(objectReferenceSchema).max(256),
  versions: z.array(
    z.strictObject({
      versionId: objectIdSchema,
      manifest: z.unknown().refine((value) => value !== undefined),
    }),
  ),
});
const manifestSchema = z.strictObject({
  schemaVersion: z.literal(1),
  projectId: objectIdSchema,
  objectId: objectIdSchema,
  versionId: objectIdSchema,
  status: z.enum(["captured", "importedPendingValidation", "accepted"]),
  name: text(800),
  category: categorySchema,
  tags: tagsSchema,
  thumbnailPath: thumbnailPathSchema,
  parentObjectId: parentObjectIdSchema,
  components: z.array(objectComponentSchema),
  files: z.array(
    objectFileSchema.extend({
      bytes: z.number().int().nonnegative().max(Number.MAX_SAFE_INTEGER),
      sha256: z.string().regex(/^[a-f0-9]{64}$/),
    }),
  ),
  references: z.array(pinnedObjectReferenceSchema),
});

export type ObjectCatalogRecord = z.infer<typeof objectCatalogRecordSchema>;
export type ObjectCatalogVersion = ObjectCatalogRecord["versions"][number];
export type ObjectRegistrationDraft = Pick<
  ObjectCatalogRecord,
  | "name"
  | "category"
  | "tags"
  | "thumbnailPath"
  | "parentObjectId"
  | "components"
  | "files"
  | "references"
>;

function unique(values: string[]) {
  return new Set(values).size === values.length;
}

export function parseObjectCatalog(value: unknown, projectId: string) {
  const objects = z.array(objectCatalogRecordSchema).parse(value);
  if (
    !unique(objects.map((object) => object.id)) ||
    objects.some(
      (object) =>
        object.projectId !== projectId ||
        !unique(object.versions.map((version) => version.versionId)),
    )
  ) {
    throw new Error("对象目录响应与当前项目或版本身份不一致");
  }
  return objects;
}

export function readObjectVersion(
  object: Pick<
    ObjectCatalogRecord,
    | "id"
    | "projectId"
    | "category"
    | "tags"
    | "thumbnailPath"
    | "parentObjectId"
  >,
  version: ObjectCatalogVersion,
) {
  const manifest = readFrozenObjectVersion(object, version);
  if (
    !manifest ||
    manifest.category !== object.category ||
    !sameTags(manifest.tags, object.tags) ||
    manifest.thumbnailPath !== object.thumbnailPath ||
    manifest.parentObjectId !== object.parentObjectId
  )
    return null;
  return manifest;
}

export function readFrozenObjectVersion(
  object: Pick<ObjectCatalogRecord, "id" | "projectId">,
  version: ObjectCatalogVersion,
) {
  const parsed = manifestSchema.safeParse(version.manifest);
  if (!parsed.success) return null;
  const manifest = parsed.data;
  if (
    manifest.projectId !== object.projectId ||
    manifest.objectId !== object.id ||
    manifest.versionId !== version.versionId ||
    !unique(manifest.components.map((component) => component.id)) ||
    !unique(manifest.files.map((file) => file.path.toLowerCase())) ||
    !unique(
      manifest.references.map((ref) =>
        JSON.stringify([ref.objectId, ref.versionId]),
      ),
    ) ||
    manifest.references.some(
      (ref) => ref.projectId !== object.projectId || ref.objectId === object.id,
    )
  ) {
    return null;
  }
  return manifest;
}

function sameTags(left: string[], right: string[]) {
  return (
    left.length === right.length &&
    left.every((tag, index) => tag === right[index])
  );
}

export function acceptedObjectReferences(
  objects: ObjectCatalogRecord[],
  projectId: string,
  ownerId?: string,
) {
  return objects.flatMap((object) =>
    object.projectId !== projectId || object.id === ownerId
      ? []
      : object.versions.flatMap((version) =>
          readObjectVersion(object, version)?.status === "accepted"
            ? [
                {
                  projectId,
                  objectId: object.id,
                  versionId: version.versionId,
                  name: object.name,
                },
              ]
            : [],
        ),
  );
}
