import assert from "node:assert/strict";
import test from "node:test";
import { Children, isValidElement, type ReactNode } from "react";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import {
  objectTaskDraftSchema,
  objectTaskRecordSchema,
  saveObjectTaskDraftSchema,
} from "../src/shared/object-tasks";
import { objectTaskDefinitionSchema } from "../src/shared/object-task-revisions";
import { ObjectTaskDraftEditor } from "../src/ui/object-tasks/ObjectTaskDraftEditor";
import { ObjectTaskDefinitionEditor } from "../src/ui/object-tasks/ObjectTaskDefinitionEditor";
import { ObjectTaskDefinitionComparison } from "../src/ui/object-tasks/ObjectTaskDefinitionComparison";
import { ObjectTaskRequirementField } from "../src/ui/object-tasks/ObjectTaskRequirementField";
import { ObjectTaskWorkspace } from "../src/ui/object-tasks/object-task-workspace";
import {
  editedRevision,
  revisionReceipt,
} from "./fixtures/object-task-revisions";
import { objectTaskSnapshot } from "./fixtures/object-tasks";

function scopeChange(
  node: ReactNode,
): (value: "required" | "optional") => void {
  for (const child of Children.toArray(node)) {
    if (
      !isValidElement<{
        children?: ReactNode;
        onChange?: (value: "required" | "optional") => void;
      }>(child)
    )
      continue;
    if (child.type === ObjectTaskRequirementField) return child.props.onChange!;
    try {
      return scopeChange(child.props.children);
    } catch {
      /* Search siblings. */
    }
  }
  throw new Error("Requirement control missing");
}

test("old API records and definitions default to required while invalid classification is rejected", () => {
  const { requirement: _, ...old } = objectTaskSnapshot().tasks[0]!;
  assert.equal(objectTaskRecordSchema.parse(old).requirement, "required");
  assert.equal(
    objectTaskDefinitionSchema.parse({
      title: "Title",
      prompt: "Goal",
      acceptance: "",
      dependsOn: [],
    }).requirement,
    "required",
  );
  assert.equal(
    objectTaskRecordSchema.safeParse({ ...old, requirement: "ignored" })
      .success,
    false,
  );
});

test("draft classification control saves and survives reopening the production workspace", async () => {
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
  const api = async (method: string, input: unknown) => {
    if (method === "objectTask.snapshot") return objectTaskSnapshot();
    if (method === "objectTask.getDraft") return structuredClone(draft);
    if (method === "objectTask.saveDraft") {
      const request = saveObjectTaskDraftSchema.parse(input);
      draft = { ...draft, revision: draft.revision + 1, plan: request.plan };
      return structuredClone(draft);
    }
    throw new Error(method);
  };
  const workspace = new ObjectTaskWorkspace("p", api);
  await workspace.refresh();
  const state = workspace.getSnapshot();
  assert.ok(state.kind === "ready");
  const view = ObjectTaskDraftEditor({ state, workspace });
  scopeChange(view)("optional");
  await workspace.save();
  const reopened = new ObjectTaskWorkspace("p", api);
  await reopened.refresh();
  const loaded = reopened.getSnapshot();
  assert.ok(loaded.kind === "ready");
  assert.equal(loaded.plan.tasks[0]?.requirement, "optional");
  const html = renderToStaticMarkup(
    createElement(ObjectTaskDraftEditor, {
      state: loaded,
      workspace: reopened,
    }),
  );
  assert.match(html, /value="optional" selected=""/);
});

test("revision control includes classification in reviewed request and immutable receipt", async () => {
  const receipt = {
    ...revisionReceipt,
    after: { ...revisionReceipt.after, requirement: "optional" as const },
  };
  let submitted: unknown;
  const session = editedRevision(async (method, input) => {
    if (method === "objectTask.revisions") return [receipt];
    submitted = input;
    return receipt;
  });
  const view = ObjectTaskDefinitionEditor({
    snapshot: objectTaskSnapshot(),
    taskId: "medium",
    definition: revisionReceipt.after,
    reason: receipt.reason,
    onDefinition: (patch) => session.updateDefinition(patch),
    onReason: () => {},
  });
  scopeChange(view)("optional");
  assert.equal(session.review(), true);
  await session.submit();
  assert.equal(
    (submitted as { definition: { requirement: string } }).definition
      .requirement,
    "optional",
  );
  const html = renderToStaticMarkup(
    createElement(ObjectTaskDefinitionComparison, {
      before: receipt.before,
      after: receipt.after,
    }),
  );
  assert.match(html, /必要工作/);
  assert.match(html, /可选工作/);
});
