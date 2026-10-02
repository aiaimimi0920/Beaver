import assert from "node:assert/strict";
import test from "node:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { confirmedFinalRelocation } from "../src/ui/object-tasks/final-relocation-draft";
import { ObjectPublicationApprovalDraft } from "../src/ui/object-tasks/object-publication-approval-draft";
import { ObjectPublication } from "../src/ui/object-tasks/object-publication";
import { PublicationApproval } from "../src/ui/object-tasks/PublicationApproval";
import {
  publicationRequestSchema,
  type PublicationOperation,
} from "../src/shared/object-publication";
import {
  finalRelocationPngFixture,
  publicationStorage,
} from "./fixtures/final-relocation";

test("standalone PNG confirmation binds exact loaded output identity, image metadata and regions", () => {
  for (const inherited of [false, true]) {
    const f = finalRelocationPngFixture(inherited);
    const confirm = () =>
      confirmedFinalRelocation(
        f.preview,
        f.feedback,
        f.confirmation,
        f.source,
        f.target,
      );
    assert.deepEqual(confirm(), f.confirmation);
    for (const [object, field, value] of [
      [f.source.request, "projectId", "foreign"],
      [f.source.request, "runId", "foreign"],
      [f.source.request, "attemptId", "foreign"],
      [f.source.request, "checkpoint", "input"],
      [f.source.request, "path", "other.png"],
      [f.source.request, "sha256", "e".repeat(64)],
      [f.source, "width", 401],
      [f.source, "height", 201],
      [f.source.image, "path", "other.png"],
      [f.source.image, "sha256", "e".repeat(64)],
      [f.source.image, "width", 401],
      [f.source.image, "height", 201],
      [f.source.image, "regions", f.source.image.regions.slice(0, 1)],
      [f.source.image.regions[0]!, "x", 0.01],
      [f.source.image.regions[0]!, "prompt", "Different owner intent"],
      [f.feedback, "previewFrame", { runId: "frame", frameId: "frame" }],
      [f.feedback.relocationRequirement!, "regionCount", 1],
    ] as [object, string, unknown][]) {
      const previous = Reflect.get(object, field);
      Reflect.set(object, field, value);
      assert.equal(confirm(), undefined, field);
      Reflect.set(object, field, previous);
    }
    assert.equal(
      confirmedFinalRelocation(
        f.preview,
        f.feedback,
        f.confirmation,
        undefined,
        f.target,
      ),
      undefined,
    );
    if (f.feedback.origin) {
      f.feedback.origin.publishedFrame = true;
      assert.equal(confirm(), undefined);
      f.feedback.origin.publishedFrame = false;
      f.source.request.runId = f.preview.review.target.runId;
      assert.equal(
        confirm(),
        undefined,
        "inherited PNG must read the original run",
      );
    }
  }
});

test("PNG mapping requires every ordered source region and exact current candidate target", () => {
  const f = finalRelocationPngFixture();
  for (const draft of [
    { ...f.confirmation, confirmed: false },
    { ...f.confirmation, sourceDigest: "e".repeat(64) },
    { ...f.confirmation, targetFrame: undefined },
    { ...f.confirmation, regions: f.confirmation.regions.slice(0, 1) },
    { ...f.confirmation, regions: [...f.confirmation.regions].reverse() },
    {
      ...f.confirmation,
      regions: [
        f.confirmation.regions[0]!,
        { status: "absent" as const, sourceRegion: 1, note: " " },
      ],
    },
  ])
    assert.equal(
      confirmedFinalRelocation(
        f.preview,
        f.feedback,
        draft,
        f.source,
        f.target,
      ),
      undefined,
    );
  f.target.source.sourceDigest = "e".repeat(64);
  assert.equal(
    confirmedFinalRelocation(
      f.preview,
      f.feedback,
      f.confirmation,
      f.source,
      f.target,
    ),
    undefined,
  );
});

test("PNG reopen preserves correspondence and reasons but never restores authorization or unread readiness", () => {
  const f = finalRelocationPngFixture();
  const storage = publicationStorage();
  const session = new ObjectPublication(
    f.preview.review,
    async () => [],
    async () => {},
    undefined,
    storage,
  );
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
  const restored = new ObjectPublicationApprovalDraft(
    f.preview,
    storage,
  ).getSnapshot();
  assert.equal(restored.restored, true);
  assert.deepEqual(restored.draft.decisions.feedback!.finalRelocation, {
    ...f.confirmation,
    confirmed: false,
  });
  for (const view of [
    session,
    new ObjectPublication(
      f.preview.review,
      async () => [],
      async () => {},
      undefined,
      storage,
    ),
  ]) {
    const html = renderToStaticMarkup(
      createElement(PublicationApproval, {
        session: view,
        preview: f.preview,
        busy: false,
      }),
    );
    assert.match(html, /重新读取原始冻结图片/);
    assert.match(html, /原反馈区域 1 对应/);
    assert.match(html, /原反馈区域 2 无对应原因/);
    assert.match(
      html,
      /<input type="checkbox" disabled=""\/>已逐区核对原反馈与最终候选输出/,
    );
    assert.match(html, /<button disabled="">接受并发布对象<\/button>/);
  }
});

test("inherited PNG response identity mismatch fails closed and the exact source can be read again", async () => {
  const f = finalRelocationPngFixture(true);
  const inputs: unknown[] = [];
  let mismatch = true;
  const session = new ObjectPublication(
    f.preview.review,
    async (method, input) => {
      assert.equal(method, "objectTask.attemptFile");
      inputs.push(structuredClone(input));
      return {
        request: mismatch
          ? { ...(input as object), runId: "followup-run" }
          : input,
        byteCount: 1,
        content: { kind: "binary" },
      };
    },
    async () => {},
    undefined,
    publicationStorage(),
  );
  const file = session.createFeedbackFile(f.feedback);
  await file.open("output", f.source.request.path, f.source.request.sha256);
  assert.equal(file.getSnapshot().response, null);
  assert.match(file.getSnapshot().error, /响应与所选文件不匹配/);
  mismatch = false;
  await file.open("output", f.source.request.path, f.source.request.sha256);
  assert.equal(file.getSnapshot().error, "");
  assert.deepEqual(inputs, [f.source.request, f.source.request]);
});

test("PNG final correspondence survives lost response and retry repeats the complete immutable request", async () => {
  const f = finalRelocationPngFixture(true);
  const storage = publicationStorage();
  const sent: unknown[] = [];
  const api = async (method: string, input: unknown) => {
    if (method === "objectTask.publications") return [];
    if (method === "objectTask.publicationPreview") return f.preview;
    assert.equal(method, "objectTask.publishCandidate");
    sent.push(structuredClone(input));
    if (sent.length === 1) throw new Error("lost PNG publication response");
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
    async () => {},
    () => "original-png",
    storage,
  );
  await session.refresh();
  await session.publish({
    acceptanceNote: "Reviewed PNG",
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
  assert.match(session.getSnapshot().error, /lost PNG publication response/);
  const original = publicationRequestSchema.parse(sent[0]);
  assert.deepEqual(original.feedback[0]!.finalRelocation, f.confirmation);
  const reopened = new ObjectPublication(
    f.preview.review,
    api,
    async () => {},
    () => "never-used",
    storage,
  );
  await reopened.retry();
  assert.equal(sent.length, 1, "reconciliation is required before retry");
  await reopened.refresh();
  reopened
    .approvalFor(f.preview)
    .edit({ note: "Unrelated new draft", decisions: {} });
  await reopened.retry();
  assert.deepEqual(sent[1], original);
  assert.equal(reopened.getSnapshot().retry, false);
  assert.equal(reopened.getSnapshot().operations[0]?.state, "published");
});
