import assert from "node:assert/strict";
import test from "node:test";
import {
  objectRecoveryOperationSchema,
  objectRecoveryRequestSchema,
  objectRecoveryViewSchema,
  type ObjectRecoveryView,
} from "../src/shared/object-recovery";
import {
  pendingView,
  recoveryReceipt,
  recoveryRequest,
  recoveryTarget,
  recoveryView,
  verifiedView,
} from "./fixtures/object-recovery";

test("recovery transport rejects unknown fields, unsafe revisions and invalid identities", () => {
  const request = recoveryRequest();
  const malformed = [
    { ...request, force: true },
    { ...request, projectId: "../other" },
    { ...request, requestId: "" },
    { ...request, target: { ...request.target, resume: true } },
    { ...request, target: { ...request.target, taskId: "a/b" } },
    { ...request, target: { ...request.target, owner: "   " } },
    { ...request, target: { ...request.target, writerGeneration: 0 } },
    { ...request, target: { ...request.target, taskRevision: 0.5 } },
    { ...request, target: { ...request.target, controlRevision: -1 } },
    {
      ...request,
      target: {
        ...request.target,
        objectRevision: Number.MAX_SAFE_INTEGER + 1,
      },
    },
  ];
  for (const input of malformed)
    assert.equal(objectRecoveryRequestSchema.safeParse(input).success, false);
  assert.deepEqual(objectRecoveryRequestSchema.parse(request), request);
});

test("durable recovery views reject forged currentness and disposition eligibility", () => {
  const damage: ((view: ObjectRecoveryView) => void)[] = [
    (view) => {
      view.target.taskId = "other";
    },
    (view) => {
      view.target.runId = "other";
    },
    (view) => {
      view.target.objectId = "other";
    },
    (view) => {
      view.target.recoveryGeneration++;
    },
    (view) => {
      view.target.controlRevision++;
    },
    (view) => {
      view.paused = true;
    },
    (view) => {
      view.reportMatchesRecords = false;
    },
    (view) => {
      view.operation!.result!.recordsCurrent = false;
    },
    (view) => {
      view.operation!.result!.writerStatus = "unconfirmed";
    },
    (view) => {
      view.operation!.result!.contentStatus = "invalid";
    },
    (view) => {
      view.operation!.result!.workspaceStatus = "drifted";
    },
    (view) => {
      view.operation!.result!.issues.push("CONTENT_HASH_MISMATCH");
    },
    (view) => {
      view.operation!.result = null;
    },
  ];
  for (const mutate of damage) {
    const view = verifiedView();
    mutate(view);
    assert.equal(objectRecoveryViewSchema.safeParse(view).success, false);
  }
  for (const view of [recoveryView(), pendingView(), verifiedView()])
    assert.deepEqual(objectRecoveryViewSchema.parse(view), view);
  const stale = verifiedView();
  stale.paused = true;
  stale.target.controlRevision++;
  stale.reportMatchesRecords = false;
  stale.canDispose = false;
  assert.deepEqual(objectRecoveryViewSchema.parse(stale), stale);
  assert.equal(
    objectRecoveryViewSchema.safeParse({ ...stale, force: true }).success,
    false,
  );
});

test("last safe generation remains readable and replayable without allowing overflow", () => {
  const request = recoveryRequest("last", {
    ...recoveryTarget(),
    recoveryGeneration: Number.MAX_SAFE_INTEGER - 1,
  });
  const receipt = recoveryReceipt(request);
  assert.deepEqual(objectRecoveryOperationSchema.parse(receipt), receipt);
  assert.equal(
    objectRecoveryViewSchema.parse(verifiedView(receipt)).target
      .recoveryGeneration,
    Number.MAX_SAFE_INTEGER,
  );
  assert.equal(
    objectRecoveryRequestSchema.safeParse({
      ...request,
      target: {
        ...request.target,
        recoveryGeneration: Number.MAX_SAFE_INTEGER,
      },
    }).success,
    false,
  );
  for (const input of [
    { ...receipt, schemaVersion: 2 },
    { ...receipt, generation: receipt.generation - 1 },
    { ...receipt, result: { ...receipt.result, resume: true } },
  ])
    assert.equal(objectRecoveryOperationSchema.safeParse(input).success, false);
});
