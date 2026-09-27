import assert from "node:assert/strict";
import test from "node:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { objectExecutionSchema } from "../src/shared/object-attempts";
import { ObjectTaskCheckpointFiles } from "../src/ui/object-tasks/ObjectTaskCheckpointFiles";
import { ObjectAttemptFile } from "../src/ui/object-tasks/object-attempt-file";
import {
  execution,
  interruptReceipt,
  session,
} from "./fixtures/object-attempts";

const a = "a".repeat(64);
const b = "b".repeat(64);
const viewer = new ObjectAttemptFile(
  { projectId: "p", runId: "r", attemptId: "a" },
  async () => null,
);

test("frozen files show additions, removals, modifications and unchanged binary content safely", () => {
  const item = execution("awaitingGate");
  item.checkpoints = {
    input: { "same.glb": a, "edited.txt": a, "removed.txt": b },
    output: {
      "same.glb": a,
      "edited.txt": b,
      "<script>.txt": a,
      constructor: a,
    },
  };
  const html = renderToStaticMarkup(
    createElement(ObjectTaskCheckpointFiles, {
      checkpoints: item.checkpoints,
      viewer,
      outputCaptured: true,
    }),
  );
  assert.match(html, /same.glb<\/td><td>未变/);
  assert.match(html, /edited.txt<\/td><td>修改/);
  assert.match(html, /removed.txt<\/td><td>删除/);
  assert.match(html, /&lt;script&gt;.txt<\/td><td>新增/);
  assert.match(html, /constructor<\/td><td>新增/);
  assert.doesNotMatch(html, /<script>/);
  assert.ok(html.includes(a) && html.includes(b));
});

test("missing output is not mistaken for an empty checkpoint that deleted every input", () => {
  for (const captured of [false, true]) {
    const html = renderToStaticMarkup(
      createElement(ObjectTaskCheckpointFiles, {
        checkpoints: { input: { "keep.txt": a }, output: null },
        viewer,
        outputCaptured: captured,
      }),
    );
    assert.match(html, /keep.txt<\/td><td>待冻结/);
    assert.doesNotMatch(html, /<td>删除<\/td>/);
    assert.match(html, captured ? /请点击刷新文件清单读取/ : /输出尚未冻结/);
  }
  const empty = renderToStaticMarkup(
    createElement(ObjectTaskCheckpointFiles, {
      checkpoints: { input: { "keep.txt": a }, output: {} },
      viewer,
      outputCaptured: true,
    }),
  );
  assert.match(empty, /keep.txt<\/td><td>删除/);
});

test("large manifests render a bounded page without dropping the total count", () => {
  const input = Object.fromEntries(
    Array.from({ length: 201 }, (_, i) => [
      `file-${String(i).padStart(3, "0")}`,
      a,
    ]),
  );
  const html = renderToStaticMarkup(
    createElement(ObjectTaskCheckpointFiles, {
      checkpoints: { input, output: null },
      viewer,
      outputCaptured: false,
    }),
  );
  assert.equal(
    (html.match(/<tbody>[\s\S]*<\/tbody>/)?.[0].match(/<tr>/g) ?? []).length,
    100,
  );
  assert.match(html, /共 201 项/);
  assert.match(html, /下一页/);
  assert.doesNotMatch(html, /file-100/);
});

test("query rejects missing, malformed and state-inconsistent checkpoint manifests", () => {
  const done = execution("interrupted");
  for (const checkpoints of [
    undefined,
    { input: {}, output: null },
    { input: { x: "not-a-hash" }, output: {} },
  ]) {
    assert.equal(
      objectExecutionSchema.safeParse({ ...done, checkpoints }).success,
      false,
    );
  }
  assert.equal(
    objectExecutionSchema.safeParse({
      ...execution(),
      checkpoints: { input: {}, output: {} },
    }).success,
    false,
  );
});

test("explicit file refresh after interruption reads frozen output without repeating the control request", async () => {
  let current = execution();
  current.checkpoints.input = { "input.txt": a };
  const calls: string[] = [];
  let foreign = false;
  const controller = session(async (method, input) => {
    calls.push(method);
    if (method === "objectTask.attempts") {
      const result = structuredClone(current);
      if (foreign) result.attempt.projectId = "other";
      return [result];
    }
    const receipt = interruptReceipt(input);
    current = {
      ...current,
      attempt: receipt.result,
      availability: "finished",
      checkpoints: { input: { "input.txt": a }, output: { "output.txt": b } },
    };
    return receipt;
  });
  await controller.refresh();
  const receipt = await controller.interrupt("attempt-1");
  assert.equal(
    controller.getSnapshot().executions[0]?.checkpoints.output,
    null,
  );
  assert.equal(await controller.refresh(true), true);
  assert.deepEqual(controller.getSnapshot().executions[0]?.checkpoints.output, {
    "output.txt": b,
  });
  assert.deepEqual(controller.getSnapshot().receipt, receipt);
  foreign = true;
  assert.equal(await controller.refresh(true), false);
  assert.deepEqual(controller.getSnapshot().executions[0]?.checkpoints.output, {
    "output.txt": b,
  });
  assert.deepEqual(controller.getSnapshot().receipt, receipt);
  assert.deepEqual(calls, [
    "objectTask.attempts",
    "objectTask.interrupt",
    "objectTask.attempts",
    "objectTask.attempts",
  ]);
});
