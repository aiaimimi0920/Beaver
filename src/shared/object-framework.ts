import { z } from "zod";

export const objectFrameworkVersion = 1;
const identityId = z
  .string()
  .min(1)
  .max(128)
  .regex(/^[A-Za-z0-9_.:-]+$/);
export const objectBaselineSchema = z.discriminatedUnion("basePolicy", [
  z.strictObject({ basePolicy: z.literal("latestAccepted") }),
  z.strictObject({
    basePolicy: z.literal("pinnedVersion"),
    selectedVersionId: identityId,
  }),
  z.strictObject({ basePolicy: z.literal("empty") }),
]);
export const objectTaskIdentitySchema = z.discriminatedUnion("layer", [
  z.strictObject({ schemaVersion: z.literal(1), layer: z.literal("coarse") }),
  z.strictObject({
    schemaVersion: z.literal(1),
    layer: z.literal("medium"),
    objectId: identityId,
    baseline: objectBaselineSchema,
  }),
  z.strictObject({
    schemaVersion: z.literal(1),
    layer: z.literal("fine"),
    objectId: identityId,
    mediumTaskId: identityId,
    runId: identityId,
    stageId: identityId,
  }),
]);
export type ObjectTaskIdentity = z.infer<typeof objectTaskIdentitySchema>;
export type ObjectBaseline = z.infer<typeof objectBaselineSchema>;

export const objectFrameworkStatusSchema = z.object({
  schemaVersion: z.literal(objectFrameworkVersion),
  projectId: z.string().min(1),
  storage: z.object({
    state: z.enum(["offline", "legacy", "detected", "invalid"]),
    message: z.string(),
    routed: z.boolean().default(false),
    manifestPath: z.literal(".beaver/project.json"),
    databasePath: z.literal(".beaver/project.sqlite"),
  }),
  capabilities: z.object({
    objectsRead: z.boolean(),
    manufactureRead: z.boolean(),
    execution: z.boolean(),
  }),
  blockers: z.array(z.object({ code: z.string(), message: z.string() })),
});
export type ObjectFrameworkStatus = z.infer<typeof objectFrameworkStatusSchema>;

export function parseObjectFrameworkStatus(
  input: unknown,
  projectId: string,
): ObjectFrameworkStatus {
  const status = objectFrameworkStatusSchema.parse(input);
  if (status.projectId !== projectId)
    throw new Error("项目状态响应与当前项目不一致");
  return status;
}
