import assert from "node:assert/strict";
import test from "node:test";
import {
  revisePlannedObjectTaskSchema,
  type RevisePlannedObjectTaskRequest,
} from "../src/shared/object-task-revisions";
import { objectTaskPlanSchema } from "../src/shared/object-tasks";
import { ObjectTaskRevision } from "../src/ui/object-tasks/object-task-revision";
import { ObjectTaskWorkspace } from "../src/ui/object-tasks/object-task-workspace";
import {
  committedRevisionSnapshot,
  deferred,
  editedRevision,
  revisionDefinition,
  revisionReceipt as receipt,
  revisionReason,
  revisionRequest,
} from "./fixtures/object-task-revisions";
import { objectTaskSnapshot } from "./fixtures/object-tasks";

test("definition changes require a separate review and confirmation, and no-op edits do not submit", async () => {
  const calls: string[] = [];
  let refreshes = 0;
  const original = objectTaskSnapshot();
  const session = editedRevision(
    async (method, input) => {
      calls.push(method);
      if (method === "objectTask.revisions") return [receipt];
      assert.deepEqual(input, revisionRequest);
      return receipt;
    },
    {
      snapshot: original,
      refresh: async () => {
        refreshes++;
        return true;
      },
    },
  );
  assert.equal(await session.submit(), null);
  assert.deepEqual(calls, []);
  assert.equal(session.review(), true);
  assert.deepEqual(calls, []);
  original.tasks.find((task) => task.id === "medium")!.prompt =
    "External mutation";
  session.edit();
  session.updateDefinition(receipt.before);
  assert.equal(session.review(), false);
  assert.match(session.getSnapshot().error, /没有变化/);
  session.updateDefinition(revisionDefinition);
  assert.equal(session.review(), true);
  assert.deepEqual(await session.submit(), receipt);
  assert.equal(session.getSnapshot().phase, "succeeded");
  assert.equal(refreshes, 1);
  assert.deepEqual(calls, ["objectTask.revisePlanned", "objectTask.revisions"]);
  assert.deepEqual(session.history.getSnapshot().entries, [receipt]);
});

test("ambiguous failures retry the frozen complete payload and suppress duplicate submissions", async () => {
  const first = deferred<unknown>();
  const sent: RevisePlannedObjectTaskRequest[] = [];
  let ids = 0;
  const session = editedRevision(
    async (method, input) => {
      if (method === "objectTask.revisions") return [receipt];
      sent.push(revisePlannedObjectTaskSchema.parse(input));
      assert.ok(input && typeof input === "object");
      Object.assign(input, {
        definition: { ...revisionDefinition, title: "Transport mutation" },
      });
      return sent.length === 1 ? first.promise : receipt;
    },
    {
      requestId: () => {
        ids++;
        return "revise-medium";
      },
    },
  );
  assert.equal(session.review(), true);
  const pending = session.submit();
  assert.equal(await session.submit(), null);
  session.updateDefinition({ title: "Unconfirmed edit" });
  first.reject(new Error("Connection lost after commit"));
  assert.equal(await pending, null);
  assert.equal(session.getSnapshot().phase, "failed");
  session.updateReason("Another reason");
  session.edit();
  assert.equal(session.getSnapshot().phase, "failed");
  assert.equal(session.getSnapshot().reason, revisionReason);
  assert.deepEqual(await session.submit(), receipt);
  assert.equal(ids, 1);
  assert.deepEqual(sent, [revisionRequest, revisionRequest]);
});

test("a malformed receipt remains retryable without claiming success or refreshing the workspace", async () => {
  let writes = 0;
  let refreshes = 0;
  const session = editedRevision(
    async (method) => {
      if (method === "objectTask.revisions") return [receipt];
      writes++;
      return writes === 1
        ? { ...receipt, after: { ...receipt.after, prompt: "Wrong result" } }
        : receipt;
    },
    {
      refresh: async () => {
        refreshes++;
        return true;
      },
    },
  );
  session.review();
  assert.equal(await session.submit(), null);
  assert.equal(session.getSnapshot().phase, "failed");
  assert.equal(session.getSnapshot().receipt, null);
  assert.equal(refreshes, 0);
  assert.deepEqual(session.history.getSnapshot().entries, []);
  assert.deepEqual(await session.submit(), receipt);
  assert.equal(refreshes, 1);
});

test("a confirmed mutation survives refresh and history failures and permits refresh-only recovery", async () => {
  let writes = 0;
  let refreshes = 0;
  const session = editedRevision(
    async (method) => {
      if (method === "objectTask.revisions")
        throw new Error("History unavailable");
      writes++;
      return receipt;
    },
    { refresh: async () => ++refreshes > 1 },
  );
  session.review();
  assert.deepEqual(await session.submit(), receipt);
  assert.equal(session.getSnapshot().phase, "succeeded");
  assert.match(session.getSnapshot().error, /回执已确认，但刷新失败/);
  assert.match(session.history.getSnapshot().error, /History unavailable/);
  assert.deepEqual(session.history.getSnapshot().entries, [receipt]);
  assert.deepEqual(await session.submit(), receipt);
  assert.equal(await session.refresh(), true);
  assert.equal(session.getSnapshot().error, "");
  assert.equal(writes, 1);
  assert.equal(refreshes, 2);
});

test("closing an old project session drops its late mutation response and cannot refresh the new project", async () => {
  const response = deferred<unknown>();
  let refreshes = 0;
  const session = editedRevision(async () => response.promise, {
    refresh: async () => {
      refreshes++;
      return true;
    },
  });
  session.review();
  const pending = session.submit();
  session.cancel();
  const next = new ObjectTaskRevision(
    "q",
    objectTaskSnapshot("q"),
    "medium",
    async () => [],
    async () => true,
  );
  const nextState = next.getSnapshot();
  response.resolve(receipt);
  assert.equal(await pending, null);
  assert.equal(session.getSnapshot().receipt, null);
  assert.deepEqual(session.history.getSnapshot().entries, []);
  assert.equal(refreshes, 0);
  assert.equal(next.getSnapshot(), nextState);
});

test("revision refresh preserves dirty planning content and exposes the changed plan version", async () => {
  let snapshot = objectTaskSnapshot();
  const api = async (method: string) => {
    if (method === "objectTask.snapshot") return snapshot;
    if (method === "objectTask.getDraft") return null;
    if (method === "objectTask.revisions") return [receipt];
    if (method === "objectTask.revisePlanned") {
      snapshot = committedRevisionSnapshot();
      return receipt;
    }
    throw new Error(`Unexpected method: ${method}`);
  };
  const workspace = new ObjectTaskWorkspace("p", api);
  assert.equal(await workspace.refresh(), true);
  const local = objectTaskPlanSchema.parse({
    assumptions: [
      {
        id: "local",
        statement: "Keep local intent",
        basis: "Unsaved user input",
      },
    ],
  });
  workspace.updatePlan(local);
  const session = editedRevision(api, { refresh: workspace.refresh });
  session.review();
  assert.deepEqual(await session.submit(), receipt);
  const state = workspace.getSnapshot();
  assert.ok(state.kind === "ready");
  assert.equal(state.snapshot.planRevision, 2);
  assert.equal(state.conflict, "plan");
  assert.equal(state.dirty, true);
  assert.deepEqual(state.plan, local);
});
