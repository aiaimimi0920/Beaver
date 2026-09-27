import assert from "node:assert/strict";
import test from "node:test";
import {
  objectTaskPlanSchema,
  saveObjectTaskDraftSchema,
  type ObjectTaskPlan,
  type SaveObjectTaskDraftRequest,
} from "../src/shared/object-tasks";
import { ObjectTaskWorkspace } from "../src/ui/object-tasks/object-task-workspace";
import { objectTaskSnapshot } from "./fixtures/object-tasks";

const localPlan = objectTaskPlanSchema.parse({
  tasks: [
    {
      id: "new-coarse",
      granularity: "coarse",
      title: "Improve the scene",
      prompt: "Keep the current art direction.",
      acceptance: "The scene remains playable.",
    },
  ],
  assumptions: [
    { id: "palette", statement: "Keep warm colors.", basis: "User direction" },
  ],
});

function ready(workspace: ObjectTaskWorkspace) {
  const state = workspace.getSnapshot();
  assert.ok(state.kind === "ready");
  return state;
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((yes) => {
    resolve = yes;
  });
  return { promise, resolve };
}

function harness(savedPlan: ObjectTaskPlan = objectTaskPlanSchema.parse({})) {
  const remote = {
    snapshot: objectTaskSnapshot(),
    draft: {
      projectId: "p",
      id: "object-task-plan",
      revision: 1,
      planRevision: 1,
      plan: savedPlan,
    },
    pendingSnapshot: null as Promise<unknown> | null,
  };
  const saves: SaveObjectTaskDraftRequest[] = [];
  const workspace = new ObjectTaskWorkspace("p", async (method, input) => {
    if (method === "objectTask.snapshot")
      return remote.pendingSnapshot ?? remote.snapshot;
    if (method === "objectTask.getDraft") return remote.draft;
    if (method === "objectTask.saveDraft") {
      const request = saveObjectTaskDraftSchema.parse(input);
      saves.push(request);
      assert.equal(request.expectedRevision, remote.draft.revision);
      assert.equal(request.expectedPlanRevision, remote.snapshot.planRevision);
      remote.draft = {
        ...remote.draft,
        revision: request.expectedRevision + 1,
        planRevision: request.expectedPlanRevision,
        plan: request.plan,
      };
      return remote.draft;
    }
    throw new Error(`Unexpected method: ${method}`);
  });
  return { workspace, remote, saves };
}

test("detached refresh remains bound to its project workspace", async () => {
  const { workspace } = harness();
  const { refresh } = workspace;
  assert.equal(await refresh(), true);
  assert.equal(ready(workspace).snapshot.planRevision, 1);
});

test("reviewed plan rebase preserves local tasks and assumptions through refresh and save", async () => {
  const { workspace, remote, saves } = harness();
  await workspace.refresh();
  workspace.updatePlan(localPlan);
  remote.snapshot.planRevision = 2;
  await workspace.refresh();
  assert.equal(ready(workspace).conflict, "plan");
  assert.equal(await workspace.save(), false);
  assert.equal(await workspace.keepLocalDraftAndRebaseRevision(), true);
  assert.equal(ready(workspace).planRevision, 2);
  assert.equal(ready(workspace).savedPlanRevision, 1);
  assert.deepEqual(ready(workspace).plan, localPlan);

  await workspace.refresh();
  assert.equal(ready(workspace).conflict, null);
  assert.equal(ready(workspace).dirty, true);
  assert.equal(await workspace.save(), true);
  assert.equal(saves.length, 1);
  assert.equal(saves[0]?.expectedPlanRevision, 2);
  assert.deepEqual(remote.draft.plan, localPlan);
  assert.equal(ready(workspace).savedPlanRevision, 2);
  assert.equal(ready(workspace).dirty, false);
});

test("identical saved content still persists its explicitly adopted plan baseline", async () => {
  const { workspace, remote, saves } = harness(localPlan);
  remote.snapshot.planRevision = 2;
  await workspace.refresh();
  assert.equal(ready(workspace).dirty, false);
  assert.equal(ready(workspace).conflict, "plan");
  assert.equal(await workspace.keepLocalDraftAndRebaseRevision(), true);
  assert.equal(ready(workspace).dirty, true);
  workspace.updatePlan(ready(workspace).plan);
  assert.equal(ready(workspace).dirty, true);
  assert.equal(await workspace.save(), true);
  assert.equal(saves.length, 1);
  assert.equal(remote.draft.planRevision, 2);
  await workspace.refresh();
  assert.equal(ready(workspace).conflict, null);
  assert.equal(ready(workspace).dirty, false);
});

test("a second remote plan change requires review again without discarding edits", async () => {
  const { workspace, remote } = harness();
  await workspace.refresh();
  workspace.updatePlan(localPlan);
  remote.snapshot.planRevision = 2;
  await workspace.refresh();
  const response = deferred<unknown>();
  remote.pendingSnapshot = response.promise;
  const rebasing = workspace.keepLocalDraftAndRebaseRevision();
  remote.snapshot.planRevision = 3;
  response.resolve(remote.snapshot);
  assert.equal(await rebasing, false);
  assert.equal(ready(workspace).snapshot.planRevision, 3);
  assert.equal(ready(workspace).planRevision, 1);
  assert.equal(ready(workspace).conflict, "plan");
  assert.deepEqual(ready(workspace).plan, localPlan);
  remote.pendingSnapshot = null;
  assert.equal(await workspace.keepLocalDraftAndRebaseRevision(), true);
  assert.equal(ready(workspace).planRevision, 3);
});

test("local edits during a rebase recheck stay intact and require another confirmation", async () => {
  const { workspace, remote } = harness(localPlan);
  remote.snapshot.planRevision = 2;
  await workspace.refresh();
  const response = deferred<unknown>();
  remote.pendingSnapshot = response.promise;
  const rebasing = workspace.keepLocalDraftAndRebaseRevision();
  const edited = {
    ...localPlan,
    objects: [{ id: "hero", name: "Hero", category: "角色" }],
  };
  workspace.updatePlan(edited);
  response.resolve(remote.snapshot);
  assert.equal(await rebasing, false);
  assert.deepEqual(ready(workspace).plan, edited);
  assert.equal(ready(workspace).planRevision, 1);
  assert.equal(ready(workspace).conflict, "plan");
  remote.pendingSnapshot = null;
  assert.equal(await workspace.keepLocalDraftAndRebaseRevision(), true);
  assert.deepEqual(ready(workspace).plan, edited);
});

test("starting an empty draft on a newer plan still saves the new baseline", async () => {
  const { workspace, remote } = harness();
  remote.snapshot.planRevision = 2;
  await workspace.refresh();
  workspace.startNewPlan();
  assert.equal(ready(workspace).dirty, true);
  assert.equal(await workspace.save(), true);
  assert.equal(remote.draft.planRevision, 2);
});
