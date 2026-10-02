import assert from "node:assert/strict";
import test from "node:test";
import {
  confirmedFinalRelocation,
  finalRelocationDraftSchema,
  matchesFinalRelocationReceipt,
  updateFinalRelocation,
  type FinalRelocationDraft,
} from "../src/ui/object-tasks/final-relocation-draft";
import { parsePublishedFeedbackFrame } from "../src/ui/object-tasks/PublishedFeedbackFrame";
import { ObjectPublicationApprovalDraft } from "../src/ui/object-tasks/object-publication-approval-draft";
import {
  finalRelocationFixture,
  publicationStorage,
} from "./fixtures/final-relocation";

test("final confirmation binds exact source and exact current output, not historical checkpoint mapping", () => {
  const f = finalRelocationFixture();
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
    [f.source, "id", "other"],
    [f.source, "runId", "other"],
    [f.source, "projectId", "other"],
    [f.source.source, "runId", "other"],
    [f.source.source.target, "projectId", "other"],
    [f.source.source.target, "runId", "other"],
    [f.source.source.target, "attemptId", "other"],
    [f.source.source.target, "checkpoint", "input"],
    [f.source.frame, "frozen", false],
    [f.source.selection!, "sequence", 99],
    [f.source.selection!, "sha256", "e".repeat(64)],
    [f.target, "id", "other"],
    [f.target, "runId", "other"],
    [f.target, "projectId", "other"],
    [f.target.source, "runId", "other"],
    [f.target.source, "sourceDigest", "e".repeat(64)],
    [f.target.source.target, "projectId", "other"],
    [f.target.source.target, "runId", "other"],
    [f.target.source.target, "attemptId", "old-attempt"],
    [f.target.source.target, "checkpoint", "input"],
    [f.target.source.target, "path", "other.tscn"],
    [f.target.source.target, "sha256", "e".repeat(64)],
    [f.target.frame, "frozen", false],
    [f.target.selection!, "sequence", 99],
    [f.target.selection!, "sha256", "e".repeat(64)],
  ] as [object, string, unknown][]) {
    const old = Reflect.get(object, field);
    Reflect.set(object, field, value);
    assert.equal(confirm(), undefined, field);
    Reflect.set(object, field, old);
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
  assert.equal(
    confirmedFinalRelocation(
      f.preview,
      f.feedback,
      f.confirmation,
      f.source,
      undefined,
    ),
    undefined,
  );
  assert.equal(
    confirmedFinalRelocation(
      f.preview,
      f.feedback,
      f.confirmation,
      f.source,
      f.source,
    ),
    undefined,
  );
});

test("approval compares canonical receipts while immediately revoking changed or unchecked drafts", () => {
  const f = finalRelocationFixture();
  const draft = finalRelocationDraftSchema.parse(f.confirmation);
  const receipt = confirmedFinalRelocation(
    f.preview,
    f.feedback,
    draft,
    f.source,
    f.target,
  );
  assert.ok(receipt);
  assert.notEqual(JSON.stringify(draft), JSON.stringify(receipt));
  assert.equal(matchesFinalRelocationReceipt(draft, receipt), true);
  assert.equal(matchesFinalRelocationReceipt(undefined, receipt), false);
  assert.equal(matchesFinalRelocationReceipt(draft, undefined), false);
  for (const changed of [
    { ...draft, confirmed: false },
    { ...draft, sourceDigest: "e".repeat(64) },
    { ...draft, targetFrame: undefined },
    { ...draft, targetFrame: { ...receipt.targetFrame, runId: "other" } },
    { ...draft, targetFrame: { ...receipt.targetFrame, frameId: "other" } },
    { ...draft, regions: [] },
    {
      ...draft,
      regions: [
        { status: "matched" as const, sourceRegion: 0, targetRegion: 1 },
        draft.regions[1]!,
      ],
    },
    {
      ...draft,
      regions: [
        draft.regions[0]!,
        {
          status: "absent" as const,
          sourceRegion: 1,
          note: "Different reason",
        },
      ],
    },
  ])
    assert.equal(matchesFinalRelocationReceipt(changed, receipt), false);
});

test("every source region requires ordered correspondence or a nonempty UTF-8 bounded reason", () => {
  const f = finalRelocationFixture();
  const invalid: FinalRelocationDraft[] = [
    { ...f.confirmation, confirmed: false },
    { ...f.confirmation, sourceDigest: "e".repeat(64) },
    { ...f.confirmation, targetFrame: undefined },
    { ...f.confirmation, regions: [] },
    { ...f.confirmation, regions: f.confirmation.regions.slice(0, 1) },
    { ...f.confirmation, regions: [...f.confirmation.regions].reverse() },
    {
      ...f.confirmation,
      regions: [f.confirmation.regions[0]!, f.confirmation.regions[0]!],
    },
    {
      ...f.confirmation,
      regions: [
        { status: "matched", sourceRegion: 0, targetRegion: 1 },
        f.confirmation.regions[1]!,
      ],
    },
    ...[" ", "字".repeat(334)].map((note) => ({
      ...f.confirmation,
      regions: [
        f.confirmation.regions[0]!,
        { status: "absent" as const, sourceRegion: 1, note },
      ],
    })),
  ];
  for (const draft of invalid)
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
  const valid = {
    ...f.confirmation,
    regions: [
      f.confirmation.regions[0]!,
      { status: "absent" as const, sourceRegion: 1, note: "字".repeat(333) },
    ],
  };
  assert.ok(
    confirmedFinalRelocation(f.preview, f.feedback, valid, f.source, f.target),
  );
});

test("published source is resolved by exact original version and reference", () => {
  const f = finalRelocationFixture(true);
  const parse = (raw: unknown) =>
    parsePublishedFeedbackFrame(raw, "p", "hero", f.feedback);
  assert.deepEqual(parse([f.source]), f.source);
  assert.ok(
    confirmedFinalRelocation(
      f.preview,
      f.feedback,
      f.confirmation,
      f.source,
      f.target,
    ),
  );
  for (const [object, field, value] of [
    [f.source.source.target, "versionId", "new-version"],
    [f.source.source.target, "objectId", "other"],
    [f.source.source.target, "projectId", "other"],
    [f.source, "id", "other"],
    [f.source, "runId", "other"],
    [f.source, "projectId", "other"],
    [f.source.source, "runId", "other"],
    [f.source.frame, "frozen", false],
    [f.source.selection!, "sequence", 99],
  ] as [object, string, unknown][]) {
    const old = Reflect.get(object, field);
    Reflect.set(object, field, value);
    assert.throws(() => parse([f.source]), /MISMATCH/);
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
    Reflect.set(object, field, old);
  }
  assert.throws(() => parse([]));
  assert.throws(() => parse([f.source, f.source]));
  assert.throws(() => parse([f.target]), /MISMATCH/);
});

test("inherited attempt source uses its original run, never the followup run", () => {
  const f = finalRelocationFixture();
  f.feedback.origin = {
    publicationRequestId: "original-publication",
    versionId: "v1",
    runId: "original-run",
    publishedFrame: false,
  };
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
  if (!("attemptId" in f.source.source.target))
    throw new Error("attempt fixture required");
  f.source.source.target.runId = "original-run";
  assert.ok(
    confirmedFinalRelocation(
      f.preview,
      f.feedback,
      f.confirmation,
      f.source,
      f.target,
    ),
  );
});

test("draft reopen preserves mapping but clears final authorization; replacing either identity clears both", () => {
  const f = finalRelocationFixture();
  const port = publicationStorage();
  const draft = new ObjectPublicationApprovalDraft(f.preview, port);
  draft.edit({
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
    port,
  ).getSnapshot();
  assert.equal(restored.restored, true);
  assert.deepEqual(restored.draft.decisions.feedback!.finalRelocation, {
    ...f.confirmation,
    confirmed: false,
  });
  assert.equal(
    draft.getSnapshot().draft.decisions.feedback!.finalRelocation!.confirmed,
    true,
  );
  for (const next of [
    { ...f.confirmation, sourceDigest: "e".repeat(64) },
    {
      ...f.confirmation,
      targetFrame: { ...f.confirmation.targetFrame, frameId: "different" },
    },
  ]) {
    assert.deepEqual(updateFinalRelocation(f.confirmation, next), {
      ...next,
      regions: [],
      confirmed: false,
    });
    draft.edit({
      decisions: {
        feedback: {
          resolution: "resolved",
          note: "Fixed",
          finalRelocation: next,
        },
      },
    });
    assert.equal(
      draft.getSnapshot().draft.decisions.feedback!.finalRelocation!.confirmed,
      false,
    );
    assert.deepEqual(
      draft.getSnapshot().draft.decisions.feedback!.finalRelocation!.regions,
      [],
    );
  }
});
