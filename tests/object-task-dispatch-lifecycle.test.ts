import assert from "node:assert/strict";
import test from "node:test";
import {
  dispatchControl,
  dispatchReceipt,
  dispatchSession,
  deferred,
} from "./fixtures/object-task-dispatch";
import { objectTaskSnapshot } from "./fixtures/object-tasks";

test("closing rejects late query and write responses and skips workspace refresh", async () => {
  for (const write of [false, true]) {
    for (const failure of [false, true]) {
      const response = deferred<unknown>();
      let request: unknown;
      let refreshes = 0;
      const session = dispatchSession(
        async (method, input) => {
          if (write && method === "objectTask.snapshot")
            return objectTaskSnapshot();
          request = input;
          return response.promise;
        },
        async () => {
          refreshes++;
          return true;
        },
      );
      if (write) await session.refresh();
      const pending = write ? session.confirm() : session.refresh();
      session.cancel();
      const closed = session.getSnapshot();
      if (failure) response.reject(new Error("Late error"));
      else
        response.resolve(
          write ? dispatchReceipt(request) : objectTaskSnapshot(),
        );
      await pending;
      assert.equal(session.getSnapshot(), closed);
      assert.equal(refreshes, 0);
    }
  }
});

test("project or run changes fail closed before a new dispatch control request", async () => {
  const cases = [
    objectTaskSnapshot("other"),
    objectTaskSnapshot(),
    objectTaskSnapshot(),
  ];
  cases[1]!.dispatchControls = [{ ...dispatchControl(), objectId: "other" }];
  const rebound = cases[2]!;
  for (const task of rebound.tasks) {
    if (task.runId === "run-hero") task.runId = "run-rebound";
    if (task.identity.layer === "fine") task.identity.runId = "run-rebound";
  }
  rebound.runs.find((run) => run.id === "run-hero")!.id = "run-rebound";
  for (const snapshot of cases) {
    let writes = 0;
    const session = dispatchSession(async (method) => {
      if (method === "objectTask.snapshot") return snapshot;
      writes++;
      throw new Error("Unexpected write");
    });
    assert.equal(await session.refresh(), false);
    assert.equal(await session.confirm(), null);
    assert.equal(writes, 0);
  }
});

test("cancelled tasks and exhausted control revisions cannot initiate a new write", async () => {
  for (const cancelled of [false, true]) {
    const snapshot = objectTaskSnapshot();
    if (cancelled) {
      snapshot.tasks.find((task) => task.id === "medium")!.status = "cancelled";
      snapshot.runs.find((run) => run.id === "run-hero")!.status = "cancelled";
    } else
      snapshot.dispatchControls = [
        dispatchControl(true, Number.MAX_SAFE_INTEGER),
      ];
    let writes = 0;
    const session = dispatchSession(async (method) => {
      if (method === "objectTask.snapshot") return snapshot;
      writes++;
      throw new Error("Unexpected write");
    });
    await session.refresh();
    assert.equal(await session.confirm(), null);
    assert.equal(writes, 0);
  }
});

test("closing during a receipt refresh retains confirmation and ignores its late failure", async () => {
  const refresh = deferred<boolean>();
  let reachedRefresh = false;
  const session = dispatchSession(
    async (method, input) =>
      method === "objectTask.snapshot"
        ? objectTaskSnapshot()
        : dispatchReceipt(input),
    async () => {
      reachedRefresh = true;
      return refresh.promise;
    },
  );
  await session.refresh();
  const pending = session.confirm();
  await new Promise<void>((resolve) => setImmediate(resolve));
  assert.equal(reachedRefresh, true);
  assert.ok(session.getSnapshot().receipt);
  session.cancel();
  const closed = session.getSnapshot();
  refresh.reject(new Error("Late workspace failure"));
  assert.equal(await pending, null);
  assert.equal(session.getSnapshot(), closed);
  assert.equal(closed.phase, "succeeded");
});
