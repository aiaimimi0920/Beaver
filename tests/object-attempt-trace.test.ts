import assert from "node:assert/strict";
import test from "node:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { ObjectAttemptTrace } from "../src/ui/object-tasks/object-attempt-trace";
import { ObjectAttemptTracePanel } from "../src/ui/object-tasks/ObjectAttemptTracePanel";

const identity = { projectId: "p", runId: "r", attemptId: "a" };
const entry = {
  sequence: 1,
  operation: "commandExecution",
  phase: "completed",
  status: "failed",
};
const html = (session: ObjectAttemptTrace) =>
  renderToStaticMarkup(createElement(ObjectAttemptTracePanel, { session }));

test("trace displays persisted outcomes, absence, truncation and errors", async () => {
  for (const [response, expected] of [
    [
      { request: identity, entries: [entry], truncated: true },
      /命令执行.*失败/,
    ],
    [
      { request: identity, entries: [], truncated: false },
      /旧版本执行不会补录/,
    ],
  ] as const) {
    const session = new ObjectAttemptTrace(
      identity,
      async (method, request) => {
        assert.equal(method, "objectTask.attemptTrace");
        assert.deepEqual(request, identity);
        return response;
      },
    );
    await session.refresh();
    assert.match(html(session), expected);
    if (response.truncated) assert.match(html(session), /256 条通知/);
  }
  const failed = new ObjectAttemptTrace(identity, async () => {
    throw new Error("STORE_UNAVAILABLE");
  });
  await failed.refresh();
  assert.match(html(failed), /STORE_UNAVAILABLE/);
});

test("trace rejects foreign identity and nonsequential evidence", async () => {
  for (const response of [
    {
      request: { ...identity, runId: "other" },
      entries: [entry],
      truncated: false,
    },
    {
      request: identity,
      entries: [{ ...entry, sequence: 2 }],
      truncated: false,
    },
  ]) {
    const session = new ObjectAttemptTrace(identity, async () => response);
    await session.refresh();
    assert.equal(session.getSnapshot().response, null);
    assert.ok(session.getSnapshot().error);
  }
});

test("refresh and close ignore stale trace replies", async () => {
  const pending: ((value: unknown) => void)[] = [];
  const session = new ObjectAttemptTrace(
    identity,
    async () => new Promise((resolve) => pending.push(resolve)),
  );
  const first = session.refresh();
  const second = session.refresh();
  pending[1]!({ request: identity, entries: [entry], truncated: false });
  await second;
  pending[0]!({ request: identity, entries: [], truncated: false });
  await first;
  assert.equal(session.getSnapshot().response?.entries.length, 1);
  const closing = session.refresh();
  session.cancel();
  pending[2]!({ request: identity, entries: [entry], truncated: false });
  await closing;
  assert.equal(session.getSnapshot().response, null);
});
