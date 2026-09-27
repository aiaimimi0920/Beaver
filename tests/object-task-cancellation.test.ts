import assert from "node:assert/strict";
import test from "node:test";
import {
  cancelPlannedObjectTaskSchema,
  objectTaskPlanSchema,
  type ObjectTaskCancellationReceipt,
} from "../src/shared/object-tasks";
import { ObjectTaskCancellation } from "../src/ui/object-tasks/object-task-cancellation";
import { ObjectTaskWorkspace } from "../src/ui/object-tasks/object-task-workspace";
import {
  cancelledObjectTaskSnapshot,
  objectTaskCancellationReceipt as receipt,
  objectTaskSnapshot,
} from "./fixtures/object-tasks";

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: Error) => void;
  const promise = new Promise<T>((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}

const requestId = () => "cancel-medium";
const refreshed = async () => true;
const unusedApi = async () => {
  throw new Error("Unexpected API call");
};

test("cancellation preview follows responsibility, not shared objects or dependencies", () => {
  const snapshot = objectTaskSnapshot();
  const independent = snapshot.tasks.find((task) => task.id === "independent")!;
  independent.dependsOn = ["medium"];
  const session = new ObjectTaskCancellation(
    "p",
    snapshot,
    "coarse",
    unusedApi,
    refreshed,
  );
  assert.deepEqual(session.tasks.map((task) => task.id).sort(), [
    "coarse",
    "fine",
    "medium",
  ]);
  assert.deepEqual(
    session.runs.map((run) => run.id),
    ["run-hero"],
  );
  const fine = new ObjectTaskCancellation(
    "p",
    snapshot,
    "fine",
    unusedApi,
    refreshed,
  );
  assert.deepEqual(
    fine.tasks.map((task) => task.id),
    ["fine"],
  );
  assert.equal(fine.runs.length, 0);
  snapshot.tasks.find((task) => task.id === "fine")!.status = "cancelled";
  const medium = new ObjectTaskCancellation(
    "p",
    snapshot,
    "medium",
    unusedApi,
    refreshed,
  );
  assert.deepEqual(
    medium.tasks.map((task) => task.id),
    ["medium"],
  );
  assert.equal(medium.runs.length, 1);
  assert.throws(
    () =>
      new ObjectTaskCancellation("p", snapshot, "fine", unusedApi, refreshed),
  );
  assert.throws(
    () =>
      new ObjectTaskCancellation(
        "other",
        snapshot,
        "medium",
        unusedApi,
        refreshed,
      ),
  );
});

test("duplicate submission is suppressed and a lost-response retry keeps the reviewed request", async () => {
  const lost = deferred<unknown>();
  const snapshot = objectTaskSnapshot();
  const inputs: unknown[] = [];
  let ids = 0;
  let refreshes = 0;
  const session = new ObjectTaskCancellation(
    "p",
    snapshot,
    "medium",
    async (method, input) => {
      assert.equal(method, "objectTask.cancelPlanned");
      inputs.push(structuredClone(input));
      return inputs.length === 1 ? lost.promise : receipt;
    },
    async () => {
      refreshes++;
      return true;
    },
    () => {
      ids++;
      return requestId();
    },
  );

  const first = session.submit();
  assert.equal(await session.submit(), null);
  assert.equal(inputs.length, 1);
  snapshot.planRevision = 99;
  snapshot.tasks.find((task) => task.id === "medium")!.revision = 12;
  lost.reject(new Error("Response lost after the server committed"));
  assert.equal(await first, null);
  assert.equal(session.getSnapshot().phase, "failed");
  assert.deepEqual(await session.submit(), receipt);
  assert.deepEqual(inputs, [
    {
      projectId: "p",
      taskId: "medium",
      requestId: "cancel-medium",
      expectedTaskRevision: 0,
      expectedPlanRevision: 1,
    },
    {
      projectId: "p",
      taskId: "medium",
      requestId: "cancel-medium",
      expectedTaskRevision: 0,
      expectedPlanRevision: 1,
    },
  ]);
  assert.equal(ids, 1);
  assert.equal(refreshes, 1);
  assert.deepEqual(await session.submit(), receipt);
  assert.equal(inputs.length, 2);
});

test("receipt identity, exact revisions and unknown fields are validated before refresh", async () => {
  const invalid: Record<string, unknown>[] = [
    { projectId: "other" },
    { taskId: "fine" },
    { requestId: "another-request" },
    { previousTaskRevision: 1 },
    { taskRevision: 2 },
    { previousPlanRevision: 0 },
    { planRevision: 3 },
    { execute: true },
  ];
  let refreshes = 0;
  for (const patch of invalid) {
    const session = new ObjectTaskCancellation(
      "p",
      objectTaskSnapshot(),
      "medium",
      async () => ({ ...receipt, ...patch }),
      async () => {
        refreshes++;
        return true;
      },
      requestId,
    );
    assert.equal(await session.submit(), null);
    assert.equal(session.getSnapshot().phase, "failed");
    assert.equal(session.getSnapshot().receipt, null);
  }
  assert.equal(refreshes, 0);
  assert.throws(() =>
    cancelPlannedObjectTaskSchema.parse({
      projectId: "p",
      taskId: "medium",
      requestId: "cancel-medium",
      expectedTaskRevision: 0,
      expectedPlanRevision: 1,
      execute: true,
    }),
  );
});

test("closing a previous project's session ignores its late receipt and refresh", async () => {
  const response = deferred<unknown>();
  const refreshedProjects: string[] = [];
  const old = new ObjectTaskCancellation(
    "p",
    objectTaskSnapshot(),
    "medium",
    async () => response.promise,
    async () => {
      refreshedProjects.push("p");
      return true;
    },
    requestId,
  );
  const pending = old.submit();
  old.cancel();
  const next = new ObjectTaskCancellation(
    "next",
    objectTaskSnapshot("next"),
    "medium",
    async () => ({ ...receipt, projectId: "next" }),
    async () => {
      refreshedProjects.push("next");
      return true;
    },
    requestId,
  );
  await next.submit();
  response.resolve(receipt);
  assert.equal(await pending, null);
  assert.equal(old.getSnapshot().receipt, null);
  assert.equal(next.getSnapshot().receipt?.projectId, "next");
  assert.deepEqual(refreshedProjects, ["next"]);
});

test("initial lifecycle cleanup does not disable a later confirmation", async () => {
  const session = new ObjectTaskCancellation(
    "p",
    objectTaskSnapshot(),
    "medium",
    async () => receipt,
    refreshed,
    requestId,
  );
  session.cancel();
  assert.deepEqual(await session.submit(), receipt);
});

test("confirmed receipt survives refresh failures and retries never send another cancellation", async () => {
  let writes = 0;
  let reads = 0;
  const session = new ObjectTaskCancellation(
    "p",
    objectTaskSnapshot(),
    "medium",
    async () => {
      writes++;
      return receipt;
    },
    async () => {
      reads++;
      if (reads === 1) return false;
      if (reads === 2) throw new Error("Snapshot temporarily unavailable");
      return true;
    },
    requestId,
  );
  assert.deepEqual(await session.submit(), receipt);
  assert.equal(session.getSnapshot().phase, "succeeded");
  assert.match(session.getSnapshot().error, /刷新失败/);
  assert.deepEqual(await session.submit(), receipt);
  assert.equal(await session.refresh(), false);
  assert.deepEqual(session.getSnapshot().receipt, receipt);
  assert.equal(await session.refresh(), true);
  assert.equal(session.getSnapshot().error, "");
  assert.equal(writes, 1);
  assert.equal(reads, 3);
});

test("a close at receipt notification prevents starting a stale refresh", async () => {
  let refreshes = 0;
  const session = new ObjectTaskCancellation(
    "p",
    objectTaskSnapshot(),
    "medium",
    async () => receipt,
    async () => {
      refreshes++;
      return true;
    },
    requestId,
  );
  session.subscribe(() => {
    if (session.getSnapshot().receipt) session.cancel();
  });
  assert.equal(await session.submit(), null);
  assert.equal(refreshes, 0);
});

test("successful cancellation refreshes the real workspace while retaining unsaved assumptions and tasks", async () => {
  let snapshot = objectTaskSnapshot();
  const api = async (method: string, input: unknown): Promise<unknown> => {
    if (method === "objectTask.snapshot") return snapshot;
    if (method === "objectTask.getDraft") return null;
    if (method === "objectTask.cancelPlanned") {
      assert.equal(
        cancelPlannedObjectTaskSchema.parse(input).expectedPlanRevision,
        1,
      );
      snapshot = cancelledObjectTaskSnapshot();
      return receipt;
    }
    throw new Error(`Unexpected method: ${method}`);
  };
  const workspace = new ObjectTaskWorkspace("p", api);
  await workspace.refresh();
  const plan = objectTaskPlanSchema.parse({
    tasks: [
      {
        id: "coarse-next",
        granularity: "coarse",
        title: "Next scene",
        prompt: "Plan a new scene",
        acceptance: "Playable",
      },
    ],
    assumptions: [
      { id: "colors", statement: "Keep warm colors", basis: "User request" },
    ],
  });
  workspace.updatePlan(plan);
  const session = new ObjectTaskCancellation(
    "p",
    snapshot,
    "medium",
    api,
    workspace.refresh,
    requestId,
  );
  assert.deepEqual(
    await session.submit(),
    receipt satisfies ObjectTaskCancellationReceipt,
  );
  const state = workspace.getSnapshot();
  assert.ok(state.kind === "ready");
  assert.deepEqual(state.snapshot, cancelledObjectTaskSnapshot());
  assert.deepEqual(state.plan, plan);
  assert.equal(state.conflict, "plan");
  assert.equal(state.dirty, true);
  assert.equal(state.planRevision, 1);
  assert.equal(session.getSnapshot().error, "");
});
