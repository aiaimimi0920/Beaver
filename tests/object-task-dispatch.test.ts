import assert from "node:assert/strict";
import test from "node:test";
import {
  setObjectTaskPausedSchema,
  type SetObjectTaskPausedRequest,
} from "../src/shared/object-task-dispatch";
import { parseObjectTaskSnapshot } from "../src/shared/object-task-snapshot";
import {
  dispatchControl,
  dispatchReceipt,
  dispatchSession,
  deferred,
} from "./fixtures/object-task-dispatch";
import { objectTaskSnapshot } from "./fixtures/object-tasks";

test("dispatch snapshot requires controls bound to unique project mediums and safe revisions", () => {
  const snapshot = objectTaskSnapshot();
  snapshot.dispatchControls = [dispatchControl()];
  assert.deepEqual(parseObjectTaskSnapshot(snapshot, "p"), snapshot);
  const mutations = [
    { projectId: "other" },
    { taskId: "fine" },
    { taskId: "missing" },
    { objectId: "other" },
    { runId: "run-independent" },
    { revision: 0 },
    { revision: -1 },
    { revision: Number.MAX_SAFE_INTEGER + 1 },
    { paused: "true" },
    { schemaVersion: 2 },
    { extra: true },
  ];
  for (const mutation of mutations) {
    assert.throws(() =>
      parseObjectTaskSnapshot(
        {
          ...snapshot,
          dispatchControls: [{ ...dispatchControl(), ...mutation }],
        },
        "p",
      ),
    );
  }
  assert.throws(() =>
    parseObjectTaskSnapshot({ ...snapshot, dispatchControls: undefined }, "p"),
  );
  assert.throws(() =>
    parseObjectTaskSnapshot(
      { ...snapshot, dispatchControls: [dispatchControl(), dispatchControl()] },
      "p",
    ),
  );
});

test("pause and unpause bind the reviewed task and control without changing task lifecycle", async () => {
  for (const paused of [false, true]) {
    const snapshot = objectTaskSnapshot();
    snapshot.dispatchControls = paused ? [dispatchControl(true, 3)] : [];
    const before = structuredClone(snapshot);
    const writes: SetObjectTaskPausedRequest[] = [];
    const session = dispatchSession(async (method, input) => {
      if (method === "objectTask.snapshot") {
        assert.deepEqual(input, { projectId: "p" });
        return snapshot;
      }
      assert.equal(method, "objectTask.setPaused");
      writes.push(setObjectTaskPausedSchema.parse(input));
      return dispatchReceipt(input);
    });
    assert.equal(await session.confirm(), null);
    await session.refresh();
    const receipt = await session.confirm();
    assert.equal(receipt?.result.paused, !paused);
    assert.deepEqual(writes, [
      {
        projectId: "p",
        taskId: "medium",
        objectId: "hero",
        runId: "run-hero",
        requestId: "pause-1",
        expectedTaskRevision: 0,
        expectedControlRevision: paused ? 3 : 0,
        paused: !paused,
      },
    ]);
    assert.deepEqual(snapshot, before);
  }
});

test("ambiguous writes retry the exact request after newer snapshots and API input mutation", async () => {
  const lost = deferred<unknown>();
  let snapshot = objectTaskSnapshot();
  let ids = 0;
  const writes: SetObjectTaskPausedRequest[] = [];
  const session = dispatchSession(
    async (method, input) => {
      if (method === "objectTask.snapshot") return snapshot;
      writes.push(setObjectTaskPausedSchema.parse(input));
      Object.assign(input as object, { requestId: "mutated" });
      return writes.length === 1 ? lost.promise : dispatchReceipt(writes[0]);
    },
    async () => true,
    () => `pause-${++ids}`,
  );
  await session.refresh();
  const pending = session.confirm();
  assert.equal(await session.confirm(), null);
  assert.equal(await session.refresh(), false);
  lost.reject(new Error("Response lost after commit"));
  assert.equal(await pending, null);
  assert.equal(session.getSnapshot().retryAvailable, true);
  snapshot = objectTaskSnapshot();
  snapshot.dispatchControls = [dispatchControl(false, 2)];
  snapshot.tasks.find((task) => task.id === "medium")!.revision = 4;
  await session.refresh();
  const receipt = await session.confirm();
  assert.ok(receipt);
  assert.deepEqual(writes[1], writes[0]);
  assert.equal(ids, 1);
  assert.equal(session.getSnapshot().control.paused, false);
  assert.equal(receipt.result.paused, true);
  assert.equal(await session.confirm(), receipt);
  assert.equal(writes.length, 2);
});

test("explicit conflicts require requery and a fresh confirmation; uncertain messages preserve retries", async () => {
  for (const error of [
    "OBJECT_TASK_REVISION_CONFLICT",
    "OBJECT_TASK_DISPATCH_REVISION_CONFLICT",
    "OBJECT_TASK_DISPATCH_NOT_CONTROLLABLE",
  ]) {
    let ids = 0;
    const snapshot = objectTaskSnapshot();
    const writes: SetObjectTaskPausedRequest[] = [];
    const session = dispatchSession(
      async (method, input) => {
        if (method === "objectTask.snapshot") return snapshot;
        writes.push(setObjectTaskPausedSchema.parse(input));
        if (writes.length === 1) throw new Error(error);
        return dispatchReceipt(input);
      },
      async () => true,
      () => `pause-${++ids}`,
    );
    await session.refresh();
    assert.equal(await session.confirm(), null);
    assert.equal(session.getSnapshot().retryAvailable, false);
    assert.equal(await session.confirm(), null);
    snapshot.dispatchControls = [dispatchControl(true, 2)];
    snapshot.tasks.find((task) => task.id === "medium")!.revision = 5;
    await session.refresh();
    assert.ok(await session.confirm());
    assert.equal(writes[1]?.requestId, "pause-2");
    assert.equal(writes[1]?.expectedTaskRevision, 5);
    assert.equal(writes[1]?.expectedControlRevision, 2);
    assert.equal(writes[1]?.paused, false);
  }
  const session = dispatchSession(async (method) => {
    if (method === "objectTask.snapshot") return objectTaskSnapshot();
    throw new Error("Response lost: OBJECT_TASK_DISPATCH_REVISION_CONFLICT");
  });
  await session.refresh();
  await session.confirm();
  assert.equal(session.getSnapshot().retryAvailable, true);
});

test("mismatched or malformed receipts cannot confirm a control write", async () => {
  for (const alter of [
    (r: ReturnType<typeof dispatchReceipt>) => {
      r.request.requestId = "other";
    },
    (r: ReturnType<typeof dispatchReceipt>) => {
      r.result.objectId = "other";
    },
    (r: ReturnType<typeof dispatchReceipt>) => {
      r.result.projectId = "other";
    },
    (r: ReturnType<typeof dispatchReceipt>) => {
      r.result.runId = "other";
    },
    (r: ReturnType<typeof dispatchReceipt>) => {
      r.result.paused = false;
    },
    (r: ReturnType<typeof dispatchReceipt>) => {
      r.result.revision++;
    },
  ]) {
    let refreshes = 0;
    const session = dispatchSession(
      async (method, input) => {
        if (method === "objectTask.snapshot") return objectTaskSnapshot();
        const receipt = dispatchReceipt(input);
        alter(receipt);
        return receipt;
      },
      async () => {
        refreshes++;
        return true;
      },
    );
    await session.refresh();
    assert.equal(await session.confirm(), null);
    assert.equal(session.getSnapshot().receipt, null);
    assert.equal(session.getSnapshot().retryAvailable, true);
    assert.equal(refreshes, 0);
  }
});

test("confirmed receipt survives workspace refresh failures without resubmitting", async () => {
  let writes = 0;
  let refreshes = 0;
  const session = dispatchSession(
    async (method, input) => {
      if (method === "objectTask.snapshot") return objectTaskSnapshot();
      writes++;
      return dispatchReceipt(input);
    },
    async () => {
      refreshes++;
      if (refreshes === 1) return false;
      if (refreshes === 2) throw new Error("Project closed");
      return true;
    },
  );
  await session.refresh();
  const receipt = await session.confirm();
  assert.ok(receipt);
  assert.equal(session.getSnapshot().phase, "succeeded");
  assert.match(session.getSnapshot().error, /回执已确认.*刷新失败/);
  assert.equal(await session.confirm(), receipt);
  assert.equal(await session.refresh(), false);
  assert.equal(await session.refresh(), true);
  assert.equal(session.getSnapshot().receipt, receipt);
  assert.equal(writes, 1);
});
