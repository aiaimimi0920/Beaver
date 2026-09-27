import type {
  ObjectTaskDefinition,
  ObjectTaskDefinitionRevision,
  RevisePlannedObjectTaskRequest,
} from "../../src/shared/object-task-revisions";
import type { ObjectTaskSnapshot } from "../../src/shared/object-tasks";
import { ObjectTaskRevision } from "../../src/ui/object-tasks/object-task-revision";
import { objectTaskSnapshot } from "./object-tasks";

export const revisionDefinition: ObjectTaskDefinition = {
  title: "Playable hero",
  prompt: "Add keyboard movement",
  acceptance: "Arrow keys move the hero without leaving the map",
  requirement: "required",
  pendingPlanning: "",
  dependsOn: ["independent"],
};
export const revisionReason = "Clarify controls before execution";
export const revisionRequest: RevisePlannedObjectTaskRequest = {
  projectId: "p",
  taskId: "medium",
  requestId: "revise-medium",
  expectedTaskRevision: 0,
  expectedPlanRevision: 1,
  definition: revisionDefinition,
  reason: revisionReason,
};
export const revisionReceipt: ObjectTaskDefinitionRevision = {
  projectId: "p",
  taskId: "medium",
  requestId: "revise-medium",
  previousTaskRevision: 0,
  taskRevision: 1,
  previousPlanRevision: 1,
  planRevision: 2,
  before: {
    title: "Hero",
    prompt: "Make a hero",
    acceptance: "Playable",
    requirement: "required",
    pendingPlanning: "",
    dependsOn: [],
  },
  after: revisionDefinition,
  reason: revisionReason,
  adoptedBy: "owner",
  createdAt: "2026-09-23T12:00:00.000Z",
  affectedTaskIds: ["fine", "medium"],
};

export function committedRevisionSnapshot(): ObjectTaskSnapshot {
  const snapshot = objectTaskSnapshot();
  snapshot.planRevision = 2;
  Object.assign(
    snapshot.tasks.find((task) => task.id === "medium")!,
    structuredClone(revisionDefinition),
    { revision: 1 },
  );
  return snapshot;
}

export function editedRevision(
  api: (method: string, input: unknown) => Promise<unknown>,
  options: {
    snapshot?: ObjectTaskSnapshot;
    refresh?: () => Promise<boolean>;
    requestId?: () => string;
  } = {},
): ObjectTaskRevision {
  const session = new ObjectTaskRevision(
    "p",
    options.snapshot ?? objectTaskSnapshot(),
    "medium",
    api,
    options.refresh ?? (async () => true),
    options.requestId ?? (() => "revise-medium"),
  );
  session.updateDefinition(revisionDefinition);
  session.updateReason(revisionReason);
  return session;
}

export function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: Error) => void;
  const promise = new Promise<T>((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}
