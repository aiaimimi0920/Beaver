import assert from "node:assert/strict";
import test from "node:test";
import { ObjectTaskResume } from "../src/ui/object-tasks/object-task-resume";
import {
  objectRecoveryResumeRequestSchema,
  type ObjectRecoveryResumeOperation,
} from "../src/shared/object-recovery";
import { verifiedView } from "./fixtures/object-recovery";

function setup(
  api: (method: string, input: unknown) => Promise<unknown>,
  refresh = async () => true,
) {
  const view = { ...verifiedView(), canResume: true };
  const started: string[] = [];
  let ids = 0;
  const session = new ObjectTaskResume(
    {
      projectId: "p",
      view: () => view,
      available: () => true,
      changed: () => {},
      refresh,
      started: (id) => started.push(id),
    },
    api,
    () => "resume-" + ++ids,
  );
  return { session, view, started };
}
function receipt(input: unknown): ObjectRecoveryResumeOperation {
  return {
    schemaVersion: 1,
    request: objectRecoveryResumeRequestSchema.parse(input),
    result: {
      status: "started",
      report: verifiedView().operation!.result!,
      attemptId: "attempt-2",
      fineTaskId: "fine",
    },
  };
}

test("retry requires explicit confirmation and retains exact authorization after response loss", async () => {
  const sent: unknown[] = [];
  const { session, started } = setup(async (method, input) => {
    assert.equal(method, "objectTask.resumeRecovery");
    sent.push(structuredClone(input));
    if (sent.length === 1) throw new Error("response lost");
    return receipt(input);
  });
  assert.equal(await session.submit(), null);
  assert.equal(sent.length, 0);
  assert.equal(session.prepare(), true);
  assert.equal(sent.length, 0);
  assert.equal(await session.submit(), null);
  assert.equal(session.getSnapshot().retryAvailable, true);
  assert.equal(session.prepare(), false);
  assert.equal((await session.submit())?.result?.status, "started");
  assert.deepEqual(sent[0], sent[1]);
  assert.deepEqual(started, ["attempt-2"]);
});

test("reopening a pending request never launches until explicit retry", async () => {
  let calls = 0;
  const { session, view, started } = setup(async (_, input) => {
    calls++;
    return receipt(input);
  });
  const pending = receipt({
    projectId: "p",
    requestId: "saved",
    target: view.target,
    verificationRequestId: "verify-1",
  });
  pending.result = null;
  session.observe({
    ...view,
    canResume: false,
    canDispose: false,
    resume: pending,
  });
  assert.equal(calls, 0);
  assert.equal(session.getSnapshot().retryAvailable, true);
  const completed = await session.submit();
  assert.equal(completed?.request.requestId, "saved");
  session.observe({ ...view, resume: completed });
  assert.equal(calls, 1);
  assert.deepEqual(started, ["attempt-2"]);
});

test("mismatched or unfinished responses preserve the original request", async () => {
  const sent: unknown[] = [];
  const { session } = setup(async (_, input) => {
    sent.push(input);
    const result = receipt(input);
    if (sent.length === 1) result.request.requestId = "foreign";
    if (sent.length === 2) result.result = null;
    return result;
  });
  session.prepare();
  assert.equal(await session.submit(), null);
  assert.equal(await session.submit(), null);
  assert.equal(session.getSnapshot().retryAvailable, true);
  assert.equal((await session.submit())?.result?.status, "started");
  assert.deepEqual(sent[0], sent[1]);
  assert.deepEqual(sent[1], sent[2]);
});

test("committed receipt survives refresh failure", async () => {
  const { session } = setup(
    async (_, input) => receipt(input),
    async () => false,
  );
  session.prepare();
  await session.submit();
  assert.equal(session.getSnapshot().receipt?.result?.status, "started");
  assert.equal(session.getSnapshot().retryAvailable, false);
  assert.match(session.getSnapshot().error, /刷新失败/);
});

test("blocked retry releases local authorization for a fresh verification", async () => {
  const { session, view } = setup(async (_, input) => ({
    ...receipt(input),
    result: {
      status: "blocked",
      report: {
        ...verifiedView().operation!.result!,
        workspaceStatus: "drifted",
        issues: ["workspace_changed"],
      },
    },
  }));
  session.prepare();
  assert.equal((await session.submit())?.result?.status, "blocked");
  assert.equal(session.blocks(), false);
  view.canResume = false;
  assert.equal(session.prepare(), false);
  view.canResume = true;
  view.target.recoveryGeneration++;
  view.operation!.request.requestId = "verify-2";
  assert.equal(session.prepare(), true);
  assert.equal(
    session.getSnapshot().confirmation?.verificationRequestId,
    "verify-2",
  );
  assert.equal(session.getSnapshot().confirmation?.requestId, "resume-2");
});
