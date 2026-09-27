import assert from "node:assert/strict";
import test from "node:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import {
  objectAttemptCheckRequestSchema,
  objectAttemptCheckReportSchema,
} from "../src/shared/object-attempt-checks";
import { ObjectAttemptChecks } from "../src/ui/object-tasks/object-attempt-checks";
import { ObjectAttemptCheckPanel } from "../src/ui/object-tasks/ObjectAttemptCheckPanel";
import { execution, deferred } from "./fixtures/object-attempts";

function report(input: unknown) {
  return objectAttemptCheckReportSchema.parse({
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
  });
}

test("closing on busy notification prevents dispatch", async () => {
  for (const operation of ["run", "refresh"] as const) {
    let calls = 0;
    const session = new ObjectAttemptChecks(
      "p",
      execution("failed").attempt,
      async () => {
        calls++;
        return null;
      },
      () => "check-1",
    );
    session.subscribe(() => {
      if (session.getSnapshot().busy) session.cancel();
    });
    await session[operation]();
    assert.equal(calls, 0);
    assert.equal(session.getSnapshot().busy, false);
  }
});

test("uncertain responses retry the exact request; a completed check allows a new request", async () => {
  const inputs: unknown[] = [];
  let ids = 0;
  const session = new ObjectAttemptChecks(
    "p",
    execution("failed").attempt,
    async (_, input) => {
      inputs.push(input);
      if (inputs.length === 1) throw new Error("response lost");
      return report(input);
    },
    () => "check-" + ++ids,
  );
  await session.run();
  assert.equal(session.getSnapshot().retry, true);
  await session.run();
  assert.deepEqual(inputs[0], inputs[1]);
  assert.equal(session.getSnapshot().reports.length, 1);
  await session.run();
  assert.notDeepEqual(inputs[1], inputs[2]);
  assert.equal(session.getSnapshot().reports.length, 2);
});

test("late results after close are ignored and reopening reads saved reports", async () => {
  const pending = deferred<unknown>();
  let saved: unknown;
  const session = new ObjectAttemptChecks(
    "p",
    execution("interrupted").attempt,
    async (method, input) => {
      if (method === "objectTask.attemptChecks") return [saved];
      saved = report(input);
      return pending.promise;
    },
    () => "check-1",
  );
  const running = session.run();
  session.cancel();
  pending.resolve(saved);
  await running;
  assert.equal(session.getSnapshot().reports.length, 0);
  await session.refresh();
  assert.equal(session.getSnapshot().reports.length, 1);
  assert.equal(session.getSnapshot().retry, false);
  const html = renderToStaticMarkup(
    createElement(ObjectAttemptCheckPanel, { session }),
  );
  assert.match(html, /不代表当前工作区状态或验收通过/);
  assert.match(html, /技术检查通过/);
  assert.match(html, /check-1/);
});

test("foreign identity and invalid aggregate results are rejected without losing retry identity", async () => {
  for (const mutation of [
    (value: ReturnType<typeof report>) => {
      value.request.target.claimToken = "foreign";
    },
    (value: ReturnType<typeof report>) => {
      value.request.requestId = "wrong";
    },
    (value: ReturnType<typeof report>) => {
      value.passed = false;
    },
    (value: ReturnType<typeof report>) => {
      value.rules[0].issues = ["corrupt"];
    },
  ]) {
    const session = new ObjectAttemptChecks(
      "p",
      execution("failed").attempt,
      async (_, input) => {
        const value = report(input);
        mutation(value);
        return value;
      },
      () => "check-1",
    );
    await session.run();
    assert.equal(session.getSnapshot().reports.length, 0);
    assert.equal(session.getSnapshot().retry, true);
    assert.ok(session.getSnapshot().error);
  }
});

test("historical query rejects cross-attempt reports and duplicate request IDs", async () => {
  const value = report({
    projectId: "p",
    requestId: "check-1",
    target: execution("failed").attempt.target,
  });
  for (const reports of [
    [value, value],
    [{ ...value, request: { ...value.request, projectId: "other" } }],
  ]) {
    const session = new ObjectAttemptChecks(
      "p",
      execution("failed").attempt,
      async () => reports,
    );
    await session.refresh();
    assert.ok(session.getSnapshot().error);
    assert.deepEqual(session.getSnapshot().reports, []);
  }
  assert.throws(
    () => new ObjectAttemptChecks("p", execution().attempt, async () => null),
  );
});
