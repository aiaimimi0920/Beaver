import { z } from "zod";

export const planningDeclarationRequestSchema = z.strictObject({
  projectId: z.string().min(1),
  taskId: z.string().min(1),
  requestId: z.string().min(1),
  expectedPlanRevision: z.number().int().nonnegative(),
  expectedScopeHash: z.string().regex(/^[a-f0-9]{64}$/),
  reason: z
    .string()
    .trim()
    .min(1)
    .refine(
      (value) => new TextEncoder().encode(value).length <= 2000,
      "说明不能超过 2,000 UTF-8 字节",
    ),
});
export const planningDeclarationSchema = z.strictObject({
  request: planningDeclarationRequestSchema,
  declaredBy: z.literal("owner"),
  createdAt: z.string(),
  taskIds: z.array(z.string()),
});
export const planningStateSchema = z.strictObject({
  taskId: z.string(),
  scopeHash: z.string().regex(/^[a-f0-9]{64}$/),
  blockers: z.array(z.string()),
  declaration: planningDeclarationSchema.nullable(),
  current: z.boolean(),
});
export type PlanningState = z.infer<typeof planningStateSchema>;
export type PlanningDeclarationRequest = z.infer<
  typeof planningDeclarationRequestSchema
>;
