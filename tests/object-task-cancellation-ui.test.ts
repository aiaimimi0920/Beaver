import assert from "node:assert/strict";
import test from "node:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { parseObjectTaskSnapshot } from "../src/shared/object-task-snapshot";
import { objectTaskPlanSchema } from "../src/shared/object-tasks";
import { ObjectTaskCancellationDialog } from "../src/ui/object-tasks/ObjectTaskCancellationDialog";
import { ObjectTaskDraftEditor } from "../src/ui/object-tasks/ObjectTaskDraftEditor";
import { ObjectTaskList } from "../src/ui/object-tasks/ObjectTaskList";
import { ObjectTaskCancellation } from "../src/ui/object-tasks/object-task-cancellation";
import { ObjectTaskWorkspace } from "../src/ui/object-tasks/object-task-workspace";
import {
  cancelledObjectTaskSnapshot,
  objectTaskCancellationReceipt,
  objectTaskSnapshot,
} from "./fixtures/object-tasks";

test("cancelled tasks and runs render accurately with controls only for planned tasks", () => {
  const snapshot = cancelledObjectTaskSnapshot();
  snapshot.tasks.find((task) => task.id === "independent")!.dependsOn = [
    "medium",
  ];
  const parsed = parseObjectTaskSnapshot(snapshot, "p");
  assert.deepEqual(parsed, snapshot);
  const html = renderToStaticMarkup(
    createElement(ObjectTaskList, {
      snapshot: parsed,
      onCancel: () => {},
    }),
  );
  assert.equal((html.match(/<button/g) ?? []).length, 2);
  assert.match(html, /aria-label="撤销任务 Game"/);
  assert.match(html, /aria-label="撤销任务 Independent iteration"/);
  assert.doesNotMatch(html, /aria-label="撤销任务 (Hero|Movement)"/);
  assert.match(html, /<dt>迭代状态<\/dt><dd>已撤销<\/dd>/);
  assert.match(html, /未记录基准（已撤销）/);
  assert.match(html, /class="object-task-dependency">medium（已撤销）/);
  assert.doesNotMatch(html, /progressbar|\d+%/);
  const busy = renderToStaticMarkup(
    createElement(ObjectTaskList, {
      snapshot: parsed,
      onCancel: () => {},
      cancelDisabled: true,
    }),
  );
  assert.equal((busy.match(/<button[^>]*disabled=""/g) ?? []).length, 2);
});

test("confirmation dialog displays ownership scope and preserves unrelated work", () => {
  const session = new ObjectTaskCancellation(
    "p",
    objectTaskSnapshot(),
    "medium",
    async () => {
      throw new Error("Unexpected API call");
    },
    async () => true,
  );
  const html = renderToStaticMarkup(
    createElement(ObjectTaskCancellationDialog, {
      session,
      close: () => {},
      refresh: async () => true,
    }),
  );
  assert.match(html, /role="dialog"/);
  assert.match(html, /aria-label="本次撤销的任务"/);
  assert.match(html, /2 项已规划任务/);
  assert.match(html, /1 次迭代/);
  for (const text of [
    "Movement",
    "Hero",
    "run-hero",
    "独立任务",
    "假设依据",
    "确认撤销",
  ]) {
    assert.ok(html.includes(text), text);
  }
  assert.doesNotMatch(html, /Independent iteration|run-independent/);
});

test("confirmed cancellation with a refresh failure offers refresh without another write", async () => {
  const session = new ObjectTaskCancellation(
    "p",
    objectTaskSnapshot(),
    "medium",
    async () => objectTaskCancellationReceipt,
    async () => false,
    () => "cancel-medium",
  );
  await session.submit();
  const html = renderToStaticMarkup(
    createElement(ObjectTaskCancellationDialog, {
      session,
      close: () => {},
      refresh: async () => true,
    }),
  );
  assert.match(html, /role="alert"/);
  assert.match(html, /撤销已确认/);
  assert.match(html, /重试刷新任务列表/);
  assert.doesNotMatch(html, /确认撤销|使用原请求重试撤销/);
});

test("plan conflict offers a retain-content rebase and cancelled parents cannot be reproposed as choices", async () => {
  for (const parentTaskId of ["medium", null]) {
    const plan = objectTaskPlanSchema.parse({
      tasks: [
        {
          id: "medium",
          granularity: "medium",
          title: "Hero",
          prompt: "Make a hero",
          acceptance: "Playable",
          objectId: "hero",
          parentTaskId: "coarse",
        },
        {
          id: "fine-new",
          granularity: "fine",
          title: "New movement",
          prompt: "Improve movement",
          acceptance: "Playable",
          objectId: "hero",
          parentTaskId,
          stageId: "movement-new",
        },
      ],
    });
    const workspace = new ObjectTaskWorkspace("p", async (method) => {
      if (method === "objectTask.snapshot")
        return cancelledObjectTaskSnapshot();
      if (method === "objectTask.getDraft")
        return {
          projectId: "p",
          id: "object-task-plan",
          revision: 1,
          planRevision: 1,
          plan,
        };
      throw new Error(`Unexpected method: ${method}`);
    });
    await workspace.refresh();
    const state = workspace.getSnapshot();
    assert.ok(state.kind === "ready");
    const html = renderToStaticMarkup(
      createElement(ObjectTaskDraftEditor, { state, workspace }),
    );
    assert.match(html, /任务首部插入/);
    assert.match(html, /任务尾部插入/);
    assert.match(html, /在任务 Hero 前插入/);
    assert.match(html, /建议标题/);
    assert.match(html, /保留本地内容并采用计划版本 2/);
    assert.match(html, /丢弃当前草稿并从最新计划开始/);
    const parentSelects = [
      ...html.matchAll(/责任父任务<select[^>]*>(.*?)<\/select>/g),
    ];
    const fineOptions = parentSelects[1]?.[1];
    assert.ok(fineOptions);
    assert.match(fineOptions, /value="independent"/);
    if (parentTaskId) {
      assert.equal((fineOptions.match(/value="medium"/g) ?? []).length, 1);
      assert.match(
        fineOptions,
        /<option[^>]*value="medium"[^>]*disabled=""[^>]*selected=""/,
      );
      assert.match(fineOptions, /已撤销，请更换父任务/);
    } else {
      assert.doesNotMatch(fineOptions, /value="medium"/);
    }
  }
});
