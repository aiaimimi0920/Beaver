import assert from "node:assert/strict";
import test from "node:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { ObjectPublication } from "../src/ui/object-tasks/object-publication";
import { PublicationApproval } from "../src/ui/object-tasks/PublicationApproval";
import {
  publicationRequestSchema,
  type PublicationDecision,
  type PublicationOperation,
} from "../src/shared/object-publication";
import {
  finalRelocationFixture,
  publicationStorage,
} from "./fixtures/final-relocation";

test("missing, unconfirmed or stale final correspondence never writes intent or dispatches publication", async () => {
  const f = finalRelocationFixture();
  const sent: string[] = [];
  const port = publicationStorage();
  const session = new ObjectPublication(
    f.preview.review,
    async (method) => {
      sent.push(method);
      if (method === "objectTask.publications") return [];
      assert.equal(method, "objectTask.publicationPreview");
      return f.preview;
    },
    async () => {},
    () => "publish",
    port,
  );
  await session.refresh();
  const originalReads = [...sent];
  for (const finalRelocation of [
    undefined,
    { ...f.confirmation, confirmed: false },
    { ...f.confirmation, sourceDigest: "e".repeat(64) },
    { ...f.confirmation, regions: f.confirmation.regions.slice(0, 1) },
  ]) {
    await session.publish({
      acceptanceNote: "Reviewed",
      confirmFiles: true,
      confirmReplacement: false,
      feedback: [
        {
          requestId: "feedback",
          resolution: "resolved",
          note: "Fixed",
          finalRelocation,
        } as PublicationDecision,
      ],
    });
    assert.match(session.getSnapshot().error, /逐区核对/);
    assert.equal(session.getSnapshot().retry, false);
    assert.equal(port.values.size, 0);
    assert.deepEqual(sent, originalReads);
  }
});

test("final correspondence survives response loss and reopen only retries the immutable original authorization", async () => {
  const f = finalRelocationFixture();
  const port = publicationStorage();
  const sent: unknown[] = [];
  const calls: string[] = [];
  let changed = 0;
  const api = async (method: string, input: unknown) => {
    calls.push(method);
    if (method === "objectTask.publications") return [];
    if (method === "objectTask.publicationPreview") return f.preview;
    assert.equal(method, "objectTask.publishCandidate");
    sent.push(structuredClone(input));
    if (sent.length === 1) throw new Error("lost response");
    const op: PublicationOperation = {
      schemaVersion: 1,
      request: publicationRequestSchema.parse(input),
      preview: f.preview,
      versionId: "v1",
      state: "published",
      error: null,
      result: {
        versionId: "v1",
        objectRevision: 2,
        taskRevision: 2,
        runRevision: 2,
        planRevision: 2,
      },
    };
    return op;
  };
  const session = new ObjectPublication(
    f.preview.review,
    api,
    async () => {
      changed++;
    },
    () => "original",
    port,
  );
  await session.refresh();
  await session.publish({
    acceptanceNote: "Reviewed",
    confirmFiles: true,
    confirmReplacement: false,
    feedback: [
      {
        requestId: "feedback",
        resolution: "resolved",
        note: "Fixed",
        finalRelocation: f.confirmation,
      },
    ],
  });
  assert.match(session.getSnapshot().error, /lost response/);
  const original = publicationRequestSchema.parse(sent[0]);
  assert.deepEqual(original.feedback[0]!.finalRelocation, f.confirmation);
  const reopened = new ObjectPublication(
    f.preview.review,
    api,
    async () => {
      changed++;
    },
    () => "never-used",
    port,
  );
  assert.equal(reopened.getSnapshot().restored, true);
  await reopened.retry();
  assert.equal(sent.length, 1, "retry before reconciliation must not dispatch");
  const beforeReads = calls.length;
  await reopened.refresh();
  assert.deepEqual(calls.slice(beforeReads), ["objectTask.publications"]);
  assert.equal(reopened.getSnapshot().preview, null);
  reopened
    .approvalFor(f.preview)
    .edit({ note: "New unrelated approval", decisions: {} });
  await reopened.publish({
    acceptanceNote: "Must not replace original",
    confirmFiles: false,
    confirmReplacement: false,
    feedback: [],
  });
  assert.deepEqual(sent[1], original);
  assert.equal(changed, 1);
  assert.equal(reopened.getSnapshot().retry, false);
  assert.equal(reopened.getSnapshot().operations[0]?.state, "published");
});

test("exact published frame reads and inherited feedback files keep original source identity", async () => {
  const f = finalRelocationFixture(true);
  const calls: [string, unknown][] = [];
  const session = new ObjectPublication(
    f.preview.review,
    async (method, input) => {
      calls.push([method, input]);
      if (method === "objectTask.publicationFrames") return [f.source];
      assert.equal(method, "objectTask.attemptFile");
      return { request: input, byteCount: 1, content: { kind: "binary" } };
    },
    async () => {},
    undefined,
    publicationStorage(),
  );
  assert.deepEqual(
    await session.publicationFrames(
      "original-publication",
      f.feedback.previewFrame!,
    ),
    [f.source],
  );
  await session
    .createFeedbackFile(f.feedback)
    .open("output", "preview.png", "a".repeat(64));
  assert.deepEqual(calls, [
    [
      "objectTask.publicationFrames",
      {
        projectId: "p",
        publicationRequestId: "original-publication",
        previewFrame: f.feedback.previewFrame,
      },
    ],
    [
      "objectTask.attemptFile",
      {
        projectId: "p",
        runId: "original-run",
        attemptId: "old-attempt",
        checkpoint: "output",
        path: "preview.png",
        sha256: "a".repeat(64),
      },
    ],
  ]);
});

test("production approval cannot authorize unread PNGs even with a complete saved mapping", () => {
  const f = finalRelocationFixture(true);
  const port = publicationStorage();
  let calls = 0;
  const make = () =>
    new ObjectPublication(
      f.preview.review,
      async () => {
        calls++;
        return [];
      },
      async () => {},
      undefined,
      port,
    );
  const session = make();
  session.approvalFor(f.preview).edit({
    note: "Reviewed",
    decisions: {
      feedback: {
        resolution: "resolved",
        note: "Fixed",
        finalRelocation: f.confirmation,
      },
    },
  });
  for (const view of [session, make()]) {
    const html = renderToStaticMarkup(
      createElement(PublicationApproval, {
        session: view,
        preview: f.preview,
        busy: false,
      }),
    );
    assert.match(html, /最终候选逐区重新确认/);
    assert.match(html, /两张原 PNG 必须读取成功/);
    assert.match(
      html,
      /<input type="checkbox" disabled=""\/>已逐区核对原反馈与最终候选输出/,
    );
    assert.match(html, /<button disabled="">接受并发布对象<\/button>/);
    assert.doesNotMatch(html, /<input[^>]*checked=""/);
  }
  assert.equal(
    calls,
    0,
    "server rendering is read-only and cannot decode frames",
  );
});
