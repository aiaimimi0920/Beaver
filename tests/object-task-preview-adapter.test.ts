import assert from "node:assert/strict";
import test from "node:test";
import type { ObjectTaskSnapshot } from "../src/shared/object-tasks";
import {
  previewIterationsFromSnapshot,
  previewTasksFromSnapshot,
} from "../src/ui/object-preview/object-task-preview-adapter";

const snapshot = {
  planRevision: 3,
  runs: [],
  assumptions: [],
  coarseDispatchControls: [],
  dispatchControls: [],
  tasks: [
    {
      id: "fine-2",
      position: 20,
      granularity: "coarse",
      title: "Second",
      prompt: "second prompt",
      acceptance: "second acceptance",
      requirement: "required",
      pendingPlanning: "",
      projectId: "project",
      objectId: null,
      parentTaskId: null,
      dependsOn: [],
      runId: null,
      stageId: null,
      identity: { schemaVersion: 1, layer: "coarse" },
      status: "cancelled",
      revision: 1,
    },
    {
      id: "coarse-1",
      position: 10,
      granularity: "coarse",
      title: "First",
      prompt: "first prompt",
      acceptance: "first acceptance",
      requirement: "required",
      pendingPlanning: "",
      projectId: "project",
      objectId: null,
      parentTaskId: null,
      dependsOn: [],
      runId: null,
      stageId: null,
      identity: { schemaVersion: 1, layer: "coarse" },
      status: "planned",
      revision: 1,
    },
  ],
} as ObjectTaskSnapshot;

test("maps persisted task identity, ordering, and cancellation without fake progress", () => {
  const tasks = previewTasksFromSnapshot(snapshot);
  assert.deepEqual(
    tasks.map((task) => task.id),
    ["coarse-1", "fine-2"],
  );
  assert.equal(tasks[0]?.level, "粗修");
  assert.equal(tasks[0]?.status, "排队中");
  assert.equal(tasks[1]?.status, "执行失败");
  assert.equal(tasks[1]?.parentId, null);
  assert.equal(tasks[1]?.progress, 0);
  assert.match(tasks[1]?.detail ?? "", /任务已取消/);
});

test("maps one medium task per object iteration and excludes fine children", () => {
  const iterations = previewIterationsFromSnapshot(
    {
      ...snapshot,
      tasks: [
        {
          ...snapshot.tasks[0]!,
          id: "fine-a",
          position: 30,
          granularity: "fine",
          objectId: "object-a",
          parentTaskId: "medium-a",
          runId: "run-a",
          stageId: "stage-1",
          identity: {
            schemaVersion: 1,
            layer: "fine",
            objectId: "object-a",
            mediumTaskId: "medium-a",
            runId: "run-a",
            stageId: "stage-1",
          },
        },
        {
          ...snapshot.tasks[0]!,
          id: "medium-b",
          position: 20,
          granularity: "medium",
          objectId: "object-b",
          parentTaskId: null,
          runId: "run-b",
          identity: {
            schemaVersion: 1,
            layer: "medium",
            objectId: "object-b",
          },
        },
        {
          ...snapshot.tasks[0]!,
          id: "medium-a",
          position: 10,
          granularity: "medium",
          objectId: "object-a",
          parentTaskId: null,
          runId: "run-a",
          identity: {
            schemaVersion: 1,
            layer: "medium",
            objectId: "object-a",
          },
        },
      ],
    } as ObjectTaskSnapshot,
    "object-a",
  );

  assert.deepEqual(
    iterations.map((task) => task.id),
    ["medium-a"],
  );
  assert.equal(iterations[0]?.level, "中修");
  assert.equal(iterations[0]?.objectId, "object-a");
});
