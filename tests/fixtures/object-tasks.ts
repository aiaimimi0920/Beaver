import type {
  ObjectTaskCancellationReceipt,
  ObjectTaskRecord,
  ObjectTaskSnapshot,
} from "../../src/shared/object-tasks";

export function objectTaskSnapshot(projectId = "p"): ObjectTaskSnapshot {
  const coarse: ObjectTaskRecord = {
    id: "coarse",
    position: 0,
    projectId,
    granularity: "coarse",
    title: "Game",
    prompt: "Make a game",
    acceptance: "Playable",
    requirement: "required",
    pendingPlanning: "",
    objectId: null,
    parentTaskId: null,
    dependsOn: [],
    runId: null,
    stageId: null,
    identity: { schemaVersion: 1, layer: "coarse" },
    status: "planned",
    revision: 0,
  };
  const medium: ObjectTaskRecord = {
    ...coarse,
    id: "medium",
    granularity: "medium",
    title: "Hero",
    prompt: "Make a hero",
    objectId: "hero",
    parentTaskId: "coarse",
    runId: "run-hero",
    identity: {
      schemaVersion: 1,
      layer: "medium",
      objectId: "hero",
      baseline: { basePolicy: "latestAccepted" },
    },
  };
  const fine: ObjectTaskRecord = {
    ...medium,
    id: "fine",
    granularity: "fine",
    title: "Movement",
    prompt: "Add movement",
    parentTaskId: "medium",
    stageId: "movement",
    dependsOn: ["independent"],
    identity: {
      schemaVersion: 1,
      layer: "fine",
      objectId: "hero",
      mediumTaskId: "medium",
      runId: "run-hero",
      stageId: "movement",
    },
  };
  const independent: ObjectTaskRecord = {
    ...medium,
    id: "independent",
    title: "Independent iteration",
    parentTaskId: null,
    runId: "run-independent",
  };
  return {
    planRevision: 1,
    tasks: [fine, coarse, independent, medium],
    runs: [medium, independent].map((task) => ({
      id: task.runId!,
      projectId,
      objectId: "hero",
      mediumTaskId: task.id,
      baselineVersionId: null,
      status: "planned",
      revision: 0,
    })),
    assumptions: [],
    coarseDispatchControls: [],
    dispatchControls: [],
  };
}

export function cancelledObjectTaskSnapshot(): ObjectTaskSnapshot {
  const snapshot = objectTaskSnapshot();
  snapshot.planRevision = 2;
  for (const task of snapshot.tasks) {
    if (task.id === "medium" || task.id === "fine") {
      task.status = "cancelled";
      task.revision = 1;
    }
  }
  for (const run of snapshot.runs) {
    if (run.id === "run-hero") {
      run.status = "cancelled";
      run.revision = 1;
    }
  }
  return snapshot;
}

export const objectTaskCancellationReceipt: ObjectTaskCancellationReceipt = {
  projectId: "p",
  taskId: "medium",
  requestId: "cancel-medium",
  previousTaskRevision: 0,
  taskRevision: 1,
  previousPlanRevision: 1,
  planRevision: 2,
};
