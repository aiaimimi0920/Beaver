import assert from "node:assert/strict";
import test from "node:test";
import {
  createStudioState,
  nextPreviewStatus,
  pausePreviewTask,
  resumePreviewTask,
  studioSchema,
} from "../src/ui/studio-preview";

test("pause and resume preserve unresolved decisions and integration phase", () => {
  const tasks = createStudioState().tasks;
  for (const task of tasks.filter((t) =>
    ["需要你决定", "正在整合"].includes(t.status),
  ))
    assert.equal(resumePreviewTask(pausePreviewTask(task)).status, task.status);
});

test("preview state is isolated, serializable and includes both conflict scenarios", () => {
  const first = createStudioState();
  const second = createStudioState();
  first.notes[0]!.draft = "human edit";
  first.assets[0]!.notes = "asset feedback";
  assert.equal(second.notes[0]!.draft, undefined);
  assert.notEqual(second.assets[0]!.notes, "asset feedback");
  assert.deepEqual(
    studioSchema.parse(JSON.parse(JSON.stringify(first))),
    first,
  );
  assert.deepEqual(
    first.tasks.filter((t) => t.status === "需要你决定").map((t) => t.conflict),
    ["script", "binary"],
  );
});

test("preview stage advancement never bypasses a decision, acceptance or pause", () => {
  assert.equal(nextPreviewStatus("执行中"), "正在整合");
  assert.equal(nextPreviewStatus("正在整合"), "待验收");
  for (const status of [
    "待验收",
    "需要你决定",
    "已暂停",
    "已认可",
    "已回退",
  ] as const)
    assert.equal(nextPreviewStatus(status), status);
});

test("corrupt or incompatible preview storage is not trusted", () => {
  assert.equal(studioSchema.safeParse({ version: 2 }).success, false);
  const state: unknown = {
    ...createStudioState(),
    tasks: [{ id: "bad", status: "completed" }],
  };
  assert.equal(studioSchema.safeParse(state).success, false);
});
