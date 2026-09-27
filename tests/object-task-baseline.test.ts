import assert from "node:assert/strict";
import test from "node:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import type { ObjectBaseline } from "../src/shared/object-framework";
import {
  objectTaskPlanSchema,
  objectTaskProposalSchema,
  saveObjectTaskDraftSchema,
} from "../src/shared/object-tasks";
import { ObjectTaskDraftEditor } from "../src/ui/object-tasks/ObjectTaskDraftEditor";
import { ObjectTaskWorkspace } from "../src/ui/object-tasks/object-task-workspace";
import { objectTaskSnapshot } from "./fixtures/object-tasks";

const medium = {
  id: "new-medium",
  granularity: "medium",
  title: "Refine the hero",
  prompt: "Improve movement.",
  acceptance: "The hero moves smoothly.",
  objectId: "hero",
};
const policies: ObjectBaseline[] = [
  { basePolicy: "latestAccepted" },
  { basePolicy: "pinnedVersion", selectedVersionId: "hero-v1" },
  { basePolicy: "empty" },
];

function ready(workspace: ObjectTaskWorkspace) {
  const state = workspace.getSnapshot();
  assert.ok(state.kind === "ready");
  return state;
}

function harness() {
  let draft = {
    projectId: "p",
    id: "object-task-plan",
    revision: 1,
    planRevision: 1,
    plan: objectTaskPlanSchema.parse({ tasks: [medium] }),
  };
  const invoke = async (method: string, input?: unknown): Promise<unknown> => {
    if (method === "objectTask.snapshot") return objectTaskSnapshot();
    if (method === "objectTask.getDraft") return draft;
    if (method === "objectTask.saveDraft") {
      const request = saveObjectTaskDraftSchema.parse(input);
      assert.equal(request.expectedRevision, draft.revision);
      assert.equal(request.expectedPlanRevision, draft.planRevision);
      draft = { ...draft, revision: draft.revision + 1, plan: request.plan };
      return draft;
    }
    throw new Error(`Unexpected method: ${method}`);
  };
  return { workspace: new ObjectTaskWorkspace("p", invoke), invoke };
}

test("medium baselines preserve explicit choices and legacy omission", () => {
  const legacy = objectTaskProposalSchema.parse(medium);
  assert.equal(Object.hasOwn(legacy, "baseline"), false);
  for (const baseline of policies) {
    const task = objectTaskProposalSchema.parse({ ...medium, baseline });
    assert.deepEqual(task.baseline, baseline);
  }
  for (const baseline of [
    { basePolicy: "unknown" },
    { basePolicy: "pinnedVersion", selectedVersionId: "" },
    { basePolicy: "pinnedVersion", selectedVersionId: "../foreign" },
    { basePolicy: "latestAccepted", selectedVersionId: "hero-v1" },
    { basePolicy: "empty", resolvedVersionId: "forged" },
    null,
  ]) {
    assert.equal(
      objectTaskProposalSchema.safeParse({ ...medium, baseline }).success,
      false,
    );
  }
  for (const granularity of ["coarse", "fine"]) {
    assert.equal(
      objectTaskProposalSchema.safeParse({
        ...medium,
        granularity,
        baseline: { basePolicy: "empty" },
      }).success,
      false,
    );
  }
});

test("baseline edits participate in draft revisions and survive a new workspace", async () => {
  const { workspace, invoke } = harness();
  await workspace.refresh();
  for (const baseline of policies) {
    const before = ready(workspace);
    workspace.updatePlan({
      ...before.plan,
      tasks: before.plan.tasks.map((task) => ({ ...task, baseline })),
    });
    assert.equal(ready(workspace).dirty, true);
    assert.equal(await workspace.save(), true);
    assert.equal(ready(workspace).dirty, false);
    const restored = new ObjectTaskWorkspace("p", invoke);
    await restored.refresh();
    assert.deepEqual(ready(restored).plan.tasks[0]?.baseline, baseline);
  }
});

test("draft editor shows medium baseline and pinned ID with the saved selection", async () => {
  const { workspace } = harness();
  await workspace.refresh();
  const render = () =>
    renderToStaticMarkup(
      createElement(ObjectTaskDraftEditor, {
        state: ready(workspace),
        workspace,
      }),
    );
  assert.match(render(), /领取任务时固定最新已接受版本/);
  assert.match(render(), /value="latestAccepted" selected=""/);
  for (const baseline of policies) {
    workspace.updatePlan(
      objectTaskPlanSchema.parse({ tasks: [{ ...medium, baseline }] }),
    );
    const html = render();
    assert.match(html, /工作基准/);
    assert.ok(html.includes(`value="${baseline.basePolicy}" selected=""`));
    if (baseline.basePolicy === "pinnedVersion") {
      assert.match(html, /已接受版本 ID/);
      assert.match(html, /value="hero-v1"/);
    } else {
      assert.doesNotMatch(html, /已接受版本 ID/);
    }
  }
  for (const granularity of ["coarse", "fine"]) {
    workspace.updatePlan(
      objectTaskPlanSchema.parse({ tasks: [{ ...medium, granularity }] }),
    );
    assert.doesNotMatch(render(), /工作基准/);
  }
});
