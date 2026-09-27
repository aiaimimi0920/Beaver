import assert from "node:assert/strict";
import test from "node:test";
import { ObjectTaskRevision } from "../src/ui/object-tasks/object-task-revision";
import { ObjectTaskRevisionHistoryStore } from "../src/ui/object-tasks/object-task-revision-history";
import {
  deferred,
  editedRevision,
  revisionReceipt as receipt,
} from "./fixtures/object-task-revisions";
import { cancelledObjectTaskSnapshot } from "./fixtures/object-tasks";

test("pre-mutation reads cannot erase a confirmed receipt and stale fresh reads are rejected", async () => {
  const old = deferred<unknown>();
  const fresh = deferred<unknown>();
  let reads = 0;
  const history = new ObjectTaskRevisionHistoryStore(
    "p",
    "medium",
    async (method, input) => {
      assert.equal(method, "objectTask.revisions");
      assert.deepEqual(input, { projectId: "p", taskId: "medium" });
      return ++reads === 1
        ? old.promise
        : reads === 2
          ? fresh.promise
          : [receipt];
    },
  );
  const oldRead = history.load();
  history.confirm(receipt);
  const freshRead = history.load();
  old.resolve([]);
  assert.equal(await oldRead, false);
  assert.deepEqual(history.getSnapshot().entries, [receipt]);
  assert.equal(history.getSnapshot().loading, true);
  assert.equal(history.getSnapshot().loaded, false);
  fresh.resolve([]);
  assert.equal(await freshRead, false);
  assert.match(history.getSnapshot().error, /缺少或改写/);
  assert.deepEqual(history.getSnapshot().entries, [receipt]);
  assert.equal(history.getSnapshot().loaded, false);
  assert.equal(await history.load(), true);
  assert.equal(history.getSnapshot().loaded, true);
  assert.equal(history.getSnapshot().error, "");
});

test("history failures retain known records and retries cannot rewrite them", async () => {
  let reads = 0;
  const history = new ObjectTaskRevisionHistoryStore(
    "p",
    "medium",
    async () => {
      reads++;
      if (reads === 2) throw new Error("History offline");
      if (reads === 3) return [{ ...receipt, reason: "Rewritten reason" }];
      return [receipt];
    },
  );
  assert.equal(await history.load(), true);
  for (const error of [/History offline/, /缺少或改写/]) {
    assert.equal(await history.load(), false);
    assert.match(history.getSnapshot().error, error);
    assert.deepEqual(history.getSnapshot().entries, [receipt]);
  }
  assert.equal(await history.load(), true);
  assert.equal(history.getSnapshot().error, "");
});

test("cleanup and immediate reload work while an obsolete generation is still pending", async () => {
  const old = deferred<unknown>();
  const current = deferred<unknown>();
  let reads = 0;
  const history = new ObjectTaskRevisionHistoryStore("p", "medium", async () =>
    ++reads === 1 ? old.promise : current.promise,
  );
  const oldRead = history.load();
  assert.equal(await history.load(), false);
  assert.equal(reads, 1);
  history.cancel();
  const currentRead = history.load();
  current.resolve([receipt]);
  assert.equal(await currentRead, true);
  old.resolve([]);
  assert.equal(await oldRead, false);
  assert.deepEqual(history.getSnapshot().entries, [receipt]);
  assert.equal(history.getSnapshot().loading, false);
  assert.equal(history.getSnapshot().loaded, true);
});

test("a slow history reload does not block mutation confirmation or workspace refresh", async () => {
  const response = deferred<unknown>();
  let refreshes = 0;
  const session = editedRevision(
    async (method) =>
      method === "objectTask.revisions" ? response.promise : receipt,
    {
      refresh: async () => {
        refreshes++;
        return true;
      },
    },
  );
  session.review();
  assert.deepEqual(await session.submit(), receipt);
  assert.equal(refreshes, 1);
  assert.equal(session.getSnapshot().refreshing, false);
  assert.equal(session.history.getSnapshot().loading, true);
  assert.deepEqual(session.history.getSnapshot().entries, [receipt]);
  session.cancel();
  response.resolve([]);
  await response.promise;
  assert.deepEqual(session.history.getSnapshot().entries, [receipt]);
});

test("cancelled task history remains readable without exposing mutation operations", async () => {
  const snapshot = cancelledObjectTaskSnapshot();
  snapshot.planRevision = 3;
  snapshot.tasks.find((task) => task.id === "medium")!.revision = 2;
  const methods: string[] = [];
  const session = new ObjectTaskRevision(
    "p",
    snapshot,
    "medium",
    async (method) => {
      methods.push(method);
      return [receipt];
    },
    async () => true,
  );
  assert.equal(session.getSnapshot().phase, "readonly");
  session.updateDefinition({ title: "Forbidden change" });
  session.updateReason("Forbidden reason");
  assert.equal(session.review(), false);
  assert.equal(await session.submit(), null);
  assert.equal(await session.history.load(), true);
  assert.deepEqual(methods, ["objectTask.revisions"]);
  assert.deepEqual(session.history.getSnapshot().entries, [receipt]);
  assert.equal(session.getSnapshot().definition.title, "Hero");
  assert.equal(session.getSnapshot().reason, "");
});
