import assert from "node:assert/strict";
import test from "node:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { ObjectPublicationApprovalDraft } from "../src/ui/object-tasks/object-publication-approval-draft";
import { ObjectPublication } from "../src/ui/object-tasks/object-publication";
import { ObjectPublicationPanel } from "../src/ui/object-tasks/ObjectPublicationPanel";
import type { PublicationPreview } from "../src/shared/object-publication";
import { execution } from "./fixtures/object-attempts";

const preview: PublicationPreview = {
  review: {
    projectId: "p",
    target: execution("awaitingGate").attempt.target,
    reviewRequestId: "review",
  },
  digest: "a".repeat(64),
  outputDigest: "b".repeat(64),
  baselineVersionId: null,
  acceptedVersionId: null,
  objectRevision: 1,
  replacementRequired: true,
  files: [],
  paths: [],
  feedback: [
    { requestId: "feedback", attemptId: "old", feedback: "Fix light" },
  ],
};
const fields = {
  note: "Reviewed the final candidate",
  decisions: {
    feedback: { resolution: "resolved" as const, note: "Light fixed" },
  },
};
function storage() {
  const values = new Map<string, string>();
  return {
    values,
    getItem: (key: string) => values.get(key) ?? null,
    setItem: (key: string, value: string) => {
      values.set(key, value);
    },
  };
}

test("reopen restores partial decisions only for the exact project, review and preview", () => {
  const port = storage();
  const draft = new ObjectPublicationApprovalDraft(preview, port);
  draft.edit({
    decisions: { feedback: { resolution: "", note: "Still checking" } },
  });
  const reopened = new ObjectPublicationApprovalDraft(preview, port);
  assert.deepEqual(reopened.getSnapshot().draft, draft.getSnapshot().draft);
  assert.equal(reopened.getSnapshot().restored, true);
  for (const other of [
    { ...preview, digest: "c".repeat(64) },
    { ...preview, review: { ...preview.review, projectId: "other" } },
    { ...preview, review: { ...preview.review, reviewRequestId: "other" } },
    {
      ...preview,
      review: {
        ...preview.review,
        target: { ...preview.review.target, attemptId: "other" },
      },
    },
  ]) {
    const isolated = new ObjectPublicationApprovalDraft(other, port);
    assert.equal(isolated.getSnapshot().restored, false);
    assert.deepEqual(isolated.getSnapshot().draft, { note: "", decisions: {} });
  }
  assert.equal(
    port.values.size,
    1,
    "old draft must remain available at its original identity",
  );
});

test("corrupt and foreign records block edits without overwriting evidence", () => {
  const port = storage();
  new ObjectPublicationApprovalDraft(preview, port).edit(fields);
  const [key, original] = [...port.values.entries()][0]!;
  const saved = JSON.parse(original);
  for (const bad of [
    "{broken",
    JSON.stringify({ ...saved, previewDigest: "other" }),
    JSON.stringify({
      ...saved,
      draft: { ...fields, decisions: { unknown: fields.decisions.feedback } },
    }),
    JSON.stringify({
      ...saved,
      draft: {
        ...fields,
        decisions: {
          feedback: { resolution: "deferred", note: "wrong route" },
        },
      },
    }),
  ]) {
    port.values.set(key, bad);
    const reopened = new ObjectPublicationApprovalDraft(preview, port);
    assert.equal(reopened.getSnapshot().blocked, true);
    reopened.edit({ note: "overwrite" });
    assert.equal(reopened.persist(), false);
    assert.equal(port.values.get(key), bad);
    port.values.set(key, original);
    reopened.restore();
    assert.equal(reopened.getSnapshot().blocked, false);
    assert.deepEqual(reopened.getSnapshot().draft, fields);
  }
});

test("failed writes retain current input across refresh and recover without submitting", async () => {
  const port = storage();
  let fail = true;
  let calls = 0;
  const session = new ObjectPublication(
    preview.review,
    async (method) => {
      calls++;
      if (method === "objectTask.publications") return [];
      assert.equal(method, "objectTask.publicationPreview");
      return preview;
    },
    async () => {},
    () => "unused",
    {
      ...port,
      setItem: (key, value) => {
        if (fail) throw new Error("quota");
        port.setItem(key, value);
      },
    },
  );
  await session.refresh();
  const draft = session.approvalFor(preview);
  draft.edit(fields);
  assert.match(draft.getSnapshot().error, /quota/);
  await session.refresh();
  assert.equal(session.approvalFor(preview), draft);
  assert.deepEqual(draft.getSnapshot().draft, fields);
  const html = renderToStaticMarkup(
    createElement(ObjectPublicationPanel, { session }),
  );
  assert.match(html, /重试保存发布草稿/);
  assert.match(html, /disabled="">接受并发布对象/);
  fail = false;
  assert.equal(draft.persist(), true);
  assert.equal(draft.getSnapshot().error, "");
  assert.deepEqual(
    new ObjectPublicationApprovalDraft(preview, port).getSnapshot().draft,
    fields,
  );
  assert.equal(calls, 4);
});

test("production form restores text but never restores file or replacement confirmations", async () => {
  const port = storage();
  new ObjectPublicationApprovalDraft(preview, port).edit(fields);
  const calls: string[] = [];
  const session = new ObjectPublication(
    preview.review,
    async (method) => {
      calls.push(method);
      return method === "objectTask.publications" ? [] : preview;
    },
    async () => {},
    () => "unused",
    port,
  );
  await session.refresh();
  const html = renderToStaticMarkup(
    createElement(ObjectPublicationPanel, { session }),
  );
  assert.match(html, /已恢复当前发布草稿/);
  assert.match(html, /Reviewed the final candidate/);
  assert.match(html, /Light fixed/);
  assert.match(html, /value="resolved" selected=""/);
  assert.doesNotMatch(html, /checked=""/);
  assert.match(html, /disabled="">接受并发布对象/);
  assert.deepEqual(calls, [
    "objectTask.publications",
    "objectTask.publicationPreview",
  ]);
});
