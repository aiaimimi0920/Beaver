import assert from "node:assert/strict";
import test from "node:test";
import type {
  ObjectRecoveryOperation,
  ObjectRecoveryRequest,
} from "../src/shared/object-recovery";
import {
  pendingView,
  recoveryReceipt,
  recoverySession,
  recoveryView,
  verifiedView,
} from "./fixtures/object-recovery";

test("unknown outcomes keep the original request across record changes and transport mutation", async () => {
  let view = recoveryView();
  const sent: ObjectRecoveryRequest[] = [];
  const session = recoverySession(async (method, input) => {
    if (method === "objectTask.recovery") return view;
    const receipt = recoveryReceipt(input);
    sent.push(structuredClone(receipt.request));
    if (sent.length === 1) {
      assert.ok(input && typeof input === "object" && "target" in input);
      input.target = { ...receipt.request.target, controlRevision: 99 };
      throw new Error("connection lost");
    }
    view = verifiedView(receipt);
    view.target.controlRevision = 1;
    view.reportMatchesRecords = false;
    view.canDispose = false;
    return receipt;
  });
  await session.refresh();
  await session.verify();
  view.target.controlRevision = 1;
  await session.refresh();
  assert.equal(session.getSnapshot().retryAvailable, true);
  const receipt = await session.verify();
  assert.deepEqual(sent[1], sent[0]);
  assert.equal(receipt?.request.target.controlRevision, 0);
  assert.equal(session.getSnapshot().view?.target.controlRevision, 1);
  assert.equal(session.getSnapshot().view?.reportMatchesRecords, false);
  assert.deepEqual(session.getSnapshot().receipt, receipt);
});

test("only exact precondition rejections discard an unaccepted request", async () => {
  const rejected = [
    "OBJECT_RECOVERY_STALE_TARGET",
    "OBJECT_RECOVERY_NOT_REQUIRED",
    "OBJECT_RECOVERY_PENDING",
    "OBJECT_RECOVERY_REQUEST_CONFLICT",
  ];
  for (const message of [
    ...rejected,
    "OBJECT_RECOVERY_BUSY",
    "wrapped OBJECT_RECOVERY_STALE_TARGET",
    "toString",
  ]) {
    let view = recoveryView();
    const sent: ObjectRecoveryRequest[] = [];
    let ids = 0;
    const session = recoverySession(
      async (method, input) => {
        if (method === "objectTask.recovery") return view;
        const receipt = recoveryReceipt(input);
        sent.push(receipt.request);
        if (sent.length === 1) throw new Error(message);
        view = verifiedView(receipt);
        return receipt;
      },
      () => `verify-${++ids}`,
    );
    await session.refresh();
    await session.verify();
    const discarded = rejected.includes(message);
    assert.equal(session.getSnapshot().retryAvailable, !discarded, message);
    if (discarded) {
      assert.equal(await session.verify(), null);
      view.target.controlRevision++;
      await session.refresh();
    }
    assert.ok(await session.verify());
    assert.equal(sent[1]?.requestId, discarded ? "verify-2" : "verify-1");
    assert.equal(sent[1]?.target.controlRevision, discarded ? 1 : 0);
  }
});

test("mismatched or unfinished receipts cannot replace an uncertain original request", async () => {
  const malformed: ((receipt: ObjectRecoveryOperation) => unknown)[] = [
    (receipt) => ({ ...receipt, schemaVersion: 2 }),
    (receipt) => ({ ...receipt, generation: receipt.generation + 1 }),
    (receipt) => ({ ...receipt, result: null }),
    (receipt) => ({
      ...receipt,
      request: { ...receipt.request, projectId: "other" },
    }),
    (receipt) => ({
      ...receipt,
      request: { ...receipt.request, requestId: "other" },
    }),
    (receipt) => ({
      ...receipt,
      request: {
        ...receipt.request,
        target: { ...receipt.request.target, owner: "other" },
      },
    }),
    (receipt) => ({
      ...receipt,
      request: {
        ...receipt.request,
        target: { ...receipt.request.target, runId: "other" },
      },
    }),
    (receipt) => ({
      ...receipt,
      request: {
        ...receipt.request,
        target: { ...receipt.request.target, controlRevision: 1 },
      },
    }),
  ];
  for (const damage of malformed) {
    let view = recoveryView();
    const sent: ObjectRecoveryRequest[] = [];
    const session = recoverySession(async (method, input) => {
      if (method === "objectTask.recovery") return view;
      const receipt = recoveryReceipt(input);
      sent.push(receipt.request);
      if (sent.length === 1) return damage(receipt);
      view = verifiedView(receipt);
      return receipt;
    });
    await session.refresh();
    assert.equal(await session.verify(), null);
    assert.equal(session.getSnapshot().receipt, null);
    assert.equal(session.getSnapshot().retryAvailable, true);
    assert.ok(await session.verify());
    assert.deepEqual(sent[1], sent[0]);
  }
});

test("refresh failure preserves an acknowledged report and refresh never repeats verification", async () => {
  let view = recoveryView();
  let failQuery = false;
  let writes = 0;
  const session = recoverySession(async (method, input) => {
    if (method === "objectTask.recovery") {
      if (failQuery) throw new Error("query offline");
      return view;
    }
    writes++;
    const receipt = recoveryReceipt(input);
    view = verifiedView(receipt);
    failQuery = true;
    return receipt;
  });
  await session.refresh();
  const receipt = await session.verify();
  assert.ok(receipt);
  assert.equal(session.getSnapshot().phase, "failed");
  assert.equal(session.getSnapshot().view, null);
  assert.equal(session.getSnapshot().retryAvailable, false);
  assert.deepEqual(session.getSnapshot().receipt, receipt);
  assert.match(session.getSnapshot().error, /核验回执已保留/);
  failQuery = false;
  view.paused = true;
  view.target.controlRevision++;
  view.reportMatchesRecords = false;
  view.canDispose = false;
  assert.equal(await session.refresh(), true);
  assert.deepEqual(session.getSnapshot().receipt, receipt);
  assert.equal(session.getSnapshot().view?.paused, true);
  assert.equal(writes, 1);
});

test("a conflicting query cannot confirm or replace a locally frozen request", async () => {
  let view = recoveryView();
  let original: ObjectRecoveryRequest | undefined;
  const session = recoverySession(async (method, input) => {
    if (method === "objectTask.recovery") return view;
    original = recoveryReceipt(input).request;
    throw new Error("response lost");
  });
  await session.refresh();
  await session.verify();
  assert.ok(original);
  view = verifiedView(
    recoveryReceipt({
      ...original,
      target: { ...original.target, claimToken: "other" },
    }),
  );
  assert.equal(await session.refresh(), false);
  assert.match(session.getSnapshot().error, /待确认请求不一致/);
  assert.equal(session.getSnapshot().retryAvailable, true);
  view = pendingView({ ...original, requestId: "another-client" });
  assert.equal(await session.refresh(), true);
  await session.verify();
  assert.equal(original.requestId, "verify-1");
  assert.equal(original.target.claimToken, "claim-1");
});
