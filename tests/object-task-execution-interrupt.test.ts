import assert from "node:assert/strict";
import test from "node:test";
import {
  objectAttemptInterruptSchema,
  type ObjectAttemptInterrupt,
} from "../src/shared/object-attempts";
import {
  deferred,
  execution,
  interruptReceipt,
  session,
} from "./fixtures/object-attempts";

test("lost-response retries retain the exact reviewed request despite new queries and API mutation", async () => {
  const lost = deferred<unknown>();
  const requests: ObjectAttemptInterrupt[] = [];
  let active = execution();
  let ids = 0;
  let refreshes = 0;
  const controller = session(
    async (method, input) => {
      if (method === "objectTask.attempts") return [active];
      assert.equal(method, "objectTask.interrupt");
      requests.push(objectAttemptInterruptSchema.parse(input));
      Object.assign(input as object, { requestId: "mutated" });
      return requests.length === 1
        ? lost.promise
        : interruptReceipt(requests[0]);
    },
    async () => {
      refreshes++;
      return true;
    },
    "p",
    () => `stop-${++ids}`,
  );
  await controller.refresh();
  const first = controller.interrupt("attempt-1");
  assert.equal(await controller.interrupt("attempt-1"), null);
  assert.equal(await controller.refresh(), false);
  lost.reject(new Error("Response lost after commit"));
  assert.equal(await first, null);
  assert.equal(controller.getSnapshot().retryAvailable, true);
  active = execution();
  active.attempt.target.threadId = "new-thread";
  active.attempt.target.turnId = "new-turn";
  active.attempt.taskRevision = 7;
  await controller.refresh();
  const receipt = await controller.interrupt();
  assert.ok(receipt);
  assert.deepEqual(requests, [
    {
      projectId: "p",
      requestId: "stop-1",
      target: execution().attempt.target,
      expectedTaskRevision: 2,
    },
    {
      projectId: "p",
      requestId: "stop-1",
      target: execution().attempt.target,
      expectedTaskRevision: 2,
    },
  ]);
  assert.equal(ids, 1);
  assert.equal(refreshes, 1);
  assert.deepEqual(await controller.interrupt(), receipt);
  assert.equal(requests.length, 2);
});

test("explicit stale-target rejections require a fresh query and a new confirmation", async () => {
  for (const failure of [
    new Error("OBJECT_ATTEMPT_STALE_TARGET"),
    "OBJECT_ATTEMPT_REVISION_CONFLICT",
  ]) {
    let active = execution();
    let ids = 0;
    const requests: ObjectAttemptInterrupt[] = [];
    const controller = session(
      async (method, input) => {
        if (method === "objectTask.attempts") return [active];
        requests.push(objectAttemptInterruptSchema.parse(input));
        if (requests.length === 1) throw failure;
        return interruptReceipt(input);
      },
      async () => true,
      "p",
      () => `stop-${++ids}`,
    );
    await controller.refresh();
    assert.equal(await controller.interrupt("attempt-1"), null);
    assert.equal(controller.getSnapshot().retryAvailable, false);
    assert.match(controller.getSnapshot().error, /刷新执行记录后重新确认/);
    assert.equal(await controller.interrupt("attempt-1"), null);
    assert.equal(requests.length, 1);
    active = execution();
    active.attempt.target.threadId = "thread-1";
    active.attempt.target.turnId = "turn-1";
    active.attempt.taskRevision = 3;
    await controller.refresh();
    assert.ok(await controller.interrupt("attempt-1"));
    const retry = requests[1];
    assert.ok(retry);
    assert.equal(retry.requestId, "stop-2");
    assert.equal(retry.target.turnId, "turn-1");
    assert.equal(retry.expectedTaskRevision, 3);
  }
});

test("mismatched receipts and unfinished results cannot confirm an interruption", async () => {
  let refreshes = 0;
  const mutations = [
    (input: unknown) => {
      const receipt = interruptReceipt(input);
      receipt.request.requestId = "other";
      return receipt;
    },
    (input: unknown) => {
      const receipt = interruptReceipt(input);
      receipt.request.expectedTaskRevision++;
      return receipt;
    },
    (input: unknown) => {
      const receipt = interruptReceipt(input);
      receipt.result.target.turnId = "other";
      return receipt;
    },
    (input: unknown) => {
      const receipt = interruptReceipt(input);
      receipt.result.projectId = "other";
      return receipt;
    },
    (input: unknown) => {
      const receipt = interruptReceipt(input);
      receipt.result.state = "running";
      receipt.result.outputCaptured = false;
      return receipt;
    },
    (input: unknown) => ({ ...interruptReceipt(input), accepted: true }),
  ];
  for (const mutate of mutations) {
    const controller = session(
      async (method, input) =>
        method === "objectTask.attempts" ? [execution()] : mutate(input),
      async () => {
        refreshes++;
        return true;
      },
    );
    await controller.refresh();
    assert.equal(await controller.interrupt("attempt-1"), null);
    assert.equal(controller.getSnapshot().phase, "failed");
    assert.equal(controller.getSnapshot().receipt, null);
    assert.equal(controller.getSnapshot().retryAvailable, true);
  }
  assert.equal(refreshes, 0);
});

test("late interruption displays the actual frozen terminal result", async () => {
  for (const state of ["awaitingGate", "failed", "interrupted"] as const) {
    const controller = session(async (method, input) =>
      method === "objectTask.attempts"
        ? [execution()]
        : interruptReceipt(input, state),
    );
    await controller.refresh();
    const receipt = await controller.interrupt("attempt-1");
    assert.equal(receipt?.result.state, state);
    assert.equal(controller.getSnapshot().phase, "succeeded");
    assert.equal(controller.getSnapshot().executions[0]?.attempt.state, state);
    assert.equal(
      controller.getSnapshot().executions[0]?.availability,
      "finished",
    );
  }
});

test("a confirmed receipt survives refresh failures without resending interruption", async () => {
  let writes = 0;
  let refreshes = 0;
  const controller = session(
    async (method, input) => {
      if (method === "objectTask.attempts") return [execution()];
      writes++;
      return interruptReceipt(input);
    },
    async () => {
      refreshes++;
      if (refreshes === 1) return false;
      if (refreshes === 2) throw new Error("Snapshot unavailable");
      return true;
    },
  );
  await controller.refresh();
  const receipt = await controller.interrupt("attempt-1");
  assert.ok(receipt);
  assert.equal(controller.getSnapshot().phase, "succeeded");
  assert.match(controller.getSnapshot().error, /回执已确认.*刷新失败/);
  assert.deepEqual(await controller.interrupt(), receipt);
  assert.equal(await controller.refresh(), false);
  assert.deepEqual(controller.getSnapshot().receipt, receipt);
  assert.equal(await controller.refresh(), true);
  assert.equal(controller.getSnapshot().error, "");
  assert.equal(writes, 1);
  assert.equal(refreshes, 3);
});
