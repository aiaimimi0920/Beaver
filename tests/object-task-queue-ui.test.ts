import assert from "node:assert/strict";
import test from "node:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { parseObjectTaskSnapshot } from "../src/shared/object-task-snapshot";
import { previewTasksFromSnapshot } from "../src/ui/object-preview/object-task-preview-adapter";
import { ObjectTaskCancellationDialog } from "../src/ui/object-tasks/ObjectTaskCancellationDialog";
import { ObjectTaskList } from "../src/ui/object-tasks/ObjectTaskList";
import { ObjectTaskRevisionDialog } from "../src/ui/object-tasks/ObjectTaskRevisionDialog";
import { ObjectTaskCancellation } from "../src/ui/object-tasks/object-task-cancellation";
import { ObjectTaskRevision } from "../src/ui/object-tasks/object-task-revision";
import {
  objectTaskCancellationReceipt,
  objectTaskSnapshot,
} from "./fixtures/object-tasks";

const heldStates = [
  ["queued", "已领取（待执行）", "排队中"],
  ["running", "执行中", "执行中"],
  ["awaitingAcceptance", "待验收", "待验收"],
  ["failed", "失败待恢复", "执行失败"],
] as const;

function heldSnapshot(status: (typeof heldStates)[number][0]) {
  const snapshot = objectTaskSnapshot();
  const revision = status === "queued" ? 1 : 2;
  Object.assign(
    snapshot.tasks.find((task) => task.id === "medium")!,
    {
      status,
      revision,
    },
  );
  Object.assign(
    snapshot.runs.find((run) => run.id === "run-hero")!,
    {
      status,
      revision,
    },
  );
  return snapshot;
}

test("queue lifecycle snapshots accept held states while fine tasks remain planned", () => {
  for (const [status] of heldStates) {
    const snapshot = heldSnapshot(status);
    assert.deepEqual(parseObjectTaskSnapshot(snapshot, "p"), snapshot);
    assert.equal(
      snapshot.tasks.find((task) => task.id === "fine")!.status,
      "planned",
    );
    snapshot.runs[0]!.status = "planned";
    assert.throws(
      () => parseObjectTaskSnapshot(snapshot, "p"),
      /迭代与中修状态不一致/,
    );
  }
});

test("unimplemented execution and acceptance states are not silently parsed", () => {
  for (const status of ["paused", "completed", "succeeded", "accepted"]) {
    const snapshot = heldSnapshot("queued");
    Object.assign(
      snapshot.tasks.find((task) => task.id === "medium")!,
      { status },
    );
    Object.assign(snapshot.runs[0]!, { status });
    assert.throws(() => parseObjectTaskSnapshot(snapshot, "p"));
  }
});

test("task and run labels reflect queue state and revisions stay read-only across responsibility", () => {
  for (const [status, label] of heldStates) {
    const html = renderToStaticMarkup(
      createElement(ObjectTaskList, {
        snapshot: heldSnapshot(status),
        onRevise: () => {},
        onCancel: () => {},
      }),
    );
    assert.ok(
      html.includes(`<span class="object-task-status">${label}</span>`),
    );
    assert.ok(html.includes(`<dt>迭代状态</dt><dd>${label}</dd>`));
    for (const title of ["Game", "Hero", "Movement"]) {
      assert.ok(
        html.includes(`aria-label="查看任务 ${title} 的修订历史"`),
        title,
      );
      assert.ok(!html.includes(`aria-label="修订任务 ${title} 的定义"`), title);
    }
    assert.match(html, /aria-label="修订任务 Independent iteration 的定义"/);
    assert.doesNotMatch(html, /aria-label="撤销任务 Hero"/);
    assert.match(html, /aria-label="撤销任务 Movement"/);
    assert.doesNotMatch(html, /progressbar|\d+%/);
  }
});

test("preview shows running, waiting acceptance and failure without inventing progress", () => {
  for (const [status, , previewStatus] of heldStates) {
    const tasks = previewTasksFromSnapshot(heldSnapshot(status));
    assert.equal(
      tasks.find((task) => task.id === "medium")!.status,
      previewStatus,
    );
    assert.equal(tasks.find((task) => task.id === "fine")!.status, "排队中");
    assert.ok(tasks.every((task) => task.progress === 0));
  }
});

test("running fine stays frozen while an unstarted sibling can still be cancelled", async () => {
  const snapshot = heldSnapshot("running");
  const fine = snapshot.tasks.find((task) => task.id === "fine")!;
  snapshot.tasks.push({
    ...fine,
    id: "later-fine",
    title: "Later fine",
    stageId: "later",
    dependsOn: ["fine"],
    identity: {
      schemaVersion: 1,
      layer: "fine",
      objectId: "hero",
      mediumTaskId: "medium",
      runId: "run-hero",
      stageId: "later",
    },
  });
  Object.assign(fine, { status: "running", revision: 1 });
  assert.deepEqual(parseObjectTaskSnapshot(snapshot, "p"), snapshot);
  const api = async () => {
    throw new Error("Unexpected API call");
  };
  const revision = new ObjectTaskRevision(
    "p",
    snapshot,
    "fine",
    api,
    async () => true,
  );
  assert.equal(revision.getSnapshot().phase, "readonly");
  assert.equal(await revision.submit(), null);
  assert.throws(
    () =>
      new ObjectTaskCancellation("p", snapshot, "fine", api, async () => true),
    /只能撤销/,
  );
  const later = new ObjectTaskCancellation(
    "p",
    snapshot,
    "later-fine",
    api,
    async () => true,
  );
  assert.equal(later.getSnapshot().phase, "reviewing");
  assert.deepEqual(
    later.tasks.map((task) => task.id),
    ["later-fine"],
  );
  assert.deepEqual(later.runs, []);
  const html = renderToStaticMarkup(
    createElement(ObjectTaskList, {
      snapshot,
      onRevise: () => {},
      onCancel: () => {},
    }),
  );
  assert.doesNotMatch(
    html,
    /aria-label="撤销任务 Movement"|aria-label="修订任务 Movement 的定义"/,
  );
  assert.match(html, /aria-label="查看任务 Movement 的修订历史"/);
  assert.match(html, /aria-label="撤销任务 Later fine"/);
  assert.equal(
    previewTasksFromSnapshot(snapshot).find((task) => task.id === "fine")!
      .status,
    "执行中",
  );
});

test("held runs prevent definition writes from medium, fine and coarse revision dialogs", async () => {
  const calls: string[] = [];
  for (const [status] of heldStates) {
    for (const taskId of ["medium", "fine", "coarse"]) {
      const session = new ObjectTaskRevision(
        "p",
        heldSnapshot(status),
        taskId,
        async (method) => {
          calls.push(method);
          throw new Error("Unexpected API call");
        },
        async () => true,
      );
      assert.equal(session.getSnapshot().phase, "readonly");
      session.updateDefinition({ title: "Changed locally" });
      session.updateReason("Changed scope");
      assert.equal(session.review(), false);
      assert.equal(await session.submit(), null);
      const html = renderToStaticMarkup(
        createElement(ObjectTaskRevisionDialog, { session, close: () => {} }),
      );
      assert.match(html, /当前任务定义|任务定义修订历史/);
      assert.doesNotMatch(html, /<textarea|核对修订|确认修订|本次修订影响范围/);
    }
  }
  assert.deepEqual(calls, []);
});

test("a claim during revision review displays the latest status and preserves rejected rebase input", async () => {
  for (const [status, label] of heldStates) {
    const calls: string[] = [];
    const latest = heldSnapshot(status);
    const session = new ObjectTaskRevision(
      "p",
      objectTaskSnapshot(),
      "medium",
      async (method) => {
        calls.push(method);
        if (method === "objectTask.snapshot") return latest;
        throw new Error("OBJECT_TASK_REVISION_CONFLICT");
      },
      async () => true,
      () => "revise-medium",
    );
    session.updateDefinition({
      title: "Local hero",
      prompt: "Keep these controls",
    });
    session.updateReason("Clarify controls");
    const definition = structuredClone(session.getSnapshot().definition);
    assert.equal(session.review(), true);
    assert.equal(await session.submit(), null);
    assert.equal(session.getSnapshot().phase, "conflict");
    assert.deepEqual(session.getSnapshot().latest, latest);
    const html = renderToStaticMarkup(
      createElement(ObjectTaskRevisionDialog, { session, close: () => {} }),
    );
    assert.ok(html.includes(`任务状态：${label}`));
    assert.match(html, /最新计划版本 1/);
    assert.equal(session.rebase(), false);
    assert.match(session.getSnapshot().latestError, /只能修订/);
    assert.deepEqual(session.getSnapshot().definition, definition);
    assert.equal(session.getSnapshot().reason, "Clarify controls");
    assert.equal(await session.submit(), null);
    assert.deepEqual(calls, [
      "objectTask.revisePlanned",
      "objectTask.snapshot",
    ]);
  }
});

test("coarse cancellation with held descendants is blocked without incomplete scope or write controls", async () => {
  const calls: string[] = [];
  for (const [status] of heldStates) {
    const session = new ObjectTaskCancellation(
      "p",
      heldSnapshot(status),
      "coarse",
      async (method) => {
        calls.push(method);
        throw new Error("Unexpected API call");
      },
      async () => true,
    );
    assert.equal(session.getSnapshot().phase, "blocked");
    assert.match(session.getSnapshot().error, /责任范围.*不能撤销规划/);
    const html = renderToStaticMarkup(
      createElement(ObjectTaskCancellationDialog, {
        session,
        close: () => {},
        refresh: async () => true,
      }),
    );
    assert.match(html, /role="alert"/);
    assert.match(html, /关闭并刷新后重新确认/);
    assert.doesNotMatch(
      html,
      /本次将撤销|本次撤销的任务|关闭的迭代|确认撤销|使用原请求重试撤销/,
    );
    assert.equal(await session.submit(), null);
    session.cancel();
    assert.equal(await session.submit(), null);
    assert.equal(session.getSnapshot().phase, "blocked");
  }
  assert.deepEqual(calls, []);
});

test("planned fine cancellation remains available under a held run and independent work stays eligible", async () => {
  for (const [status] of heldStates) {
    const snapshot = heldSnapshot(status);
    const before = structuredClone(snapshot);
    const requests: unknown[] = [];
    let refreshes = 0;
    const receipt = {
      ...objectTaskCancellationReceipt,
      taskId: "fine",
      requestId: "cancel-fine",
    };
    const session = new ObjectTaskCancellation(
      "p",
      snapshot,
      "fine",
      async (method, input) => {
        assert.equal(method, "objectTask.cancelPlanned");
        requests.push(input);
        return receipt;
      },
      async () => {
        refreshes++;
        return true;
      },
      () => "cancel-fine",
    );
    assert.equal(session.getSnapshot().phase, "reviewing");
    assert.deepEqual(
      session.tasks.map((task) => task.id),
      ["fine"],
    );
    assert.deepEqual(session.runs, []);
    assert.deepEqual(await session.submit(), receipt);
    assert.deepEqual(requests, [
      {
        projectId: "p",
        taskId: "fine",
        requestId: "cancel-fine",
        expectedTaskRevision: 0,
        expectedPlanRevision: 1,
      },
    ]);
    assert.equal(refreshes, 1);
    assert.deepEqual(snapshot, before);
    const independent = new ObjectTaskCancellation(
      "p",
      snapshot,
      "independent",
      async () => null,
      async () => true,
    );
    assert.equal(independent.getSnapshot().phase, "reviewing");
    assert.deepEqual(
      independent.tasks.map((task) => task.id),
      ["independent"],
    );
    assert.deepEqual(
      independent.runs.map((run) => run.id),
      ["run-independent"],
    );
  }
});
