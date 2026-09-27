import assert from "node:assert/strict";
import test from "node:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { ObjectPublication } from "../src/ui/object-tasks/object-publication";
import { ObjectPublicationPanel } from "../src/ui/object-tasks/ObjectPublicationPanel";
import {
  publicationRequestSchema,
  type PublicationPreview,
  type PublicationOperation,
} from "../src/shared/object-publication";
import { execution, deferred } from "./fixtures/object-attempts";
const review = {
  projectId: "p",
  target: execution("awaitingGate").attempt.target,
  reviewRequestId: "review",
};
const preview: PublicationPreview = {
  review,
  digest: "a".repeat(64),
  outputDigest: "b".repeat(64),
  baselineVersionId: null,
  acceptedVersionId: null,
  objectRevision: 1,
  replacementRequired: false,
  files: [{ path: "result.txt", role: "source" }],
  paths: [{ path: "result.txt", after: "c".repeat(64) }],
  feedback: [],
};
const approval = {
  acceptanceNote: "Reviewed output",
  confirmFiles: true,
  confirmReplacement: false,
  feedback: [],
};
function operation(
  input: unknown,
  state: PublicationOperation["state"] = "applying",
): PublicationOperation {
  return {
    schemaVersion: 1,
    request: publicationRequestSchema.parse(input),
    preview,
    versionId: "v1",
    state,
    error: null,
    result:
      state === "published"
        ? {
            versionId: "v1",
            objectRevision: 2,
            taskRevision: 2,
            runRevision: 2,
            planRevision: 2,
          }
        : null,
  };
}
function pending() {
  return operation({
    ...review,
    ...approval,
    requestId: "pub",
    previewDigest: preview.digest,
  });
}

test("lost publication response retries immutable request and refreshes consumers", async () => {
  const sent: unknown[] = [];
  let changed = 0;
  const session = new ObjectPublication(
    review,
    async (method, input) => {
      if (method === "objectTask.publications") return [];
      if (method === "objectTask.publicationPreview") return preview;
      sent.push(input);
      if (sent.length === 1) throw new Error("lost response");
      return operation(input, "published");
    },
    async () => {
      changed++;
    },
    () => "pub",
  );
  await session.refresh();
  await session.publish(approval);
  assert.equal(session.getSnapshot().retry, true);
  await session.publish({ ...approval, acceptanceNote: "changed" });
  assert.deepEqual(sent[0], sent[1]);
  assert.equal(changed, 1);
  assert.equal(session.getSnapshot().operations[0]?.state, "published");
  assert.match(
    renderToStaticMarkup(createElement(ObjectPublicationPanel, { session })),
    /已发布，对象占用已释放/,
  );
});

test("reopen only queries; uncertain abort survives refresh of applying journal", async () => {
  const calls: string[] = [];
  let aborts = 0;
  const session = new ObjectPublication(
    review,
    async (method) => {
      calls.push(method);
      if (method === "objectTask.publications") return [pending()];
      assert.equal(method, "objectTask.abortPublication");
      if (++aborts === 1) throw new Error("connection lost before dispatch");
      return { ...pending(), state: "aborted" };
    },
    async () => {},
    () => "unused",
  );
  await session.refresh();
  assert.deepEqual(calls, ["objectTask.publications"]);
  await session.abort("pub", true);
  await session.refresh();
  await session.retry();
  assert.equal(aborts, 2);
  assert.ok(!calls.includes("objectTask.publishCandidate"));
  assert.equal(session.getSnapshot().retry, false);
});

test("cancel ignores late response and foreign receipt retains retry", async () => {
  const response = deferred<unknown>();
  const session = new ObjectPublication(
    review,
    async (method) =>
      method === "objectTask.publications"
        ? []
        : method === "objectTask.publicationPreview"
          ? preview
          : response.promise,
    async () => {},
    () => "pub",
  );
  await session.refresh();
  const sending = session.publish(approval);
  session.cancel();
  response.resolve({ ...pending(), state: "aborted" });
  await sending;
  assert.deepEqual(session.getSnapshot().operations, []);
  assert.equal(session.getSnapshot().retry, true);
  const other = new ObjectPublication(
    review,
    async (method) =>
      method === "objectTask.publications"
        ? []
        : method === "objectTask.publicationPreview"
          ? preview
          : {
              ...pending(),
              request: { ...pending().request, requestId: "foreign" },
            },
    async () => {},
    () => "pub",
  );
  await other.refresh();
  await other.publish(approval);
  assert.equal(other.getSnapshot().retry, true);
  assert.match(other.getSnapshot().error, /回执/);
});

test("replacement and every feedback require explicit owner decisions", async () => {
  let writes = 0;
  const session = new ObjectPublication(
    review,
    async (method) => {
      if (method === "objectTask.publications") return [];
      if (method === "objectTask.publicationPreview")
        return {
          ...preview,
          replacementRequired: true,
          feedback: [
            { requestId: "rework", attemptId: "old", feedback: "Fix movement" },
          ],
        };
      writes++;
      throw new Error("not expected");
    },
    async () => {},
  );
  await session.refresh();
  await session.publish(approval);
  assert.equal(writes, 0);
  await session.publish({ ...approval, confirmReplacement: true });
  assert.equal(writes, 0);
  const html = renderToStaticMarkup(
    createElement(ObjectPublicationPanel, { session }),
  );
  assert.match(html, /result.txt/);
  assert.match(html, /Fix movement/);
  assert.match(html, /历史或空基线/);
});

test("query recovers completed publication and refreshes consumers without mutation", async () => {
  let changed = 0;
  const session = new ObjectPublication(
    review,
    async (method) => {
      assert.equal(method, "objectTask.publications");
      return [operation(pending().request, "published")];
    },
    async () => {
      changed++;
    },
  );
  await session.refresh();
  assert.equal(changed, 1);
  assert.equal(session.getSnapshot().preview, null);
  assert.equal(session.getSnapshot().retry, false);
});

test("published feedback reopens original attempt evidence with isolated cancellable reads", async () => {
  const image = {
    path: "preview.png",
    sha256: "c".repeat(64),
    width: 400,
    height: 200,
    regions: [{ x: 0.1, y: 0.2, width: 0.5, height: 0.5, prompt: "Brighten" }],
  };
  const feedback = {
    requestId: "rework",
    attemptId: "old-attempt",
    feedback: "Fix light",
    image,
  };
  const op = operation(pending().request, "published");
  op.preview = { ...preview, feedback: [feedback] };
  const reads: unknown[] = [];
  const late = deferred<unknown>();
  const session = new ObjectPublication(
    review,
    async (method, input) => {
      if (method === "objectTask.publications") return [op];
      assert.equal(method, "objectTask.attemptFile");
      reads.push(input);
      if (reads.length === 1) return late.promise;
      return { request: input, byteCount: 1, content: { kind: "binary" } };
    },
    async () => {},
  );
  await session.refresh();
  assert.equal(reads.length, 0);
  const html = renderToStaticMarkup(
    createElement(ObjectPublicationPanel, { session }),
  );
  assert.match(html, /Fix light/);
  assert.match(html, /Brighten/);
  assert.match(html, /回看原始图片与编号/);
  const first = session.createFeedbackFile(feedback);
  const second = session.createFeedbackFile(feedback);
  const pendingRead = first.open("output", image.path, image.sha256);
  await second.open("output", image.path, image.sha256);
  assert.deepEqual(reads[0], {
    projectId: review.projectId,
    runId: review.target.runId,
    attemptId: "old-attempt",
    checkpoint: "output",
    path: image.path,
    sha256: image.sha256,
  });
  first.cancel();
  late.resolve({
    request: reads[0],
    byteCount: 1,
    content: { kind: "binary" },
  });
  await pendingRead;
  assert.equal(first.getSnapshot().response, null);
  assert.equal(second.getSnapshot().response?.content.kind, "binary");
});
