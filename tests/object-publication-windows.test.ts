import assert from "node:assert/strict";
import test from "node:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { ObjectPublication } from "../src/ui/object-tasks/object-publication";
import { ObjectPublicationPanel } from "../src/ui/object-tasks/ObjectPublicationPanel";
import {
  PublicationIntentStore,
  loadPublicationIntent,
} from "../src/ui/object-tasks/object-publication-intent";
import {
  publicationRequestSchema,
  type PublicationOperation,
  type PublicationPreview,
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
  files: [],
  paths: [],
  feedback: [],
};
const approval = {
  acceptanceNote: "Reviewed output",
  confirmFiles: true,
  confirmReplacement: false,
  feedback: [],
};
const request = publicationRequestSchema.parse({
  ...review,
  ...approval,
  requestId: "first",
  previewDigest: preview.digest,
});
function operation(state: PublicationOperation["state"]): PublicationOperation {
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
  return {
    values,
    getItem: (key: string) => values.get(key) ?? null,
    setItem: (key: string, value: string) => {
      values.set(key, value);
    },
  };
}

test("stale window cannot replace original intent; reread reconciles without sending", async () => {
  const store = storage();
  const sent: unknown[] = [];
  const api = async (method: string, input: unknown) => {
    if (method === "objectTask.publications") return [];
    if (method === "objectTask.publicationPreview") return preview;
    sent.push(input);
    throw new Error("response lost");
  };
  const first = new ObjectPublication(
    review,
    api,
    async () => {},
    () => "first",
    store,
  );
  const stale = new ObjectPublication(
    review,
    api,
    async () => {},
    () => "stale",
    store,
  );
  await Promise.all([first.refresh(), stale.refresh()]);
  await Promise.all([first.publish(approval), stale.publish(approval)]);
  assert.deepEqual(sent, [request]);
  assert.deepEqual(loadPublicationIntent(review, store)?.request, request);
  assert.equal(stale.getSnapshot().recoveryBlocked, true);
  assert.match(
    renderToStaticMarkup(
      createElement(ObjectPublicationPanel, { session: stale }),
    ),
    /另一窗口已更新原发布请求/,
  );
  stale.restore();
  await stale.retry();
  assert.equal(sent.length, 1);
  await stale.refresh();
  assert.equal(sent.length, 1);
  await stale.retry();
  assert.deepEqual(sent, [request, request]);
});

test("late published response cannot erase another window's abort intent", async () => {
  const store = storage();
  const pending = deferred();
  let published = false;
  const api = async (method: string) => {
    if (method === "objectTask.publications")
      return [operation(published ? "published" : "applying")];
    if (method === "objectTask.publishCandidate") return pending.promise;
    assert.equal(method, "objectTask.abortPublication");
    return operation("aborting");
  };
  const first = new ObjectPublication(
    review,
    api,
    async () => {},
    undefined,
    store,
  );
  await first.refresh();
  const publishing = first.retry();
  // Wait until the request has been durably saved before opening the second window.
  await new Promise<void>((resolve) => setImmediate(resolve));
  const second = new ObjectPublication(
    review,
    api,
    async () => {},
    undefined,
    store,
  );
  await second.refresh();
  await second.abort("first", true);
  assert.equal(loadPublicationIntent(review, store)?.abort, true);
  published = true;
  pending.resolve(operation("published"));
  await publishing;
  assert.equal(first.getSnapshot().recoveryBlocked, true);
  assert.equal(loadPublicationIntent(review, store)?.abort, true);
  await second.refresh();
  assert.equal(loadPublicationIntent(review, store)?.request, null);
});

test("completed intent tombstones reject an older empty snapshot", async () => {
  const store = storage();
  const older = new PublicationIntentStore(review, store);
  const current = new PublicationIntentStore(review, store);
  older.read();
  current.read();
  await current.save(request, false);
  await current.save(null, false);
  const tombstone = [...store.values];
  await assert.rejects(older.save(request, false), /另一窗口/);
  assert.deepEqual([...store.values], tombstone);
});

test("revision-less saved requests remain recoverable and gain a revision on retry", async () => {
  const store = storage();
  const original = new PublicationIntentStore(review, store);
  original.read();
  await original.save(request, false);
  const [name, raw] = [...store.values][0]!;
  const legacy = JSON.parse(raw);
  delete legacy.revision;
  store.setItem(name, JSON.stringify(legacy));
  const reopened = new PublicationIntentStore(review, store);
  assert.deepEqual(reopened.read()?.request, request);
  await reopened.save(request, false);
  assert.ok(loadPublicationIntent(review, store)?.revision);
  assert.deepEqual(loadPublicationIntent(review, store)?.request, request);
});

test("queued browser lock cancellation saves intent but never dispatches; absent locks fail closed", async () => {
  const oldWindow = Object.getOwnPropertyDescriptor(globalThis, "window");
  const queued = deferred();
  const names: string[] = [];
  const store = storage();
  let writes = 0;
  const api = async (method: string) => {
    if (method === "objectTask.publications") return [];
    if (method === "objectTask.publicationPreview") return preview;
    writes++;
    return operation("published");
  };
  try {
    Object.defineProperty(globalThis, "window", {
      configurable: true,
      value: {
        navigator: {
          locks: {
            request: async (name: string, work: () => void) => {
              names.push(name);
              await queued.promise;
              work();
            },
          },
        },
        localStorage: store,
      },
    });
    const session = new ObjectPublication(
      review,
      api,
      async () => {},
      () => "first",
      store,
    );
    await session.refresh();
    const publishing = session.publish(approval);
    session.cancel();
    queued.resolve(undefined);
    await publishing;
    assert.equal(writes, 0);
    assert.equal(names.length, 1);
    assert.match(names[0]!, /beaver.publication-intent.v1:/);
    assert.deepEqual(loadPublicationIntent(review, store)?.request, request);
    Object.defineProperty(globalThis, "window", {
      configurable: true,
      value: { navigator: {}, localStorage: store },
    });
    const reopened = new ObjectPublication(
      review,
      api,
      async () => {},
      undefined,
      store,
    );
    await reopened.refresh();
    await reopened.retry();
    assert.equal(writes, 0);
    assert.match(reopened.getSnapshot().storageError, /不支持跨窗口发布锁/);
  } finally {
    if (oldWindow) Object.defineProperty(globalThis, "window", oldWindow);
    else Reflect.deleteProperty(globalThis, "window");
  }
});
