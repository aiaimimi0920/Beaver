import {
  objectTaskPlanSchema,
  type ObjectTaskCommitReceipt,
  type ObjectTaskDraft,
  type ObjectTaskPlan,
  type ObjectTaskSnapshot,
} from "../../shared/object-tasks";

export type ObjectTaskWorkspaceState =
  | { kind: "loading" }
  | { kind: "error"; message: string }
  | {
      kind: "ready";
      snapshot: ObjectTaskSnapshot;
      plan: ObjectTaskPlan;
      savedPlan: ObjectTaskPlan;
      savedPlanRevision: number;
      draftRevision: number;
      committedRequestId?: string;
      planRevision: number;
      dirty: boolean;
      operation: "idle" | "refreshing" | "saving" | "committing" | "unlocking";
      conflict: "draft" | "plan" | null;
      error: string | null;
      receipt: ObjectTaskCommitReceipt | null;
    };

export function emptyPlan(): ObjectTaskPlan {
  return objectTaskPlanSchema.parse({});
}

export function samePlan(left: ObjectTaskPlan, right: ObjectTaskPlan): boolean {
  return JSON.stringify(left) === JSON.stringify(right);
}

export function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export function conflictKind(message: string): "draft" | "plan" | null {
  if (message.includes("OBJECT_TASK_DRAFT_REVISION_CONFLICT")) return "draft";
  if (message.includes("OBJECT_TASK_PLAN_REVISION_CONFLICT")) return "plan";
  return null;
}

export function workspaceFromRemote(
  snapshot: ObjectTaskSnapshot,
  draft: ObjectTaskDraft | null,
): ObjectTaskWorkspaceState {
  const plan = draft?.plan ?? emptyPlan();
  return {
    kind: "ready",
    snapshot,
    plan,
    savedPlan: plan,
    savedPlanRevision: draft?.planRevision ?? snapshot.planRevision,
    draftRevision: draft?.revision ?? 0,
    committedRequestId: draft?.committedRequestId,
    planRevision: draft?.planRevision ?? snapshot.planRevision,
    dirty: false,
    operation: "idle",
    conflict:
      draft &&
      !draft.committedRequestId &&
      draft.planRevision !== snapshot.planRevision
        ? "plan"
        : null,
    error: null,
    receipt: null,
  };
}
