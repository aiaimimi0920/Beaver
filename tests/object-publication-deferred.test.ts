import assert from "node:assert/strict";
import test from "node:test";
import { ObjectPublicationDeferred } from "../src/ui/object-tasks/object-publication-deferred";
import { deferredFeedbackSchema } from "../src/shared/object-publication-deferred";
import { execution } from "./fixtures/object-attempts";
import { relocation } from "./fixtures/feedback-relocation";

const review = {
  projectId: "p",
  target: execution("awaitingGate").attempt.target,
  reviewRequestId: "review",
};
const input = {
  relocation,
  previewFrame: { runId: "preview-run", frameId: "saved-frame" },
  feedback: "Improve contrast",
  later: { title: "Improve labels", acceptance: "Labels are readable" },
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

test("deferred feedback refuses two image authorities", () => {
  assert.equal(
    deferredFeedbackSchema.safeParse({
      ...input,
      projectId: "p",
      requestId: "id",
      review,
      image: {
        path: "preview.png",
        sha256: "a".repeat(64),
        width: 20,
        height: 10,
        regions: [],
      },
    }).success,
    false,
  );
});

test("deferred feedback persists before send and replays original request after reopening", async () => {
  const disk = storage();
  const sent: unknown[] = [];
  let refreshes = 0;
  const api = async (method: string, raw: unknown) => {
    assert.equal(method, "objectTask.deferCandidateFeedback");
    const request = deferredFeedbackSchema.parse(raw);
    assert.deepEqual(JSON.parse([...disk.values.values()][0]!), request);
    sent.push(request);
    if (sent.length === 1) throw new Error("response lost");
    return request;
  };
  const session = new ObjectPublicationDeferred(
    review,
    api,
    async () => {
      refreshes++;
    },
    () => "original",
    disk,
  );
  await session.save(input);
  assert.ok(session.getSnapshot().pending);
  const reopened = new ObjectPublicationDeferred(
    review,
    api,
    async () => {
      refreshes++;
    },
    () => "must-not-be-used",
    disk,
  );
  await reopened.save({
    ...input,
    relocation: {
      ...relocation,
      regions: [{ status: "absent", sourceRegion: 0, note: "Changed" }],
    },
    feedback: "must not replace uncertain request",
  });
  assert.deepEqual(sent[0], sent[1]);
  assert.equal(reopened.getSnapshot().pending, null);
  assert.equal(reopened.getSnapshot().saved?.requestId, "original");
  assert.deepEqual(
    reopened.getSnapshot().saved?.previewFrame,
    input.previewFrame,
  );
  assert.equal(disk.values.size, 0);
  assert.deepEqual(reopened.getSnapshot().saved?.relocation, relocation);
  assert.equal(refreshes, 1);
});

test("storage failures and corrupt recovery never issue new deferred requests", async () => {
  const disk = storage();
  let calls = 0;
  const api = async () => {
    calls++;
    return {};
  };
  const session = new ObjectPublicationDeferred(
    review,
    api,
    async () => {},
    () => "id",
    {
      ...disk,
      setItem() {
        throw new Error("quota");
      },
    },
  );
  await session.save(input);
  assert.match(session.getSnapshot().error, /quota/);
  assert.equal(calls, 0);
  const corrupt = new ObjectPublicationDeferred(
    review,
    api,
    async () => {},
    () => "new",
    {
      ...disk,
      getItem: () => "{broken",
    },
  );
  await corrupt.save(input);
  assert.equal(corrupt.getSnapshot().blocked, true);
  assert.equal(calls, 0);
});

test("mismatched receipt retains the immutable request and does not refresh publication", async () => {
  const session = new ObjectPublicationDeferred(
    review,
    async (_, request) => ({
      ...deferredFeedbackSchema.parse(request),
      feedback: "wrong",
    }),
    async () => {
      assert.fail("must not refresh");
    },
    () => "id",
    storage(),
  );
  await session.save(input);
  assert.equal(session.getSnapshot().pending?.feedback, input.feedback);
  assert.match(session.getSnapshot().error, /回执/);
});
