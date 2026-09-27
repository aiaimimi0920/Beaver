import assert from "node:assert/strict";
import test from "node:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import {
  coarseDispatchReceiptSchema,
  setCoarsePausedSchema,
} from "../src/shared/object-task-dispatch";
import { parseObjectTaskSnapshot } from "../src/shared/object-task-snapshot";
import { ObjectTaskDispatch } from "../src/ui/object-tasks/object-task-dispatch";
import { ObjectTaskDispatchDialog } from "../src/ui/object-tasks/ObjectTaskDispatchDialog";
import { ObjectTaskList } from "../src/ui/object-tasks/ObjectTaskList";
import { objectTaskSnapshot } from "./fixtures/object-tasks";
import { dispatchControl } from "./fixtures/object-task-dispatch";

const control = {
  schemaVersion: 1 as const,
  projectId: "p",
  taskId: "coarse",
  paused: true,
  revision: 1,
};
function receipt(input: unknown) {
  const request = setCoarsePausedSchema.parse(input);
  return coarseDispatchReceiptSchema.parse({
    request,
    result: {
      ...control,
      paused: request.paused,
      revision: request.expectedControlRevision + 1,
    },
  });
}

test("coarse snapshot policy fails closed on missing, foreign, duplicate or medium targets", () => {
  const snapshot = objectTaskSnapshot();
  for (const controls of [
    undefined,
    [control, control],
    [{ ...control, taskId: "medium" }],
    [{ ...control, projectId: "other" }],
    [{ ...control, revision: 0 }],
  ]) {
    assert.throws(() =>
      parseObjectTaskSnapshot(
        { ...snapshot, coarseDispatchControls: controls },
        "p",
      ),
    );
  }
  snapshot.coarseDispatchControls = [control];
  assert.deepEqual(parseObjectTaskSnapshot(snapshot, "p"), snapshot);
});

test("coarse confirmation retries the frozen request despite transport mutation and newer policy", async () => {
  const snapshot = objectTaskSnapshot();
  const requests: unknown[] = [];
  const session = new ObjectTaskDispatch(
    "p",
    snapshot,
    "coarse",
    async (method, input) => {
      if (method === "objectTask.snapshot") return snapshot;
      assert.equal(method, "objectTask.setCoarsePaused");
      requests.push(structuredClone(input));
      if (requests.length === 1) {
        if (input && typeof input === "object")
          Object.assign(input, { paused: false, requestId: "mutated" });
        throw new Error("Response lost");
      }
      return receipt(input);
    },
    async () => true,
    () => "pause-coarse",
  );
  await session.refresh();
  assert.equal(requests.length, 0);
  const html = renderToStaticMarkup(
    createElement(ObjectTaskDispatchDialog, { session, close: () => {} }),
  );
  assert.match(html, /现有及后续新增中修/);
  assert.match(html, /保留每个中修自身的暂停设置/);
  assert.match(html, /确认暂停派发/);
  assert.equal(await session.confirm(), null);
  snapshot.coarseDispatchControls = [
    { ...control, paused: false, revision: 2 },
  ];
  await session.refresh();
  assert.ok(await session.confirm());
  assert.deepEqual(requests[0], requests[1]);
  assert.deepEqual(requests[0], {
    projectId: "p",
    taskId: "coarse",
    requestId: "pause-coarse",
    expectedTaskRevision: 0,
    expectedControlRevision: 0,
    paused: true,
  });
});

test("coarse and inherited pause are visible while child pause stays independently controllable", async () => {
  const snapshot = objectTaskSnapshot();
  snapshot.coarseDispatchControls = [control];
  snapshot.dispatchControls = [dispatchControl()];
  const html = renderToStaticMarkup(
    createElement(ObjectTaskList, { snapshot, onDispatch: () => {} }),
  );
  assert.match(html, /管理任务 Game 的派发/);
  assert.equal((html.match(/所属粗修已暂停派发/g) ?? []).length, 1);
  const session = new ObjectTaskDispatch(
    "p",
    snapshot,
    "medium",
    async () => snapshot,
    async () => true,
  );
  await session.refresh();
  assert.equal(session.getSnapshot().parentPaused, true);
  assert.equal(session.getSnapshot().control.paused, true);
  assert.match(
    renderToStaticMarkup(
      createElement(ObjectTaskDispatchDialog, { session, close: () => {} }),
    ),
    /仍需等待粗修解除暂停/,
  );
  const coarse = new ObjectTaskDispatch(
    "p",
    snapshot,
    "coarse",
    async (method, input) =>
      method === "objectTask.snapshot" ? snapshot : receipt(input),
    async () => true,
  );
  await coarse.refresh();
  assert.equal((await coarse.confirm())?.result.paused, false);
  assert.equal(snapshot.dispatchControls[0]?.paused, true);
});

test("coarse conflict requires refresh and mismatched receipt remains unconfirmed", async () => {
  for (const conflict of [true, false]) {
    const snapshot = objectTaskSnapshot();
    const session = new ObjectTaskDispatch(
      "p",
      snapshot,
      "coarse",
      async (method, input) => {
        if (method === "objectTask.snapshot") return snapshot;
        if (conflict) throw new Error("OBJECT_TASK_DISPATCH_REVISION_CONFLICT");
        const result = receipt(input);
        result.request.requestId = "foreign";
        return result;
      },
      async () => true,
    );
    await session.refresh();
    assert.equal(await session.confirm(), null);
    assert.equal(session.getSnapshot().retryAvailable, !conflict);
    assert.equal(session.getSnapshot().receipt, null);
  }
});
