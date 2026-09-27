import assert from "node:assert/strict";
import test from "node:test";
import {
  deferred,
  session as executionSession,
} from "./fixtures/object-attempts";
import {
  pendingView,
  recoveryReceipt,
  recoveryRequest,
  recoverySession,
  recoveryTarget,
  recoveryView,
  verifiedView,
} from "./fixtures/object-recovery";

test("closing the execution dialog invalidates its recovery query during a project switch", async () => {
  const response = deferred<unknown>();
  const previous = executionSession(async () => response.promise);
  const query = previous.recovery.refresh();
  previous.cancel();
  const stopped = previous.recovery.getSnapshot();
  const calls: unknown[] = [];
  const next = executionSession(
    async (method, input) => {
      calls.push({ method, input });
      return null;
    },
    async () => true,
    "other",
  );
  await next.recovery.refresh();
  response.resolve(pendingView());
  assert.equal(await query, false);
  assert.equal(previous.recovery.getSnapshot(), stopped);
  assert.equal(previous.recovery.getSnapshot().retryAvailable, false);
  assert.equal(next.recovery.getSnapshot().phase, "ready");
  assert.deepEqual(calls, [
    {
      method: "objectTask.recovery",
      input: { projectId: "other", taskId: "medium" },
    },
  ]);
});

test("closing while verifying ignores late outcomes and can still query the durable result", async () => {
  for (const failed of [false, true]) {
    const response = deferred<unknown>();
    let view = recoveryView();
    let writes = 0;
    const session = recoverySession(async (method, input) => {
      if (method === "objectTask.recovery") return view;
      writes++;
      view = verifiedView(recoveryReceipt(input));
      return response.promise;
    });
    await session.refresh();
    const work = session.verify();
    assert.equal(await session.verify(), null);
    assert.equal(await session.refresh(), false);
    session.cancel();
    const stopped = session.getSnapshot();
    if (failed) response.reject(new Error("late network error"));
    else response.resolve(view.operation);
    assert.equal(await work, null);
    assert.equal(session.getSnapshot(), stopped);
    assert.equal(session.getSnapshot().retryAvailable, true);
    await session.refresh();
    assert.equal(session.getSnapshot().retryAvailable, false);
    assert.deepEqual(session.getSnapshot().receipt, view.operation);
    assert.equal(writes, 1);
  }
});

test("synchronous close listeners prevent query and verification dispatch", async () => {
  const calls: string[] = [];
  const session = recoverySession(async (method) => {
    calls.push(method);
    return recoveryView();
  });
  let unsubscribe = session.subscribe(() => {
    if (session.getSnapshot().refreshing) session.cancel();
  });
  assert.equal(await session.refresh(), false);
  assert.deepEqual(calls, []);
  unsubscribe();
  await session.refresh();
  unsubscribe = session.subscribe(() => {
    if (session.getSnapshot().phase === "submitting") session.cancel();
  });
  assert.equal(await session.verify(), null);
  assert.deepEqual(calls, ["objectTask.recovery"]);
  assert.equal(session.getSnapshot().retryAvailable, true);
  unsubscribe();
});

test("closing on acknowledgement retains the report but suppresses the follow-up query", async () => {
  const calls: string[] = [];
  const session = recoverySession(async (method, input) => {
    calls.push(method);
    return method === "objectTask.recovery"
      ? recoveryView()
      : recoveryReceipt(input);
  });
  await session.refresh();
  const unsubscribe = session.subscribe(() => {
    if (session.getSnapshot().receipt) session.cancel();
  });
  assert.equal(await session.verify(), null);
  assert.equal(session.getSnapshot().receipt?.request.requestId, "verify-1");
  assert.deepEqual(calls, ["objectTask.recovery", "objectTask.verifyRecovery"]);
  unsubscribe();
});

test("generation exhaustion disables fresh verification while the final pending request can retry", async () => {
  const request = recoveryRequest("last", {
    ...recoveryTarget(),
    recoveryGeneration: Number.MAX_SAFE_INTEGER - 1,
  });
  let view = verifiedView(recoveryReceipt(request));
  let writes = 0;
  const session = recoverySession(
    async (method, input) => {
      if (method === "objectTask.recovery") return view;
      writes++;
      const receipt = recoveryReceipt(input);
      view = verifiedView(receipt);
      return receipt;
    },
    () => {
      throw new Error("generation is exhausted");
    },
  );
  await session.refresh();
  assert.equal(await session.verify(), null);
  assert.equal(writes, 0);
  view = pendingView(request);
  await session.refresh();
  assert.equal((await session.verify())?.generation, Number.MAX_SAFE_INTEGER);
  assert.equal(writes, 1);
});
