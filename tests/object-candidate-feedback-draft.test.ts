import assert from "node:assert/strict";
import test from "node:test";
import { ObjectCandidateFeedbackDraft } from "../src/ui/object-tasks/object-candidate-feedback-draft";
import { ObjectPublicationDeferred } from "../src/ui/object-tasks/object-publication-deferred";
import { execution } from "./fixtures/object-attempts";
const review = {
  projectId: "p",
  target: execution("awaitingGate").attempt.target,
  reviewRequestId: "review",
};
function storage() {
  const values = new Map<string, string>();
  return {
    values,
    getItem: (key: string) => values.get(key) ?? null,
    setItem: (key: string, value: string) => {
      values.set(key, value);
    },
    removeItem: (key: string) => {
      values.delete(key);
    },
  };
}
const image = {
  path: "preview.png",
  sha256: "c".repeat(64),
  regions: [
    { x: 0.1, y: 0.2, width: 0.3, height: 0.4, prompt: "黄色".repeat(300) },
  ],
};
test("feedback drafts preserve both routes, incomplete regional text and exact review identity", () => {
  const disk = storage();
  const draft = new ObjectCandidateFeedbackDraft(review, disk);
  draft.edit({
    route: "later",
    feedback: "Move label",
    title: "Labels",
    acceptance: "Readable",
    image,
  });
  const reopened = new ObjectCandidateFeedbackDraft(review, disk);
  assert.deepEqual(reopened.getSnapshot().draft, draft.getSnapshot().draft);
  reopened.edit({ route: "current" });
  assert.equal(
    new ObjectCandidateFeedbackDraft(review, disk).getSnapshot().draft.image
      ?.regions[0]?.prompt,
    image.regions[0]!.prompt,
  );
  for (const other of [
    { ...review, projectId: "other" },
    { ...review, reviewRequestId: "new" },
    { ...review, target: { ...review.target, attemptId: "new" } },
  ]) {
    assert.equal(
      new ObjectCandidateFeedbackDraft(other, disk).getSnapshot().draft
        .feedback,
      "",
    );
  }
});
test("failed writes retain input for retry; corrupt and foreign drafts cannot be overwritten", () => {
  const disk = storage();
  let fail = true;
  const draft = new ObjectCandidateFeedbackDraft(review, {
    ...disk,
    setItem(key, value) {
      if (fail) throw Error("quota");
      disk.setItem(key, value);
    },
  });
  draft.edit({ feedback: "Keep this" });
  assert.match(draft.getSnapshot().error, /quota/);
  assert.equal(draft.getSnapshot().draft.feedback, "Keep this");
  fail = false;
  assert.equal(draft.persist(), true);
  const key = [...disk.values.keys()][0]!;
  const good = disk.getItem(key)!;
  for (const raw of [
    "{broken",
    JSON.stringify({
      ...JSON.parse(good),
      review: { ...review, projectId: "foreign" },
    }),
  ]) {
    disk.setItem(key, raw);
    const corrupt = new ObjectCandidateFeedbackDraft(review, disk);
    corrupt.edit({ feedback: "overwrite" });
    assert.equal(corrupt.persist(), false);
    assert.equal(disk.getItem(key), raw);
    disk.setItem(key, good);
    corrupt.restore();
    assert.equal(corrupt.getSnapshot().blocked, false);
    assert.equal(corrupt.getSnapshot().draft.feedback, "Keep this");
  }
});
test("publication conflict preserves draft and immutable deferred target across reopen", async () => {
  const disk = storage();
  const draft = new ObjectCandidateFeedbackDraft(review, disk);
  draft.edit({
    route: "later",
    feedback: "Original",
    title: "Next",
    acceptance: "Works",
    previewFrame: { runId: "preview-run", frameId: "saved-frame" },
  });
  const sent: unknown[] = [];
  const api = async (_: string, input: unknown) => {
    sent.push(input);
    throw Error("OBJECT_PUBLICATION_PENDING");
  };
  await new ObjectPublicationDeferred(
    review,
    api,
    async () => {},
    () => "original",
    disk,
  ).save({
    feedback: "Original",
    later: { title: "Next", acceptance: "Works" },
    previewFrame: draft.getSnapshot().draft.previewFrame,
  });
  const restored = new ObjectCandidateFeedbackDraft(review, disk);
  assert.deepEqual(restored.getSnapshot().draft, draft.getSnapshot().draft);
  restored.edit({ feedback: "New edit" });
  await new ObjectPublicationDeferred(
    review,
    api,
    async () => {},
    () => "wrong",
    disk,
  ).save({
    feedback: "New edit",
    later: { title: "Other", acceptance: "Other" },
  });
  assert.deepEqual(sent[0], sent[1]);
  assert.equal(
    new ObjectCandidateFeedbackDraft(review, disk).getSnapshot().draft.feedback,
    "New edit",
  );
});
