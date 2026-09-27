import assert from "node:assert/strict";
import test from "node:test";
import { Children, isValidElement, type ReactNode } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import {
  objectTaskDraftSchema,
  objectTaskPlanSchema,
  saveObjectTaskDraftSchema,
} from "../src/shared/object-tasks";
import { ObjectTaskDraftEditor } from "../src/ui/object-tasks/ObjectTaskDraftEditor";
import { ObjectTaskWorkspace } from "../src/ui/object-tasks/object-task-workspace";
import { objectTaskSnapshot } from "./fixtures/object-tasks";

function ready(workspace: ObjectTaskWorkspace) {
  const state = workspace.getSnapshot();
  assert.equal(state.kind, "ready");
  if (state.kind !== "ready") throw new Error("Workspace not ready");
  return state;
}

function editor(workspace: ObjectTaskWorkspace) {
  return ObjectTaskDraftEditor({ state: ready(workspace), workspace });
}

function button(tree: ReactNode, label: string): () => void {
  let click: (() => void) | undefined;
  function visit(node: ReactNode) {
    Children.forEach(node, (child) => {
      if (
        !isValidElement<{
          children?: ReactNode;
          "aria-label"?: string;
          onClick?: () => void;
        }>(child)
      )
        return;
      const props = child.props;
      if (
        child.type === "button" &&
        (props["aria-label"] === label ||
          Children.toArray(props.children).join("") === label)
      )
        click = props.onClick;
      visit(props.children);
    });
  }
  visit(tree);
  assert.ok(click, `Missing button: ${label}`);
  return click;
}

async function setup() {
  let draft = objectTaskDraftSchema.parse({
    projectId: "p",
    id: "object-task-plan",
    revision: 1,
    planRevision: 1,
    plan: {
      tasks: ["A", "B", "C"].map((id, position) => ({
        id,
        position,
        title: id,
        granularity: "coarse",
        prompt: `Build ${id}`,
        acceptance: "Playable",
      })),
    },
  });
  const api = async (method: string, input: unknown) => {
    if (method === "objectTask.snapshot") return objectTaskSnapshot();
    if (method === "objectTask.getDraft") return structuredClone(draft);
    if (method === "objectTask.saveDraft") {
      const request = saveObjectTaskDraftSchema.parse(input);
      assert.equal(request.expectedRevision, draft.revision);
      draft = {
        ...draft,
        revision: draft.revision + 1,
        plan: structuredClone(request.plan),
      };
      return structuredClone(draft);
    }
    throw new Error(`Unexpected method: ${method}`);
  };
  const workspace = new ObjectTaskWorkspace("p", api);
  await workspace.refresh();
  return {
    workspace,
    reopen: async () => {
      const reopened = new ObjectTaskWorkspace("p", api);
      await reopened.refresh();
      return reopened;
    },
  };
}

for (const side of ["前", "后"] as const) {
  test(`stale ${side} insertion uses task identity in the latest plan and survives reopening`, async () => {
    const { workspace, reopen } = await setup();
    const click = button(editor(workspace), `在任务 B ${side}插入`);
    workspace.updatePlan((plan) => ({
      ...plan,
      objects: [
        { id: "keep", name: "Preserve latest edits", category: "其他" },
      ],
      tasks: [
        plan.tasks[1]!,
        plan.tasks[2]!,
        { ...plan.tasks[0]!, title: "Edited A" },
      ],
    }));
    click();
    const inserted = ready(workspace).plan.tasks.find(
      (task) => !["A", "B", "C"].includes(task.id),
    );
    assert.ok(inserted);
    const expected =
      side === "前"
        ? [inserted.id, "B", "C", "A"]
        : ["B", inserted.id, "C", "A"];
    assert.deepEqual(
      ready(workspace).plan.tasks.map((task) => task.id),
      expected,
    );
    assert.equal(ready(workspace).plan.tasks.at(-1)?.title, "Edited A");
    // Complete the blank task as a user would before saving the draft.
    workspace.updatePlan((plan) => ({
      ...plan,
      tasks: plan.tasks.map((task) =>
        task.id === inserted.id
          ? {
              ...task,
              granularity: "coarse",
              title: "New",
              prompt: "Build new",
              acceptance: "Playable",
            }
          : task,
      ),
    }));
    assert.equal(await workspace.save(), true);
    const restored = ready(await reopen());
    assert.deepEqual(
      restored.plan.tasks.map((task) => task.id),
      expected,
    );
    assert.deepEqual(
      restored.plan.tasks.map((task) => task.position),
      [0, 1, 2, 3],
    );
    assert.equal(restored.plan.objects[0]?.name, "Preserve latest edits");
    assert.equal(restored.dirty, false);
  });
}

test("a deleted anchor reports a visible error without restoring stale tasks", async () => {
  const { workspace } = await setup();
  const click = button(editor(workspace), "在任务 B 后插入");
  workspace.updatePlan((plan) => ({
    ...plan,
    tasks: plan.tasks.filter((task) => task.id !== "B"),
  }));
  const before = ready(workspace);
  click();
  assert.deepEqual(ready(workspace).plan, before.plan);
  assert.equal(ready(workspace).dirty, before.dirty);
  assert.match(
    renderToStaticMarkup(editor(workspace)),
    /插入位置对应的任务已不存在/,
  );
});

test("head, tail and add controls retain intervening insertions from the same render", async () => {
  const { workspace } = await setup();
  workspace.updatePlan(objectTaskPlanSchema.parse({}));
  const tree = editor(workspace);
  button(tree, "任务尾部插入")();
  const tail = ready(workspace).plan.tasks[0]?.id;
  assert.ok(tail);
  button(tree, "任务首部插入")();
  const head = ready(workspace).plan.tasks[0]?.id;
  assert.ok(head);
  button(tree, "添加精修")();
  const tasks = ready(workspace).plan.tasks;
  assert.deepEqual(
    tasks.slice(0, 2).map((task) => task.id),
    [head, tail],
  );
  assert.equal(tasks.length, 3);
  assert.equal(tasks[2]?.granularity, "fine");
  assert.deepEqual(
    tasks.map((task) => task.position),
    [0, 1, 2],
  );
});
