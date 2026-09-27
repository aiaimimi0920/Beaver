import assert from "node:assert/strict";
import test from "node:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import {
  objectRecoveryDispositionOperationSchema,
  objectRecoveryDispositionRequestSchema,
} from "../src/shared/object-recovery";
import { ObjectTaskDispositionPanel } from "../src/ui/object-tasks/ObjectTaskDispositionPanel";
import { recoverySession, verifiedView } from "./fixtures/object-recovery";

test("removal requires confirmation and retries the exact request after a lost response", async () => {
  const view = verifiedView();
  const sent: unknown[] = [];
  const session = recoverySession(async (method, input) => {
    if (method === "objectTask.recovery") return view;
    assert.equal(method, "objectTask.disposeRecovery");
    sent.push(structuredClone(input));
    const request = objectRecoveryDispositionRequestSchema.parse(input);
    const receipt = objectRecoveryDispositionOperationSchema.parse({
      schemaVersion: 1,
      request,
      result: {
        status: "cancelledAndWorkspaceRemoved",
        report: view.operation!.result,
        removed: {
          workspace: ".beaver/workspaces/run-hero",
          attemptId: "attempt-1",
          fineTaskId: "fine-1",
          outputFileCount: 1,
        },
        taskRevision: request.target.taskRevision + 1,
        runRevision: request.target.runRevision + 1,
        planRevision: 4,
      },
    });
    view.disposition = receipt;
    view.canDispose = false;
    view.reportMatchesRecords = false;
    if (sent.length === 1) throw new Error("response lost");
    return receipt;
  });
  const render = () =>
    renderToStaticMarkup(
      createElement(ObjectTaskDispositionPanel, {
        session,
        state: session.getSnapshot(),
      }),
    );
  await session.refresh();
  assert.equal(await session.dispose(), null);
  assert.equal(sent.length, 0);
  assert.equal(session.prepareCancelAndRemoveWorkspace(), true);
  assert.match(render(), /确认取消并删除工作区/);
  assert.equal(await session.dispose(), null);
  assert.match(render(), /重试处置/);
  assert.equal(
    (await session.dispose())?.result?.status,
    "cancelledAndWorkspaceRemoved",
  );
  assert.deepEqual(sent[0], sent[1]);
  assert.match(render(), /已删除工作目录/);
  assert.doesNotMatch(render(), /重试处置|确认取消并删除工作区/);
  assert.equal(session.canVerify(), false);
  assert.equal(session.prepareCancelAndKeep(), false);
  const reopened = recoverySession(async () => view);
  await reopened.refresh();
  assert.equal(reopened.canVerify(), false);
  assert.equal(
    reopened.getSnapshot().dispositionReceipt?.result?.status,
    "cancelledAndWorkspaceRemoved",
  );
});

test("a removal result cannot acknowledge a keep request", () => {
  const view = verifiedView();
  const receipt = {
    schemaVersion: 1,
    request: {
      projectId: "p",
      requestId: "dispose",
      target: view.target,
      verificationRequestId: "verify-1",
      choice: "cancelAndKeep",
    },
    result: {
      status: "cancelledAndWorkspaceRemoved",
      report: view.operation!.result,
      removed: {
        workspace: "work",
        attemptId: "attempt",
        fineTaskId: "fine",
        outputFileCount: 0,
      },
      taskRevision: view.target.taskRevision + 1,
      runRevision: view.target.runRevision + 1,
      planRevision: 3,
    },
  };
  assert.equal(
    objectRecoveryDispositionOperationSchema.safeParse(receipt).success,
    false,
  );
});
