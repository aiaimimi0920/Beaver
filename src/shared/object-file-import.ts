import { z } from "zod";

const schema = z.strictObject({
  projectId: z.string().min(1),
  preparationId: z.string().min(1),
  state: z.enum([
    "applying",
    "aborting",
    "importedPendingValidation",
    "aborted",
  ]),
  objectIds: z.array(z.string().min(1)).min(1).max(1024),
  writes: z.array(z.string().min(1)),
  error: z.string().nullable(),
});
export type FileImportOperation = z.infer<typeof schema>;

export function parseFileImportOperation(
  value: unknown,
  projectId: string,
  preparationId: string,
) {
  const operation = schema.parse(value);
  if (
    operation.projectId !== projectId ||
    operation.preparationId !== preparationId ||
    new Set(operation.objectIds).size !== operation.objectIds.length ||
    new Set(operation.writes).size !== operation.writes.length
  ) {
    throw new Error("导入记录身份不匹配");
  }
  return operation;
}
