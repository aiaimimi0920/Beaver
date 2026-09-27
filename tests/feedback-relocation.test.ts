import assert from "node:assert/strict";
import test from "node:test";
import {
  confirmedRelocation,
  type RelocationDraft,
} from "../src/ui/object-tasks/feedback-relocation-draft";
import { ObjectCandidateFeedbackDraft } from "../src/ui/object-tasks/object-candidate-feedback-draft";
import { feedbackFrame, relocation } from "./fixtures/feedback-relocation";
import { execution } from "./fixtures/object-attempts";

test("partial correspondence survives reopening, but changing either frame clears prior decisions", () => {
  const values = new Map<string, string>();
  const disk = {
    getItem: (key: string) => values.get(key) ?? null,
    setItem: (key: string, value: string) => {
      values.set(key, value);
    },
  };
  const review = {
    projectId: "p",
    target: execution("awaitingGate").attempt.target,
    reviewRequestId: "review",
  };
  const draft = new ObjectCandidateFeedbackDraft(review, disk);
  const previewFrame = { runId: "preview-run", frameId: "saved-frame" };
  draft.edit({ previewFrame, relocation });
  const partial: RelocationDraft = {
    ...relocation,
    confirmed: false,
    regions: [
      relocation.regions[0]!,
      { status: "absent", sourceRegion: 1, note: "" },
    ],
  };
  draft.edit({ relocation: partial });
  const reopened = new ObjectCandidateFeedbackDraft(review, disk);
  assert.deepEqual(reopened.getSnapshot().draft.relocation, partial);
  for (const patch of [
    { previewFrame: { ...previewFrame, frameId: "other-frame" } },
    { relocation: { ...relocation, sourceAttemptId: "another-attempt" } },
    {
      relocation: {
        ...relocation,
        sourceFrame: { ...relocation.sourceFrame, runId: "another-preview" },
      },
    },
  ]) {
    reopened.edit({ previewFrame, relocation });
    reopened.edit({ relocation });
    assert.deepEqual(reopened.getSnapshot().draft.relocation, relocation);
    reopened.edit(patch);
    assert.deepEqual(reopened.getSnapshot().draft.relocation?.regions, []);
    assert.equal(reopened.getSnapshot().draft.relocation?.confirmed, false);
  }
});

test("confirmation requires matching frame authority, complete decisions and real target region bounds", () => {
  const source = feedbackFrame(true),
    target = feedbackFrame(false);
  assert.deepEqual(confirmedRelocation(relocation, source, target), relocation);
  assert.equal(confirmedRelocation(relocation, undefined, target), undefined);
  assert.equal(confirmedRelocation(relocation, source, undefined), undefined);
  for (const invalid of [
    { ...relocation, confirmed: false },
    { ...relocation, regions: relocation.regions.slice(0, 1) },
    { ...relocation, sourceAttemptId: "foreign" },
    {
      ...relocation,
      sourceFrame: { ...relocation.sourceFrame, frameId: "foreign" },
    },
    {
      ...relocation,
      regions: [
        { status: "matched", sourceRegion: 0, targetRegion: 1 },
        relocation.regions[1]!,
      ],
    },
    ...["", " ", "字".repeat(334)].map((note) => ({
      ...relocation,
      regions: [
        relocation.regions[0]!,
        { status: "absent", sourceRegion: 1, note },
      ],
    })),
  ] as RelocationDraft[]) {
    assert.equal(confirmedRelocation(invalid, source, target), undefined);
  }
  for (const patch of [
    { projectId: "other" },
    { runId: "other" },
    { attemptId: "old-attempt" },
    { checkpoint: "input" },
  ]) {
    const foreign = structuredClone(target);
    Object.assign(foreign.source.target, patch);
    assert.equal(confirmedRelocation(relocation, source, foreign), undefined);
  }
});
