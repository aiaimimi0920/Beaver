import assert from "node:assert/strict";
import test from "node:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { ObjectPublicationFollowup } from "../src/ui/object-tasks/object-publication-followup";
import { ObjectPublicationFollowupPanel } from "../src/ui/object-tasks/ObjectPublicationFollowupPanel";
import { publicationFollowupRequestSchema } from "../src/shared/object-publication-followup";
import type { PublicationOperation } from "../src/shared/object-publication";
import { execution, deferred } from "./fixtures/object-attempts";

const target = execution("awaitingGate").attempt.target;
const review = { projectId: "p", target, reviewRequestId: "review" };
const op: PublicationOperation = {
  schemaVersion: 1,
  state: "published",
  error: null,
  versionId: "v1",
  request: {
    ...review,
    requestId: "pub",
    previewDigest: "a".repeat(64),
    acceptanceNote: "ok",
    confirmFiles: true,
    confirmReplacement: false,
    feedback: [],
  },
  preview: {
    review,
    digest: "a".repeat(64),
    outputDigest: "b".repeat(64),
    baselineVersionId: null,
    acceptedVersionId: null,
    objectRevision: 0,
    replacementRequired: false,
    files: [],
    paths: [],
    feedback: [],
  },
  result: {
    versionId: "v1",
    objectRevision: 1,
    taskRevision: 1,
    runRevision: 1,
    planRevision: 1,
  },
};
const draft = {
  title: "Improve movement",
  feedback: "Reduce speed near walls",
  acceptance: "Stable motion near walls",
  previewFrame: { runId: "preview-run", frameId: "frozen-frame" },
};
function draftStorage() {
  const data = new Map<string, string>();
  return {
    data,
    getItem: (key: string) => data.get(key) ?? null,
    setItem: (key: string, value: string) => {
      data.set(key, value);
    },
  };
}
function receipt(input: unknown) {
  return {
    request: publicationFollowupRequestSchema.parse(input),
    source: target,
    mediumTaskId: "later",
    fineTaskId: "fine",
    runId: "later-run",
    planRevision: 2,
  };
}

test("lost response preserves draft, retries identical request and refreshes tasks once", async () => {
  const sent: unknown[] = [];
  let changes = 0;
  const session = new ObjectPublicationFollowup(
    op,
    async (method, input) => {
      assert.equal(method, "objectTask.createPublicationFollowup");
      sent.push(input);
      if (sent.length === 1) throw new Error("lost response");
      return receipt(input);
    },
    async () => {
      changes++;
    },
    () => "command",
  );
  session.edit(draft);
  await session.create();
  assert.equal(session.getSnapshot().retry, true);
  session.edit({ feedback: "must not replace uncertain request" });
  assert.deepEqual(session.getSnapshot().draft, draft);
  await session.create();
  assert.deepEqual(sent[0], sent[1]);
  assert.equal(changes, 1);
  assert.equal(session.getSnapshot().receipts.length, 1);
  assert.equal(session.getSnapshot().draft.feedback, "");
  const html = renderToStaticMarkup(
    createElement(ObjectPublicationFollowupPanel, { session }),
  );
  assert.match(html, /later/);
  assert.match(html, /Stable motion near walls/);
  assert.match(html, /v1/);
  assert.match(html, /frozen-frame/);
});

test("cancel ignores late response; reopen recovers committed receipt without another mutation", async () => {
  const pending = deferred<unknown>();
  let sent: unknown;
  let reads = 0;
  const session = new ObjectPublicationFollowup(
    op,
    async (method, input) => {
      if (method === "objectTask.publicationFollowups") {
        reads++;
        return [receipt(sent)];
      }
      sent = input;
      return pending.promise;
    },
    async () => {},
    () => "command",
  );
  session.edit(draft);
  const create = session.create();
  session.cancel();
  pending.resolve(receipt(sent));
  await create;
  assert.equal(session.getSnapshot().receipts.length, 0);
  assert.deepEqual(session.getSnapshot().draft, draft);
  await session.refresh();
  assert.equal(reads, 1);
  assert.equal(session.getSnapshot().retry, false);
  assert.equal(session.getSnapshot().receipts.length, 1);
});

test("mismatched provenance cannot clear input or report success; invalid text remains editable", async () => {
  const session = new ObjectPublicationFollowup(
    op,
    async (_method, input) => ({
      ...receipt(input),
      source: { ...target, attemptId: "other" },
    }),
    async () => {
      assert.fail("must not refresh");
    },
  );
  session.edit(draft);
  await session.create();
  assert.match(session.getSnapshot().error, /来源不一致/);
  assert.deepEqual(session.getSnapshot().draft, draft);
  assert.equal(session.getSnapshot().receipts.length, 0);
  const invalid = new ObjectPublicationFollowup(
    op,
    async () => {
      assert.fail("invalid input must not write");
    },
    async () => {},
  );
  await invalid.create();
  assert.equal(invalid.getSnapshot().retry, false);
  invalid.edit(draft);
  assert.deepEqual(invalid.getSnapshot().draft, draft);
});

test("fresh session restores draft and write-ahead request; committed retry is recovered read-only", async () => {
  const storage = draftStorage();
  let sent: unknown;
  const first = new ObjectPublicationFollowup(
    op,
    async (_method, input) => {
      sent = input;
      const raw = [...storage.data.values()][0];
      assert.ok(raw);
      const saved = JSON.parse(raw);
      assert.deepEqual(
        saved.request,
        input,
        "request must be durable before API mutation",
      );
      throw Error("response lost");
    },
    async () => {},
    () => "durable-request",
    storage,
  );
  first.edit(draft);
  const editable = new ObjectPublicationFollowup(
    op,
    async () => [],
    async () => {},
    undefined,
    storage,
  );
  assert.deepEqual(editable.getSnapshot().draft, draft);
  assert.equal(editable.getSnapshot().retry, false);
  await first.create();
  first.cancel();
  let changes = 0;
  const restored = new ObjectPublicationFollowup(
    op,
    async (method) => {
      assert.equal(method, "objectTask.publicationFollowups");
      return [receipt(sent)];
    },
    async () => {
      changes++;
    },
    () => {
      throw Error("must not allocate another request");
    },
    storage,
  );
  assert.equal(restored.getSnapshot().retry, true);
  assert.deepEqual(restored.getSnapshot().draft, draft);
  await restored.refresh();
  assert.equal(changes, 1);
  assert.equal(restored.getSnapshot().retry, false);
  const clean = new ObjectPublicationFollowup(
    op,
    async () => [],
    async () => {},
    undefined,
    storage,
  );
  assert.equal(clean.getSnapshot().draft.feedback, "");
  assert.equal(clean.getSnapshot().retry, false);
});

test("storage failure blocks mutation; restart retries the original request and isolates publications", async () => {
  const storage = draftStorage();
  let blocked = true;
  const writes: unknown[] = [];
  const api = async (_method: string, input: unknown) => {
    writes.push(input);
    return receipt(input);
  };
  const first = new ObjectPublicationFollowup(
    op,
    api,
    async () => {},
    () => "original",
    {
      getItem: storage.getItem,
      setItem: (key, value) => {
        if (blocked) throw Error("quota exceeded");
        storage.setItem(key, value);
      },
    },
  );
  first.edit(draft);
  await first.create();
  assert.equal(writes.length, 0);
  assert.match(first.getSnapshot().storageError, /无法保存/);
  blocked = false;
  // Persist the pending request, then lose the API response before a fresh session.
  const pending = deferred<unknown>();
  const original = new ObjectPublicationFollowup(
    op,
    async (_method, input) => {
      writes.push(input);
      return pending.promise;
    },
    async () => {},
    () => "persisted",
    storage,
  );
  original.edit(draft);
  const create = original.create();
  original.cancel();
  const other = new ObjectPublicationFollowup(
    { ...op, versionId: "v2" },
    api,
    async () => {},
    undefined,
    storage,
  );
  assert.equal(other.getSnapshot().draft.feedback, "");
  const restored = new ObjectPublicationFollowup(
    op,
    api,
    async () => {},
    () => "must-not-use",
    storage,
  );
  await restored.create();
  assert.deepEqual(writes[0], writes[1]);
  pending.resolve(receipt(writes[0]));
  await create;
});

test("corrupt or foreign local requests remain untouched and prevent new mutations", async () => {
  const storage = draftStorage();
  const seed = new ObjectPublicationFollowup(
    op,
    async () => [],
    async () => {},
    undefined,
    storage,
  );
  seed.edit(draft);
  const key = [...storage.data.keys()][0];
  assert.ok(key);
  const good = storage.data.get(key)!;
  for (const raw of [
    "broken JSON",
    JSON.stringify({ ...JSON.parse(good), projectId: "foreign" }),
    JSON.stringify({
      ...JSON.parse(good),
      draft: { ...draft, previewFrame: undefined },
      request: {
        projectId: "p",
        publicationRequestId: "pub",
        versionId: "v1",
        requestId: "uncertain",
        ...draft,
      },
    }),
  ]) {
    storage.data.set(key, raw);
    const session = new ObjectPublicationFollowup(
      op,
      async () => {
        assert.fail("must not mutate");
      },
      async () => {},
      undefined,
      storage,
    );
    session.edit(draft);
    await session.create();
    assert.equal(session.getSnapshot().recoveryBlocked, true);
    assert.equal(storage.data.get(key), raw);
    storage.data.set(key, good);
    session.restore();
    assert.equal(session.getSnapshot().recoveryBlocked, false);
    assert.deepEqual(session.getSnapshot().draft, draft);
  }
});
