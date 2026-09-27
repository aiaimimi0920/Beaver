import assert from "node:assert/strict";
import test from "node:test";
import {
  objectTaskDraftSchema,
  objectTaskPlanSchema,
  type ObjectTaskAssumption,
} from "../src/shared/object-tasks";
import { ObjectTaskWorkspace } from "../src/ui/object-tasks/object-task-workspace";
import { objectTaskSnapshot } from "./fixtures/object-tasks";

function plan() {
  return objectTaskPlanSchema.parse({
    objects: [{ id: "hero", name: "Hero" }],
    tasks: [
      {
        id: "coarse-new",
        granularity: "coarse",
        title: "Improve the game",
        prompt: "Improve the core loop.",
        acceptance: "The loop is playable.",
      },
    ],
  });
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((yes) => {
    resolve = yes;
  });
  return { promise, resolve };
}

function draft(revision: number, planRevision = 1) {
  return objectTaskDraftSchema.parse({
    projectId: "p",
    id: "object-task-plan",
    revision,
    planRevision,
    plan: {},
  });
}

test("refresh preserves edits made while remote state is loading", async () => {
  const snapshotResponse = deferred<unknown>();
  const draftResponse = deferred<unknown>();
  let slow = false;
  const workspace = new ObjectTaskWorkspace("p", async (method) => {
    if (method === "objectTask.snapshot") {
      return slow ? snapshotResponse.promise : objectTaskSnapshot();
    }
    if (method === "objectTask.getDraft") {
      return slow ? draftResponse.promise : draft(1);
    }
    throw new Error(`Unexpected method: ${method}`);
  });
  await workspace.refresh();
  slow = true;
  const refreshing = workspace.refresh();
  workspace.updatePlan(plan());
  snapshotResponse.resolve(objectTaskSnapshot());
  draftResponse.resolve(draft(1));
  await refreshing;
  const state = workspace.getSnapshot();
  assert.equal(state.kind, "ready");
  if (state.kind !== "ready") return;
  assert.equal(state.dirty, true);
  assert.deepEqual(state.plan, plan());
  assert.equal(state.conflict, null);
});

test("draft revision conflict keeps local content and rebases only after rechecking", async () => {
  let remoteDraft = draft(1);
  const saves: unknown[] = [];
  const workspace = new ObjectTaskWorkspace("p", async (method, input) => {
    if (method === "objectTask.snapshot") return objectTaskSnapshot();
    if (method === "objectTask.getDraft") return remoteDraft;
    if (method === "objectTask.saveDraft") {
      saves.push(input);
      if (saves.length === 1)
        throw new Error("OBJECT_TASK_DRAFT_REVISION_CONFLICT");
      const request = input as { expectedRevision: number; plan: unknown };
      remoteDraft = objectTaskDraftSchema.parse({
        projectId: "p",
        id: "object-task-plan",
        revision: request.expectedRevision + 1,
        planRevision: 1,
        plan: request.plan,
      });
      return remoteDraft;
    }
    throw new Error(`Unexpected method: ${method}`);
  });
  await workspace.refresh();
  workspace.updatePlan(plan());
  assert.equal(await workspace.save(), false);
  let state = workspace.getSnapshot();
  assert.equal(state.kind, "ready");
  if (state.kind !== "ready") return;
  assert.equal(state.conflict, "draft");
  assert.equal(state.dirty, true);
  assert.deepEqual(state.plan, plan());

  remoteDraft = draft(2);
  assert.equal(await workspace.keepLocalDraftAndRebaseRevision(), true);
  state = workspace.getSnapshot();
  assert.equal(state.kind, "ready");
  if (state.kind !== "ready") return;
  assert.equal(state.conflict, null);
  assert.equal(state.draftRevision, 2);
  assert.equal(state.dirty, true);
  assert.deepEqual(state.plan, plan());
  assert.equal(await workspace.save(), true);
  assert.deepEqual(
    saves.map(
      (value) => (value as { expectedRevision: number }).expectedRevision,
    ),
    [1, 2],
  );
});

test("plan revision conflict preserves local content and permits an explicit fresh draft", async () => {
  let currentSnapshot = objectTaskSnapshot();
  const workspace = new ObjectTaskWorkspace("p", async (method) => {
    if (method === "objectTask.snapshot") return currentSnapshot;
    if (method === "objectTask.getDraft") return draft(1);
    throw new Error(`Unexpected method: ${method}`);
  });
  await workspace.refresh();
  workspace.updatePlan(plan());
  currentSnapshot = { ...objectTaskSnapshot(), planRevision: 2 };
  await workspace.refresh();
  assert.equal(await workspace.save(), false);
  const state = workspace.getSnapshot();
  assert.equal(state.kind, "ready");
  if (state.kind !== "ready") return;
  assert.equal(state.conflict, "plan");
  assert.deepEqual(state.plan, plan());
  workspace.startNewPlan();
  const reset = workspace.getSnapshot();
  assert.equal(reset.kind, "ready");
  if (reset.kind !== "ready") return;
  assert.equal(reset.conflict, null);
  assert.equal(reset.planRevision, 2);
  assert.deepEqual(reset.plan, { objects: [], tasks: [], assumptions: [] });
});

test("saved assumptions survive workspace recreation and refresh", async () => {
  let remoteDraft: unknown = null;
  const api = async (method: string, input: unknown): Promise<unknown> => {
    if (method === "objectTask.snapshot") return objectTaskSnapshot();
    if (method === "objectTask.getDraft") return remoteDraft;
    if (method === "objectTask.saveDraft") {
      const request = input as {
        expectedRevision: number;
        expectedPlanRevision: number;
        plan: unknown;
      };
      remoteDraft = objectTaskDraftSchema.parse({
        projectId: "p",
        id: "object-task-plan",
        revision: request.expectedRevision + 1,
        planRevision: request.expectedPlanRevision,
        plan: request.plan,
      });
      return remoteDraft;
    }
    throw new Error(`Unexpected method: ${method}`);
  };
  const assumption: ObjectTaskAssumption = {
    id: "assumption-lighting",
    statement: "Use a warm palette for interior scenes.",
    basis: "The existing project art direction uses warm ambient light.",
    source: "automatic",
    sourceDetail: "Initial plan synthesis",
  };
  const firstWorkspace = new ObjectTaskWorkspace("p", api);
  await firstWorkspace.refresh();
  const initial = firstWorkspace.getSnapshot();
  assert.equal(initial.kind, "ready");
  if (initial.kind !== "ready") return;
  firstWorkspace.updatePlan({
    ...plan(),
    assumptions: [assumption],
  });
  assert.equal(await firstWorkspace.save(), true);

  const reopenedWorkspace = new ObjectTaskWorkspace("p", api);
  await reopenedWorkspace.refresh();
  const reopened = reopenedWorkspace.getSnapshot();
  assert.equal(reopened.kind, "ready");
  if (reopened.kind !== "ready") return;
  assert.deepEqual(reopened.plan.assumptions, [assumption]);
  assert.equal(reopened.dirty, false);
});

test("commit validates its receipt and retains the submitted plan read-only", async () => {
  let currentSnapshot = objectTaskSnapshot();
  let currentDraft = draft(0);
  const methods: string[] = [];
  const workspace = new ObjectTaskWorkspace("p", async (method, input) => {
    methods.push(method);
    if (method === "objectTask.snapshot") return currentSnapshot;
    if (method === "objectTask.getDraft") return currentDraft;
    if (method === "objectTask.saveDraft") {
      const request = input as {
        expectedRevision: number;
        expectedPlanRevision: number;
        plan: unknown;
      };
      currentDraft = objectTaskDraftSchema.parse({
        projectId: "p",
        id: "object-task-plan",
        revision: request.expectedRevision + 1,
        planRevision: request.expectedPlanRevision,
        plan: request.plan,
      });
      return currentDraft;
    }
    if (method === "objectTask.commit") {
      const request = input as {
        requestId: string;
        expectedDraftRevision: number;
        expectedPlanRevision: number;
      };
      currentSnapshot = { ...currentSnapshot, planRevision: 2 };
      currentDraft = {
        ...currentDraft,
        revision: 2,
        committedRequestId: request.requestId,
      };
      return {
        projectId: "p",
        requestId: request.requestId,
        draftId: "object-task-plan",
        draftRevision: request.expectedDraftRevision,
        previousPlanRevision: request.expectedPlanRevision,
        planRevision: 2,
        objectIds: ["hero"],
        taskIds: ["coarse-new"],
        runs: [],
      };
    }
    throw new Error(`Unexpected method: ${method}`);
  });
  await workspace.refresh();
  workspace.updatePlan(plan());
  assert.equal(await workspace.save(), true);
  assert.equal(await workspace.commit(), true);
  const state = workspace.getSnapshot();
  assert.equal(state.kind, "ready");
  if (state.kind !== "ready") return;
  assert.equal(state.snapshot.planRevision, 2);
  assert.equal(state.planRevision, 1);
  assert.equal(state.draftRevision, 2);
  assert.equal(state.dirty, false);
  assert.equal(state.receipt?.taskIds[0], "coarse-new");
  assert.deepEqual(state.plan, plan());
  assert.equal(state.committedRequestId, state.receipt?.requestId);
  assert.equal(state.conflict, null);
  assert.deepEqual(methods, [
    "objectTask.snapshot",
    "objectTask.getDraft",
    "objectTask.saveDraft",
    "objectTask.commit",
    "objectTask.snapshot",
    "objectTask.getDraft",
  ]);
  assert.equal(await workspace.commit(), false);
});
