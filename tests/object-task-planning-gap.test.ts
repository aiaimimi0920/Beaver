import assert from "node:assert/strict";
import test from "node:test";
import { Children, createElement, isValidElement, type ReactNode } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import {
  objectTaskDraftSchema,
  saveObjectTaskDraftSchema,
} from "../src/shared/object-tasks";
import { ObjectTaskDraftEditor } from "../src/ui/object-tasks/ObjectTaskDraftEditor";
import { ObjectTaskDefinitionEditor } from "../src/ui/object-tasks/ObjectTaskDefinitionEditor";
import { ObjectTaskDefinitionComparison } from "../src/ui/object-tasks/ObjectTaskDefinitionComparison";
import { ObjectTaskPlanningField } from "../src/ui/object-tasks/ObjectTaskPlanningField";
import { ObjectTaskBoard } from "../src/ui/object-tasks/ObjectTaskBoard";
import { ObjectTaskWorkspace } from "../src/ui/object-tasks/object-task-workspace";
import {
  editedRevision,
  revisionReceipt,
} from "./fixtures/object-task-revisions";
import { objectTaskSnapshot } from "./fixtures/object-tasks";

function planningChange(
  node: ReactNode,
): ((value: string) => void) | undefined {
  for (const child of Children.toArray(node)) {
    if (
      !isValidElement<{
        children?: ReactNode;
        onChange?: (value: string) => void;
      }>(child)
    )
      continue;
    if (child.type === ObjectTaskPlanningField) return child.props.onChange;
    const found = planningChange(child.props.children);
    if (found) return found;
  }
}

const pending = "Scene integration\nExport requirements";

test("draft pending requirements survive production save and reopen without becoming a completion declaration", async () => {
  let draft = objectTaskDraftSchema.parse({
    projectId: "p",
    id: "object-task-plan",
    revision: 1,
    planRevision: 1,
    plan: {
      tasks: [
        {
          id: "new",
          granularity: "coarse",
          title: "New goal",
          prompt: "Make a game",
          acceptance: "Playable",
        },
      ],
    },
  });
  assert.equal(draft.plan.tasks[0]?.pendingPlanning, "");
  const api = async (method: string, input: unknown) => {
    if (method === "objectTask.snapshot") return objectTaskSnapshot();
    if (method === "objectTask.getDraft") return structuredClone(draft);
    if (method === "objectTask.saveDraft") {
      draft = {
        ...draft,
        revision: draft.revision + 1,
        plan: saveObjectTaskDraftSchema.parse(input).plan,
      };
      return structuredClone(draft);
    }
    throw new Error(method);
  };
  const workspace = new ObjectTaskWorkspace("p", api);
  await workspace.refresh();
  const state = workspace.getSnapshot();
  assert.ok(state.kind === "ready");
  const change = planningChange(ObjectTaskDraftEditor({ state, workspace }));
  assert.ok(change);
  change(pending);
  await workspace.save();
  const reopened = new ObjectTaskWorkspace("p", api);
  await reopened.refresh();
  const loaded = reopened.getSnapshot();
  assert.ok(loaded.kind === "ready");
  assert.equal(loaded.plan.tasks[0]?.pendingPlanning, pending);
  assert.match(
    renderToStaticMarkup(
      createElement(ObjectTaskDraftEditor, {
        state: loaded,
        workspace: reopened,
      }),
    ),
    /Scene integration/,
  );
});

test("revision records planning gaps and fully accepted known children still leave parent progress unknown", async () => {
  const receipt = {
    ...revisionReceipt,
    after: { ...revisionReceipt.after, pendingPlanning: pending },
  };
  let submitted: unknown;
  const session = editedRevision(async (method, input) => {
    if (method === "objectTask.revisions") return [receipt];
    submitted = input;
    return receipt;
  });
  const snapshot = objectTaskSnapshot();
  const view = ObjectTaskDefinitionEditor({
    snapshot,
    taskId: "medium",
    definition: receipt.before,
    reason: receipt.reason,
    onDefinition: (patch) => session.updateDefinition(patch),
    onReason: () => {},
  });
  const change = planningChange(view);
  assert.ok(change);
  change(pending);
  assert.equal(session.review(), true);
  await session.submit();
  assert.equal(
    (submitted as { definition: { pendingPlanning: string } }).definition
      .pendingPlanning,
    pending,
  );
  assert.match(
    renderToStaticMarkup(
      createElement(ObjectTaskDefinitionComparison, {
        before: receipt.before,
        after: receipt.after,
      }),
    ),
    /Scene integration/,
  );
  snapshot.tasks.find((task) => task.id === "medium")!.pendingPlanning =
    pending;
  snapshot.tasks.find((task) => task.id === "fine")!.status = "accepted";
  const html = renderToStaticMarkup(
    createElement(ObjectTaskBoard, {
      snapshot,
      disabled: false,
      execute: () => {},
    }),
  );
  assert.match(html, /必要精修已接受 1 \/ 1/);
  assert.match(html, /尚待规划（Hero）：Scene integration/);
  assert.match(html, /整体进度未知/);
  assert.equal(
    snapshot.tasks.find((task) => task.id === "medium")!.status,
    "planned",
  );
});
