import assert from "node:assert/strict";
import test from "node:test";
import {
  pendingView,
  recoveryReceipt,
  recoverySession,
  recoveryView,
  verifiedView,
} from "./fixtures/object-recovery";

test("an unprepared task is query-only and cannot start verification", async () => {
  const calls: unknown[] = [];
  const session = recoverySession(async (method, input) => {
    calls.push({ method, input });
    return null;
  });
  assert.equal(await session.refresh(), true);
  assert.equal(await session.verify(), null);
  assert.equal(session.getSnapshot().view, null);
  assert.equal(session.getSnapshot().phase, "ready");
  assert.deepEqual(calls, [
    {
      method: "objectTask.recovery",
      input: { projectId: "p", taskId: "medium" },
    },
  ]);
});

test("reopening a pending operation adopts its exact saved request only on explicit retry", async () => {
  let view = pendingView();
  const saved = structuredClone(view.operation!.request);
  const calls: { method: string; input: unknown }[] = [];
  const session = recoverySession(
    async (method, input) => {
      calls.push({ method, input });
      if (method === "objectTask.recovery") return structuredClone(view);
      const receipt = recoveryReceipt(input);
      view = verifiedView(receipt);
      return receipt;
    },
    () => {
      throw new Error("reopening must keep the saved request ID");
    },
  );
  assert.equal(await session.refresh(), true);
  assert.equal(session.getSnapshot().retryAvailable, true);
  assert.equal(calls.length, 1);
  const receipt = await session.verify();
  assert.deepEqual(receipt?.request, saved);
  assert.deepEqual(
    calls.map((call) => call.method),
    ["objectTask.recovery", "objectTask.verifyRecovery", "objectTask.recovery"],
  );
  assert.deepEqual(calls[1]?.input, saved);
  assert.equal(session.getSnapshot().retryAvailable, false);
  assert.deepEqual(session.getSnapshot().receipt, receipt);
});

test("queries reject records from another project, task, object or run without a write", async () => {
  const views = [
    {
      ...recoveryView(),
      target: { ...recoveryView().target, taskId: "other" },
    },
    {
      ...recoveryView(),
      target: { ...recoveryView().target, objectId: "other" },
    },
    { ...recoveryView(), target: { ...recoveryView().target, runId: "other" } },
    verifiedView(
      recoveryReceipt({
        ...verifiedView().operation!.request,
        projectId: "other",
      }),
    ),
    {
      ...verifiedView(),
      operation: { ...verifiedView().operation, schemaVersion: 2 },
    },
    {
      ...verifiedView(),
      target: { ...verifiedView().target, recoveryGeneration: 3 },
    },
  ];
  for (const view of views) {
    const methods: string[] = [];
    const session = recoverySession(async (method) => {
      methods.push(method);
      return view;
    });
    assert.equal(await session.refresh(), false);
    assert.equal(session.getSnapshot().phase, "failed");
    assert.equal(session.getSnapshot().view, null);
    assert.equal(await session.verify(), null);
    assert.deepEqual(methods, ["objectTask.recovery"]);
  }
});

test("a completed original request can be confirmed by query after its response was lost", async () => {
  let view = recoveryView();
  let writes = 0;
  let ids = 0;
  const session = recoverySession(
    async (method, input) => {
      if (method === "objectTask.recovery") return view;
      const receipt = recoveryReceipt(input);
      view = verifiedView(receipt);
      if (++writes === 1) throw new Error("response lost after commit");
      return receipt;
    },
    () => `verify-${++ids}`,
  );
  await session.refresh();
  assert.equal(await session.verify(), null);
  assert.equal(session.getSnapshot().retryAvailable, true);
  assert.equal(await session.refresh(), true);
  assert.equal(session.getSnapshot().retryAvailable, false);
  assert.equal(session.getSnapshot().receipt?.request.requestId, "verify-1");
  assert.equal(writes, 1);
  const next = await session.verify();
  assert.equal(next?.request.requestId, "verify-2");
  assert.equal(next?.request.target.recoveryGeneration, 1);
});

test("JSON property ordering does not break query identity or exact receipt confirmation", async () => {
  const reorder = (value: object) =>
    Object.fromEntries(Object.entries(value).reverse());
  let view = recoveryView();
  const session = recoverySession(async (method, input) => {
    if (method === "objectTask.recovery")
      return { ...reorder(view), target: reorder(view.target) };
    const receipt = recoveryReceipt(input);
    view = verifiedView(receipt);
    return {
      ...reorder(receipt),
      request: {
        ...reorder(receipt.request),
        target: reorder(receipt.request.target),
      },
    };
  });
  await session.refresh();
  assert.ok(await session.verify());
  assert.equal(session.getSnapshot().error, "");
});
