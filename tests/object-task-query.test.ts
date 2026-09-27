import assert from "node:assert/strict";
import test from "node:test";
import { ObjectTaskQuery } from "../src/ui/object-tasks/object-task-query";
import { objectTaskSnapshot } from "./fixtures/object-tasks";

function deferred() {
  let resolve!: (value: unknown) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<unknown>((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}

test("object task refresh uses explicit project and ignores older success and failure responses", async () => {
  for (const lateFailure of [false, true]) {
    const requests: ReturnType<typeof deferred>[] = [];
    const query = new ObjectTaskQuery("p", async (method, input) => {
      assert.equal(method, "objectTask.snapshot");
      assert.deepEqual(input, { projectId: "p" });
      const response = deferred();
      requests.push(response);
      return response.promise;
    });
    const first = query.refresh();
    const second = query.refresh();
    const [older, newer] = requests;
    assert.ok(older);
    assert.ok(newer);
    assert.equal(query.getSnapshot().kind, "loading");
    const newest = { ...objectTaskSnapshot(), planRevision: 2 };
    newer.resolve(newest);
    await second;
    if (lateFailure) older.reject(new Error("Old response"));
    else older.resolve(objectTaskSnapshot());
    await first;
    assert.deepEqual(query.getSnapshot(), { kind: "ready", snapshot: newest });
  }
});

test("project switch cancels old updates and starts a separate project snapshot", async () => {
  const response = deferred();
  const oldQuery = new ObjectTaskQuery("old", async () => response.promise);
  let updates = 0;
  const unsubscribe = oldQuery.subscribe(() => {
    updates++;
  });
  const pending = oldQuery.refresh();
  assert.equal(updates, 1);
  oldQuery.cancel();
  const nextQuery = new ObjectTaskQuery("new", async (_method, input) => {
    assert.deepEqual(input, { projectId: "new" });
    return objectTaskSnapshot("new");
  });
  assert.equal(nextQuery.getSnapshot().kind, "loading");
  await nextQuery.refresh();
  response.resolve(objectTaskSnapshot("old"));
  await pending;
  assert.equal(updates, 1);
  unsubscribe();
  assert.deepEqual(nextQuery.getSnapshot(), {
    kind: "ready",
    snapshot: objectTaskSnapshot("new"),
  });
});

test("object task query exposes failures and can refresh to a real empty result", async () => {
  let calls = 0;
  const query = new ObjectTaskQuery("p", async () => {
    calls++;
    if (calls === 1) throw new Error("Project runtime is closed");
    if (calls === 2) return objectTaskSnapshot("wrong-project");
    return {
      planRevision: 0,
      tasks: [],
      runs: [],
      assumptions: [],
      coarseDispatchControls: [],
      dispatchControls: [],
    };
  });
  await query.refresh();
  assert.deepEqual(query.getSnapshot(), {
    kind: "error",
    message: "Project runtime is closed",
  });
  await query.refresh();
  const state = query.getSnapshot();
  assert.equal(state.kind, "error");
  if (state.kind === "error") assert.match(state.message, /不属于当前项目/);
  await query.refresh();
  assert.deepEqual(query.getSnapshot(), {
    kind: "ready",
    snapshot: {
      planRevision: 0,
      tasks: [],
      runs: [],
      assumptions: [],
      coarseDispatchControls: [],
      dispatchControls: [],
    },
  });
});
