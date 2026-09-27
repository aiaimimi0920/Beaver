import assert from "node:assert/strict";
import test from "node:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import {
  objectTaskDraftSchema,
  objectTaskPlanSchema,
  unlockObjectTaskDraftSchema,
} from "../src/shared/object-tasks";
import { ObjectTaskWorkspace } from "../src/ui/object-tasks/object-task-workspace";
import { ObjectTaskDraftEditor } from "../src/ui/object-tasks/ObjectTaskDraftEditor";
import { ObjectTaskPlanningPanel } from "../src/ui/object-tasks/ObjectTaskPlanningPanel";
import { objectTaskSnapshot } from "./fixtures/object-tasks";

const plan = objectTaskPlanSchema.parse({
  tasks: [
    {
      id: "draft-coarse",
      granularity: "coarse",
      title: "Submitted",
      prompt: "Make a game",
      acceptance: "Playable",
    },
  ],
});
function fixture() {
  let draft = objectTaskDraftSchema.parse({
    projectId: "p",
    id: "object-task-plan",
    revision: 2,
    planRevision: 0,
    plan,
    committedRequestId: "commit",
  });
  const calls: { method: string; input: unknown }[] = [];
  let lose = false;
  let receipt: typeof draft | null = null;
  const api = async (method: string, input: unknown) => {
    calls.push({ method, input });
    if (method === "objectTask.snapshot") return objectTaskSnapshot();
    if (method === "objectTask.getDraft") return draft;
    if (method === "objectTask.unlockDraft") {
      const request = unlockObjectTaskDraftSchema.parse(input);
      if (!receipt) {
        assert.equal(request.expectedRevision, draft.revision);
        assert.equal(request.expectedPlanRevision, 1);
        draft = objectTaskDraftSchema.parse({
          ...draft,
          revision: 3,
          planRevision: 1,
          committedRequestId: undefined,
          plan: {},
        });
        receipt = draft;
      }
      if (lose) {
        lose = false;
        throw new Error("response lost");
      }
      return receipt;
    }
    throw new Error("Unexpected " + method);
  };
  return {
    calls,
    api,
    workspace: () => new ObjectTaskWorkspace("p", api),
    lose: () => {
      lose = true;
    },
    remoteEdit: () => {
      draft = { ...draft, revision: draft.revision + 1, plan };
    },
    setLocked: (locked: boolean) => {
      draft = {
        ...draft,
        revision: draft.revision + 1,
        committedRequestId: locked ? "commit" : undefined,
        planRevision: 1,
      };
    },
  };
}

test("reopen shows submitted content read-only and only explicit next draft enables editing", async () => {
  const f = fixture();
  const workspace = f.workspace();
  await workspace.refresh();
  const state = workspace.getSnapshot();
  assert.equal(state.kind, "ready");
  if (state.kind !== "ready") return;
  assert.equal(state.committedRequestId, "commit");
  assert.equal(state.conflict, null);
  const before = f.calls.length;
  workspace.updatePlan(objectTaskPlanSchema.parse({}));
  workspace.startNewPlan();
  assert.equal(await workspace.save(), false);
  assert.equal(await workspace.commit(), false);
  assert.equal(await workspace.keepLocalDraftAndRebaseRevision(), false);
  assert.equal(f.calls.length, before);
  assert.deepEqual(workspace.getSnapshot(), state);
  const html = renderToStaticMarkup(
    createElement(ObjectTaskDraftEditor, { state, workspace }),
  );
  assert.match(html, /草稿已提交并锁定/);
  assert.match(html, /<fieldset disabled=""/);
  assert.ok(html.includes("<button>开始下一份草稿</button>"));
  const planning = renderToStaticMarkup(
    createElement(ObjectTaskPlanningPanel, { projectId: "p", state }),
  );
  assert.ok(planning.includes('<button disabled="">开始规划</button>'));
  assert.equal(await workspace.unlockDraft(), true);
  const next = workspace.getSnapshot();
  assert.equal(next.kind, "ready");
  if (next.kind !== "ready") return;
  assert.equal(next.committedRequestId, undefined);
  assert.equal(next.draftRevision, 3);
  assert.equal(next.planRevision, 1);
  assert.deepEqual(next.plan.tasks, []);
  assert.deepEqual(next.snapshot, state.snapshot);
  workspace.updatePlan(plan);
  const edited = workspace.getSnapshot();
  assert.ok(edited.kind === "ready" && edited.dirty);
});

test("lost unlock response replays exact request and preserves subsequent remote edits", async () => {
  const f = fixture();
  const workspace = f.workspace();
  await workspace.refresh();
  f.lose();
  assert.equal(await workspace.unlockDraft(), false);
  const failed = workspace.getSnapshot();
  assert.ok(failed.kind === "ready" && failed.committedRequestId === "commit");
  f.remoteEdit();
  assert.equal(await workspace.unlockDraft(), true);
  const requests = f.calls.filter((c) => c.method === "objectTask.unlockDraft");
  assert.deepEqual(requests[0]?.input, requests[1]?.input);
  const current = workspace.getSnapshot();
  assert.ok(current.kind === "ready");
  assert.equal(current.draftRevision, 4);
  assert.deepEqual(current.plan, plan);
  const reopened = f.workspace();
  await reopened.refresh();
  assert.deepEqual(reopened.getSnapshot(), current);
});

test("a stale editor cannot rebase local edits over a remotely committed draft", async () => {
  const f = fixture();
  f.setLocked(false);
  const workspace = f.workspace();
  await workspace.refresh();
  const local = {
    ...plan,
    tasks: plan.tasks.map((task) => ({ ...task, title: "Local edit" })),
  };
  workspace.updatePlan(local);
  f.setLocked(true);
  await workspace.refresh();
  assert.equal(await workspace.keepLocalDraftAndRebaseRevision(), false);
  const current = workspace.getSnapshot();
  assert.ok(current.kind === "ready");
  assert.deepEqual(current.plan, local);
  assert.match(current.error ?? "", /已提交并锁定/);
  assert.equal(await workspace.save(), false);
  await workspace.reloadDiscardingLocal();
  const reloaded = workspace.getSnapshot();
  assert.ok(
    reloaded.kind === "ready" && reloaded.committedRequestId === "commit",
  );
});
