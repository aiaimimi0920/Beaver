import assert from "node:assert/strict";
import test from "node:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { ObjectTaskExecution } from "../src/ui/object-tasks/object-task-execution";
import { ObjectTaskResume } from "../src/ui/object-tasks/object-task-resume";
import { ObjectStageAdvancePanel } from "../src/ui/object-tasks/ObjectStageAdvancePanel";
import { ObjectTaskResumePanel } from "../src/ui/object-tasks/ObjectTaskResumePanel";
import { prepareStageAdvance } from "../src/ui/object-tasks/object-stage-advance";
import {
  objectRecoveryResumeRequestSchema,
  objectRecoveryResumeOperationSchema,
} from "../src/shared/object-recovery";
import { objectAttemptCheckRequestSchema } from "../src/shared/object-attempt-checks";
import { execution, deferred } from "./fixtures/object-attempts";
import { objectTaskSnapshot } from "./fixtures/object-tasks";
import { verifiedView } from "./fixtures/object-recovery";

const approval = {
  attemptId: "attempt-1",
  checkRequestId: "check",
  nextFineTaskId: "fine-next",
  nextFineRevision: 0,
  acceptanceNote: "Reviewed movement",
};
function receipt(input: unknown) {
  return objectRecoveryResumeOperationSchema.parse({
    schemaVersion: 1,
    request: objectRecoveryResumeRequestSchema.parse(input),
    result: {
      status: "started",
      report: verifiedView().operation!.result,
      attemptId: "attempt-2",
      fineTaskId: "fine-next",
    },
  });
}

test("production approval binds checks, next definition and note before explicit confirmation", async () => {
  const snapshot = objectTaskSnapshot();
  const fine = snapshot.tasks.find((task) => task.id === "fine")!;
  snapshot.tasks.push({
    ...fine,
    id: "fine-next",
    title: "Review movement",
    position: 1,
    dependsOn: ["fine"],
  });
  const attempt = execution("awaitingGate").attempt;
  attempt.taskRevision = 3;
  const sent: unknown[] = [];
  let ids = 0;
  const session = new ObjectTaskExecution(
    "p",
    snapshot,
    "medium",
    async (method, input) => {
      if (method === "objectTask.recovery") return verifiedView();
      if (method === "objectTask.attempts") return [];
      if (method === "objectTask.checkAttempt")
        return {
          request: objectAttemptCheckRequestSchema.parse(input),
          runnerVersion: 1,
          attemptDigest: "a".repeat(64),
          inputDigest: "b".repeat(64),
          outputDigest: "c".repeat(64),
          time: "2026-09-25T12:00:00Z",
          passed: true,
          rules: ["checkpoint-integrity", "code-structure"].map((id) => ({
            id,
            version: 1,
            passed: true,
            issues: [],
            filesChecked: 0,
          })),
        };
      assert.equal(method, "objectTask.advanceAttempt");
      sent.push(structuredClone(input));
      if (sent.length === 1) throw new Error("response lost");
      return receipt(input);
    },
    async () => true,
    () => "request-" + ++ids,
  );
  await session.recovery.refresh();
  assert.equal(prepareStageAdvance(session, attempt, "Reviewed"), false);
  await session.checksFor(attempt).run();
  assert.equal(prepareStageAdvance(session, attempt, " "), false);
  assert.equal(
    prepareStageAdvance(session, { ...attempt, taskRevision: 2 }, "Reviewed"),
    false,
  );
  assert.match(
    renderToStaticMarkup(
      createElement(ObjectStageAdvancePanel, { session, attempt }),
    ),
    /Review movement/,
  );
  assert.equal(prepareStageAdvance(session, attempt, "Reviewed"), true);
  assert.equal(sent.length, 0);
  assert.match(
    renderToStaticMarkup(
      createElement(ObjectTaskResumePanel, { session: session.recovery }),
    ),
    /接受当前细任务/,
  );
  await session.recovery.resume.submit();
  assert.equal(session.recovery.resume.getSnapshot().retryAvailable, true);
  await session.recovery.resume.submit();
  assert.deepEqual(sent[0], sent[1]);
  const request = objectRecoveryResumeRequestSchema.parse(sent[0]);
  assert.equal(request.advance?.nextFineTaskId, "fine-next");
  assert.equal(request.advance?.acceptanceNote, "Reviewed");
  assert.equal(request.advance?.checkRequestId, "request-1");
});

test("pending advance restores without execution, exact retry uses advance API, late closed response is ignored", async () => {
  const view = verifiedView();
  const pending = {
    schemaVersion: 1 as const,
    request: objectRecoveryResumeRequestSchema.parse({
      projectId: "p",
      requestId: "saved",
      target: view.target,
      verificationRequestId: "verify-1",
      advance: { ...approval, acceptanceNote: "  Reviewed movement  " },
    }),
    result: null,
  };
  const response = deferred<unknown>();
  assert.equal(
    pending.request.advance?.acceptanceNote,
    "  Reviewed movement  ",
  );
  let calls = 0;
  let started = 0;
  const session = new ObjectTaskResume(
    {
      projectId: "p",
      view: () => view,
      available: () => true,
      changed: () => {},
      refresh: async () => true,
      started: () => {
        started++;
      },
    },
    async (method, input) => {
      assert.equal(method, "objectTask.advanceAttempt");
      assert.deepEqual(input, pending.request);
      calls++;
      return calls === 1 ? response.promise : receipt(input);
    },
    () => "unused",
  );
  session.observe({ ...view, resume: pending, canDispose: false });
  assert.equal(calls, 0);
  const submitting = session.submit();
  session.cancel();
  response.resolve(receipt(pending.request));
  await submitting;
  assert.equal(started, 0);
  assert.equal(session.getSnapshot().receipt, null);
  assert.equal(session.getSnapshot().retryAvailable, true);
  await session.submit();
  assert.equal(started, 1);
  assert.equal(calls, 2);
});

test("synchronous close suppresses approval dispatch and post-receipt refresh", async () => {
  for (const closeAt of ["submit", "receipt"]) {
    let calls = 0;
    let refreshes = 0;
    let started = 0;
    const session = new ObjectTaskResume(
      {
        projectId: "p",
        view: verifiedView,
        available: () => true,
        changed: () => {
          const state = session.getSnapshot();
          if (
            (closeAt === "submit" && state.submitting) ||
            (closeAt === "receipt" && state.receipt)
          )
            session.cancel();
        },
        refresh: async () => {
          refreshes++;
          return true;
        },
        started: () => {
          started++;
        },
      },
      async (_method, input) => {
        calls++;
        return receipt(input);
      },
      () => "close",
    );
    assert.equal(session.prepareAdvance(approval), true);
    await session.submit();
    assert.equal(calls, closeAt === "submit" ? 0 : 1);
    assert.equal(refreshes, 0);
    assert.equal(started, 0);
    assert.equal(!!session.getSnapshot().receipt, closeAt === "receipt");
  }
});

test("advance receipt rejects a different successor and oversized UTF-8 owner notes", () => {
  const request = {
    projectId: "p",
    requestId: "saved",
    target: verifiedView().target,
    verificationRequestId: "verify-1",
    advance: approval,
  };
  const wrong = receipt(request);
  if (wrong.result?.status === "started") wrong.result.fineTaskId = "foreign";
  assert.equal(
    objectRecoveryResumeOperationSchema.safeParse(wrong).success,
    false,
  );
  assert.equal(
    objectRecoveryResumeRequestSchema.safeParse({
      ...request,
      advance: { ...approval, acceptanceNote: "验".repeat(1334) },
    }).success,
    false,
  );
});
