import assert from "node:assert/strict";
import test from "node:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import {
  objectTaskDraftSchema,
  saveObjectTaskDraftSchema,
} from "../src/shared/object-tasks";
import { ObjectTaskDraftEditor } from "../src/ui/object-tasks/ObjectTaskDraftEditor";
import { ObjectTaskTitleController } from "../src/ui/object-tasks/object-task-title-suggestion";
import { ObjectTaskWorkspace } from "../src/ui/object-tasks/object-task-workspace";
import { objectTaskSnapshot } from "./fixtures/object-tasks";

function ready(workspace: ObjectTaskWorkspace) {
  const state = workspace.getSnapshot();
  assert.ok(state.kind === "ready");
  return state;
}

async function setup() {
  let draft = objectTaskDraftSchema.parse({
    projectId: "p",
    id: "object-task-plan",
    revision: 1,
    planRevision: 1,
    plan: {
      tasks: [
        {
          id: "A",
          title: "Original",
          granularity: "coarse",
          prompt: "Add dash",
          acceptance: "Buffered input",
        },
      ],
    },
  });
  let resolve!: (result: unknown) => void;
  let reject!: (error: Error) => void;
  const calls: unknown[] = [];
  const api = async (method: string, input: unknown) => {
    if (method === "objectTask.snapshot") return objectTaskSnapshot();
    if (method === "objectTask.getDraft") return structuredClone(draft);
    if (method === "objectTask.suggestTitle") {
      calls.push(input);
      return new Promise<unknown>((ok, fail) => {
        resolve = ok;
        reject = fail;
      });
    }
    if (method === "objectTask.saveDraft") {
      const request = saveObjectTaskDraftSchema.parse(input);
      draft = {
        ...draft,
        revision: draft.revision + 1,
        plan: structuredClone(request.plan),
      };
      return structuredClone(draft);
    }
    throw new Error(method);
  };
  const workspace = new ObjectTaskWorkspace("p", api);
  await workspace.refresh();
  return {
    workspace,
    controller: new ObjectTaskTitleController(workspace, "A"),
    calls,
    resolve: (value: unknown) => resolve(value),
    reject: (error: Error) => reject(error),
    reopen: async () => {
      const w = new ObjectTaskWorkspace("p", api);
      await w.refresh();
      return w;
    },
  };
}

test("Codex candidate requires adoption, changes only title and survives draft reopening", async () => {
  const f = await setup();
  const before = structuredClone(ready(f.workspace).plan);
  const html = renderToStaticMarkup(
    createElement(ObjectTaskDraftEditor, {
      state: ready(f.workspace),
      workspace: f.workspace,
    }),
  );
  assert.match(html, /Codex 建议标题/);
  const pending = f.controller.request();
  await f.controller.request();
  assert.equal(f.calls.length, 1);
  assert.deepEqual(f.calls[0], {
    projectId: "p",
    prompt: "Add dash",
    acceptance: "Buffered input",
  });
  f.resolve({ title: "Buffered dash" });
  await pending;
  assert.deepEqual(ready(f.workspace).plan, before);
  assert.equal(f.controller.getSnapshot().candidate, "Buffered dash");
  f.workspace.updatePlan((plan) => ({
    ...plan,
    objects: [{ id: "new", name: "Hero", category: "角色" }],
  }));
  f.controller.accept();
  assert.deepEqual(
    ready(f.workspace).plan.tasks,
    before.tasks.map((task) => ({ ...task, title: "Buffered dash" })),
  );
  assert.equal(ready(f.workspace).plan.objects.length, 1);
  await f.workspace.save();
  assert.deepEqual(ready(await f.reopen()).plan, ready(f.workspace).plan);
});

for (const phase of ["request", "adopt"]) {
  test(
    "manual edits during " +
      phase +
      " reject stale suggestions without losing content",
    async () => {
      const f = await setup();
      const pending = f.controller.request();
      if (phase === "adopt") {
        f.resolve({ title: "Stale" });
        await pending;
      }
      f.workspace.updatePlan((plan) => ({
        ...plan,
        tasks: plan.tasks.map((task) => ({
          ...task,
          title: "Manual",
          prompt: "Add jump",
        })),
      }));
      if (phase === "request") {
        f.resolve({ title: "Stale" });
        await pending;
      }
      f.controller.accept();
      assert.match(f.controller.getSnapshot().error ?? "", /任务内容已变化/);
      assert.equal(ready(f.workspace).plan.tasks[0]?.title, "Manual");
      assert.equal(ready(f.workspace).plan.tasks[0]?.prompt, "Add jump");
    },
  );
}

test("deleted tasks and dismissed requests never recreate tasks or apply late results", async () => {
  const f = await setup();
  const pending = f.controller.request();
  f.workspace.updatePlan((plan) => ({ ...plan, tasks: [] }));
  f.resolve({ title: "Ghost" });
  await pending;
  f.controller.accept();
  assert.match(f.controller.getSnapshot().error ?? "", /任务已不存在/);
  assert.deepEqual(ready(f.workspace).plan.tasks, []);
  const other = await setup();
  const late = other.controller.request();
  other.controller.dismiss();
  other.resolve({ title: "Late" });
  await late;
  assert.equal(other.controller.getSnapshot().candidate, null);
  assert.equal(ready(other.workspace).plan.tasks[0]?.title, "Original");
});

test("provider failure and malformed output leave the draft unchanged", async () => {
  for (const outcome of ["error", "malformed"] as const) {
    const f = await setup();
    const before = structuredClone(ready(f.workspace).plan);
    const pending = f.controller.request();
    if (outcome === "error") f.reject(new Error("Provider unavailable"));
    else f.resolve({ title: "x".repeat(49) });
    await pending;
    assert.ok(f.controller.getSnapshot().error);
    assert.deepEqual(ready(f.workspace).plan, before);
  }
});
