import assert from "node:assert/strict";
import test from "node:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { ObjectPublication } from "../src/ui/object-tasks/object-publication";
import { ObjectPublicationPanel } from "../src/ui/object-tasks/ObjectPublicationPanel";
import {
  loadPublicationIntent,
  PublicationIntentStore,
} from "../src/ui/object-tasks/object-publication-intent";
import {
  publicationRequestSchema,
  type PublicationPreview,
  type PublicationOperation,
} from "../src/shared/object-publication";
import { execution } from "./fixtures/object-attempts";

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
  files: [],
  paths: [],
  feedback: [],
};
const approval = {
  acceptanceNote: "Accepted original",
  confirmFiles: true,
  confirmReplacement: false,
  feedback: [],
};
const request = publicationRequestSchema.parse({
  ...review,
  ...approval,
  requestId: "original",
  previewDigest: preview.digest,
});
function operation(
  state: PublicationOperation["state"] = "applying",
): PublicationOperation {
  return {
    schemaVersion: 1,
    request,
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
function storage() {
  const values = new Map<string, string>();
  let fail = false;
  return {
    values,
    fail: (next: boolean) => {
      fail = next;
    },
    getItem: (key: string) => values.get(key) ?? null,
    setItem: (key: string, value: string) => {
      if (fail) throw new Error("disk full");
      values.set(key, value);
    },
  };
}

async function seedIntent(store: ReturnType<typeof storage>) {
  const intent = new PublicationIntentStore(review, store);
  intent.read();
  await intent.save(request, false);
}

test("reload before journal retains exact publication; read-only reconciliation precedes explicit retry", async () => {
  const store = storage();
  const sent: unknown[] = [];
  const api = async (method: string, input: unknown) => {
    if (method === "objectTask.publications") return [];
    if (method === "objectTask.publicationPreview") return preview;
    sent.push(input);
    assert.deepEqual(loadPublicationIntent(review, store)?.request, input);
    throw new Error("lost before dispatch");
  };
  const first = new ObjectPublication(
    review,
    api,
    async () => {},
    () => "original",
    store,
  );
  await first.refresh();
  await first.publish(approval);
  const reopened = new ObjectPublication(
    {
      ...review,
      reviewRequestId: "new-review",
      target: { ...review.target, attemptId: "new-attempt" },
    },
    api,
    async () => {},
    () => "must-not-use",
    store,
  );
  await reopened.retry();
  assert.equal(sent.length, 1);
  await reopened.refresh();
  assert.equal(sent.length, 1);
  assert.equal(reopened.getSnapshot().preview, null);
  await reopened.publish({ ...approval, acceptanceNote: "changed" });
  assert.deepEqual(sent, [request, request]);
  assert.match(
    renderToStaticMarkup(
      createElement(ObjectPublicationPanel, { session: reopened }),
    ),
    /已恢复原发布或中止请求/,
  );
});

test("uncertain abort reload never reverts to publish; terminal query clears it", async () => {
  const store = storage();
  const writes: string[] = [];
  let completed = false;
  const api = async (method: string) => {
    if (method === "objectTask.publications")
      return [operation(completed ? "aborted" : "applying")];
    writes.push(method);
    assert.equal(loadPublicationIntent(review, store)?.abort, true);
    throw new Error("lost abort");
  };
  const first = new ObjectPublication(
    review,
    api,
    async () => {},
    undefined,
    store,
  );
  await first.refresh();
  await first.abort("original", true);
  const second = new ObjectPublication(
    review,
    api,
    async () => {},
    undefined,
    store,
  );
  await second.refresh();
  await second.retry();
  assert.deepEqual(writes, [
    "objectTask.abortPublication",
    "objectTask.abortPublication",
  ]);
  completed = true;
  await second.refresh();
  assert.equal(loadPublicationIntent(review, store)?.request, null);
  assert.equal(second.getSnapshot().retry, false);
});

test("failed intent write sends nothing, retains ID, and failed terminal clearing requires query", async () => {
  const store = storage();
  let writes = 0;
  let completed = false;
  const api = async (method: string) => {
    if (method === "objectTask.publications")
      return completed ? [operation("published")] : [];
    if (method === "objectTask.publicationPreview") return preview;
    writes++;
    completed = true;
    store.fail(true);
    return operation("published");
  };
  const session = new ObjectPublication(
    review,
    api,
    async () => {},
    () => "original",
    store,
  );
  await session.refresh();
  store.fail(true);
  await session.publish(approval);
  assert.equal(writes, 0);
  assert.equal(session.getSnapshot().retry, true);
  store.fail(false);
  await session.retry();
  assert.equal(writes, 1);
  assert.equal(session.getSnapshot().reconciled, false);
  await session.retry();
  assert.equal(writes, 1);
  store.fail(false);
  const reopened = new ObjectPublication(
    review,
    api,
    async () => {},
    undefined,
    store,
  );
  await reopened.refresh();
  assert.equal(writes, 1);
  assert.equal(loadPublicationIntent(review, store)?.request, null);
  assert.equal(reopened.getSnapshot().operations[0]?.state, "published");
});

test("corrupt and foreign intent blocks overwrite; read retry restores; project and run are isolated", async () => {
  const store = storage();
  await seedIntent(store);
  const [key, original] = [...store.values][0]!;
  for (const broken of [
    "",
    "{",
    JSON.stringify({
      ...JSON.parse(original),
      request: { ...request, projectId: "foreign" },
    }),
  ]) {
    store.values.set(key, broken);
    let calls = 0;
    const session = new ObjectPublication(
      review,
      async () => {
        calls++;
        return [];
      },
      async () => {},
      undefined,
      store,
    );
    await session.refresh();
    await session.publish(approval);
    await session.retry();
    assert.equal(calls, 0);
    assert.equal(store.values.get(key), broken);
    assert.equal(session.getSnapshot().recoveryBlocked, true);
    assert.match(
      renderToStaticMarkup(createElement(ObjectPublicationPanel, { session })),
      /重试读取原发布请求/,
    );
    store.values.set(key, original);
    session.restore();
    await session.refresh();
    assert.equal(session.getSnapshot().recoveryBlocked, false);
  }
  assert.equal(
    loadPublicationIntent({ ...review, projectId: "other" }, store),
    null,
  );
  assert.equal(
    loadPublicationIntent(
      { ...review, target: { ...review.target, runId: "other" } },
      store,
    ),
    null,
  );
});

test("server aborting receipt changes subsequent retry to abort", async () => {
  const store = storage();
  const writes: string[] = [];
  const session = new ObjectPublication(
    review,
    async (method) => {
      if (method === "objectTask.publications") return [];
      if (method === "objectTask.publicationPreview") return preview;
      writes.push(method);
      return operation(
        method === "objectTask.publishCandidate" ? "aborting" : "aborted",
      );
    },
    async () => {},
    () => "original",
    store,
  );
  await session.refresh();
  await session.publish(approval);
  assert.equal(loadPublicationIntent(review, store)?.abort, true);
  await session.retry();
  assert.deepEqual(writes, [
    "objectTask.publishCandidate",
    "objectTask.abortPublication",
  ]);
  assert.equal(loadPublicationIntent(review, store)?.request, null);
});

test("foreign pending or conflicting receipt cannot replace persisted request or enable retry", async () => {
  for (const conflict of [
    { ...operation(), request: { ...request, requestId: "other" } },
    { ...operation(), request: { ...request, acceptanceNote: "different" } },
  ]) {
    const store = storage();
    await seedIntent(store);
    let writes = 0;
    const session = new ObjectPublication(
      review,
      async (method) => {
        if (method === "objectTask.publications") return [conflict];
        writes++;
        return operation();
      },
      async () => {},
      undefined,
      store,
    );
    await session.refresh();
    await session.retry();
    assert.equal(writes, 0);
    assert.equal(session.getSnapshot().reconciled, false);
    assert.deepEqual(loadPublicationIntent(review, store)?.request, request);
  }
});
