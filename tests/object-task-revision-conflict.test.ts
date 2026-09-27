import assert from "node:assert/strict";
import test from "node:test";
import {
  objectTaskDefinition,
  revisePlannedObjectTaskSchema,
  type RevisePlannedObjectTaskRequest,
} from "../src/shared/object-task-revisions";
import type { ObjectTaskSnapshot } from "../src/shared/object-tasks";
import {
  deferred,
  editedRevision,
  revisionDefinition,
  revisionReceipt,
  revisionReason,
} from "./fixtures/object-task-revisions";
import {
  cancelledObjectTaskSnapshot,
  objectTaskSnapshot,
} from "./fixtures/object-tasks";

for (const code of [
  "OBJECT_TASK_PLAN_REVISION_CONFLICT",
  "OBJECT_TASK_REVISION_CONFLICT",
]) {
  test(`${code} preserves input and requires explicit rebase, review and a new request`, async () => {
    const latest = objectTaskSnapshot();
    latest.planRevision = 4;
    const remote = latest.tasks.find((task) => task.id === "medium")!;
    Object.assign(remote, {
      title: "Remote hero",
      prompt: "Remote goal",
      revision: 2,
    });
    const applied = {
      ...revisionReceipt,
      requestId: "revision-after-conflict",
      previousTaskRevision: 2,
      taskRevision: 3,
      previousPlanRevision: 4,
      planRevision: 5,
      before: objectTaskDefinition(remote),
    };
    const sent: RevisePlannedObjectTaskRequest[] = [];
    let ids = 0;
    const session = editedRevision(
      async (method, input) => {
        if (method === "objectTask.snapshot") return latest;
        if (method === "objectTask.revisions") return [applied];
        assert.equal(method, "objectTask.revisePlanned");
        sent.push(revisePlannedObjectTaskSchema.parse(input));
        if (sent.length === 1) throw new Error(code);
        return applied;
      },
      {
        requestId: () =>
          ++ids === 1 ? "revise-medium" : "revision-after-conflict",
      },
    );
    assert.equal(session.review(), true);
    assert.equal(await session.submit(), null);
    assert.equal(session.getSnapshot().phase, "conflict");
    assert.deepEqual(session.getSnapshot().definition, revisionDefinition);
    assert.equal(session.getSnapshot().reason, revisionReason);
    assert.equal(session.getSnapshot().snapshot.planRevision, 1);
    assert.equal(session.getSnapshot().latest?.planRevision, 4);
    assert.equal(await session.submit(), null);
    assert.equal(session.review(), false);
    assert.equal(session.rebase(), true);
    assert.equal(session.getSnapshot().phase, "editing");
    assert.equal(session.getSnapshot().snapshot.planRevision, 4);
    assert.deepEqual(session.getSnapshot().definition, revisionDefinition);
    assert.equal(session.getSnapshot().reason, revisionReason);
    assert.equal(await session.submit(), null);
    assert.equal(sent.length, 1);
    assert.equal(session.review(), true);
    assert.deepEqual(await session.submit(), applied);
    const rebasedRequest = sent[1];
    assert.ok(rebasedRequest);
    assert.equal(rebasedRequest.requestId, "revision-after-conflict");
    assert.equal(rebasedRequest.expectedTaskRevision, 2);
    assert.equal(rebasedRequest.expectedPlanRevision, 4);
    assert.deepEqual(rebasedRequest.definition, revisionDefinition);
    assert.deepEqual(
      session.getSnapshot().receipt?.before,
      objectTaskDefinition(remote),
    );
  });
}

test("rebase refuses a cancelled or removed target, changed responsibility and a resolved run", async () => {
  const cases: Array<{
    change: (snapshot: ObjectTaskSnapshot) => void;
    error: RegExp;
  }> = [
    {
      change: (snapshot) =>
        Object.assign(snapshot, cancelledObjectTaskSnapshot()),
      error: /未撤销/,
    },
    {
      change: (snapshot) => {
        snapshot.tasks = snapshot.tasks.filter(
          (task) => !["medium", "fine"].includes(task.id),
        );
        snapshot.runs = snapshot.runs.filter((run) => run.id !== "run-hero");
      },
      error: /找不到/,
    },
    {
      change: (snapshot) => {
        snapshot.tasks.find((task) => task.id === "medium")!.parentTaskId =
          null;
      },
      error: /身份已改变/,
    },
    {
      change: (snapshot) => {
        snapshot.runs.find((run) => run.id === "run-hero")!.baselineVersionId =
          "resolved-baseline";
      },
      error: /执行基准/,
    },
  ];
  for (const { change, error } of cases) {
    const latest = objectTaskSnapshot();
    latest.planRevision = 2;
    change(latest);
    const session = editedRevision(async (method) => {
      if (method === "objectTask.snapshot") return latest;
      throw new Error("OBJECT_TASK_PLAN_REVISION_CONFLICT");
    });
    session.review();
    await session.submit();
    assert.ok(session.getSnapshot().latest, session.getSnapshot().latestError);
    assert.equal(session.rebase(), false);
    assert.match(session.getSnapshot().latestError, error);
    assert.equal(session.getSnapshot().phase, "conflict");
    assert.equal(session.getSnapshot().snapshot.planRevision, 1);
    assert.deepEqual(session.getSnapshot().definition, revisionDefinition);
  }
});

test("rebase retains a cancelled dependency so the owner can remove it before review", async () => {
  const latest = objectTaskSnapshot();
  latest.planRevision = 2;
  latest.tasks.find((task) => task.id === "independent")!.status = "cancelled";
  latest.runs.find((run) => run.id === "run-independent")!.status = "cancelled";
  const session = editedRevision(async (method) => {
    if (method === "objectTask.snapshot") return latest;
    throw new Error("OBJECT_TASK_PLAN_REVISION_CONFLICT");
  });
  session.review();
  await session.submit();
  assert.equal(session.rebase(), true);
  assert.deepEqual(session.getSnapshot().definition.dependsOn, ["independent"]);
  assert.equal(session.review(), false);
  assert.match(session.getSnapshot().error, /已撤销/);
  session.updateDefinition({ dependsOn: [] });
  assert.equal(session.review(), true);
});

test("latest-plan retries reject wrong-project and stale snapshots without losing input", async () => {
  const latest = objectTaskSnapshot();
  latest.planRevision = 2;
  const regressed = objectTaskSnapshot();
  regressed.planRevision = 0;
  const responses = [
    new Error("Snapshot unavailable"),
    objectTaskSnapshot(),
    objectTaskSnapshot("q"),
    regressed,
    latest,
    new Error("Connection lost"),
  ];
  let writes = 0;
  const session = editedRevision(async (method) => {
    if (method !== "objectTask.snapshot") {
      writes++;
      throw new Error("OBJECT_TASK_REVISION_CONFLICT");
    }
    const response = responses.shift();
    if (response instanceof Error) throw response;
    return response;
  });
  session.review();
  await session.submit();
  assert.match(session.getSnapshot().latestError, /Snapshot unavailable/);
  for (const expected of [
    /未包含冲突后的版本/,
    /不属于当前项目/,
    /未包含冲突后的版本/,
  ]) {
    assert.equal(await session.loadLatest(), false);
    assert.equal(session.getSnapshot().latest, null);
    assert.match(session.getSnapshot().latestError, expected);
  }
  assert.equal(await session.loadLatest(), true);
  assert.equal(session.getSnapshot().latest?.planRevision, 2);
  assert.equal(await session.loadLatest(), false);
  assert.equal(session.getSnapshot().latest, null);
  assert.equal(session.rebase(), false);
  assert.deepEqual(session.getSnapshot().definition, revisionDefinition);
  assert.equal(session.getSnapshot().reason, revisionReason);
  assert.equal(writes, 1);
});

test("closing during conflict recovery drops the late latest-plan response", async () => {
  const requested = deferred<void>();
  const response = deferred<unknown>();
  const session = editedRevision(async (method) => {
    if (method === "objectTask.snapshot") {
      requested.resolve();
      return response.promise;
    }
    throw new Error("OBJECT_TASK_REVISION_CONFLICT");
  });
  session.review();
  const pending = session.submit();
  await requested.promise;
  session.cancel();
  const latest = objectTaskSnapshot();
  latest.planRevision = 2;
  response.resolve(latest);
  assert.equal(await pending, null);
  assert.equal(session.getSnapshot().latest, null);
  assert.equal(session.getSnapshot().latestLoading, false);
  assert.equal(session.rebase(), false);
});
