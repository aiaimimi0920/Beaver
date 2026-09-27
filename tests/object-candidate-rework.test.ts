import assert from "node:assert/strict";
import test from "node:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { ObjectTaskExecution } from "../src/ui/object-tasks/object-task-execution";
import { ObjectTaskResume } from "../src/ui/object-tasks/object-task-resume";
import {
  prepareCandidateRework,
  ObjectCandidateReworkPanel,
} from "../src/ui/object-tasks/ObjectCandidateReworkPanel";
import { ObjectTaskResumePanel } from "../src/ui/object-tasks/ObjectTaskResumePanel";
import { objectCandidateReportSchema } from "../src/shared/object-candidate-review";
import {
  objectRecoveryResumeOperationSchema,
  objectRecoveryResumeRequestSchema,
} from "../src/shared/object-recovery";
import { execution } from "./fixtures/object-attempts";
import { objectTaskSnapshot } from "./fixtures/object-tasks";
import { verifiedView } from "./fixtures/object-recovery";
import { relocation } from "./fixtures/feedback-relocation";

const attempt = { ...execution("awaitingGate").attempt, taskRevision: 3 };
const image = {
  path: "preview.png",
  sha256: "c".repeat(64),
  width: 20,
  height: 10,
  regions: [
    { x: 0.1, y: 0.2, width: 0.5, height: 0.5, prompt: "Brighten this area" },
  ],
};
const report = objectCandidateReportSchema.parse({
  schemaVersion: 1,
  request: {
    projectId: "p",
    requestId: "review",
    target: attempt.target,
    checkRequestId: "check",
  },
  time: "now",
  sourceDigest: "a".repeat(64),
  outputDigest: "b".repeat(64),
  baselineVersionId: null,
  acceptedVersionIdAtClaim: null,
  acceptedVersionIdAtReview: null,
  objectRevisionAtClaim: 0,
  objectRevisionAtReview: 0,
  stages: [
    {
      taskId: "fine",
      title: "Final",
      revision: 3,
      status: "awaitingAcceptance",
      acceptance: "Works",
    },
  ],
  references: [],
  files: [
    {
      path: image.path,
      before: null,
      after: image.sha256,
      reference: false,
      owners: [],
    },
  ],
  rules: ["checkpoint-integrity", "code-structure"].map((id) => ({
    id,
    version: 1,
    passed: true,
    issues: [],
    filesChecked: 0,
  })),
  blockers: [
    "PUBLICATION_NOT_IMPLEMENTED",
    "FINAL_FINE_OWNER_ACCEPTANCE_REQUIRED",
    "FEEDBACK_REVIEW_UNAVAILABLE",
  ],
});
function receipt(input: unknown) {
  return objectRecoveryResumeOperationSchema.parse({
    schemaVersion: 1,
    request: input,
    result: {
      status: "started",
      report: verifiedView().operation!.result,
      attemptId: "attempt-2",
      fineTaskId: "fine",
    },
  });
}

for (const archived of [false, true])
  test(
    "production rework confirms and retries exact authorization archived=" +
      archived,
    async () => {
      const previewFrame = { runId: "preview-run", frameId: "saved-frame" };
      const sent: unknown[] = [];
      const session = new ObjectTaskExecution(
        "p",
        objectTaskSnapshot(),
        "medium",
        async (method, input) => {
          if (method === "objectTask.recovery") return verifiedView();
          if (method === "objectTask.attempts") return [];
          assert.equal(method, "objectTask.reworkCandidate");
          sent.push(structuredClone(input));
          if (sent.length === 1) throw Error("response lost");
          return receipt(input);
        },
        async () => true,
        () => "rework",
      );
      await session.recovery.refresh();
      assert.equal(
        prepareCandidateRework(
          session,
          { ...attempt, taskRevision: 2 },
          report,
          "Feedback",
        ),
        false,
      );
      assert.equal(
        prepareCandidateRework(
          session,
          attempt,
          {
            ...report,
            request: {
              ...report.request,
              target: { ...attempt.target, attemptId: "older" },
            },
          },
          "Feedback",
        ),
        false,
      );
      assert.equal(
        prepareCandidateRework(session, attempt, report, " "),
        false,
      );
      assert.equal(
        prepareCandidateRework(session, attempt, report, "Feedback", {
          ...image,
          sha256: "d".repeat(64),
        }),
        false,
      );
      assert.match(
        renderToStaticMarkup(
          createElement(ObjectCandidateReworkPanel, {
            session,
            attempt,
            report,
          }),
        ),
        /修改意见/,
      );
      assert.equal(
        prepareCandidateRework(
          session,
          attempt,
          report,
          "Reduce movement speed",
          archived ? undefined : image,
          archived ? previewFrame : undefined,
          archived ? relocation : undefined,
        ),
        true,
      );
      assert.equal(sent.length, 0);
      const html = renderToStaticMarkup(
        createElement(ObjectTaskResumePanel, { session: session.recovery }),
      );
      assert.match(html, /Reduce movement speed/);
      assert.match(html, archived ? /saved-frame/ : /preview.png/);
      assert.match(html, archived ? /相机来源/ : /Brighten this area/);
      assert.match(html, /费用/);
      if (archived) assert.match(html, /Badge removed/);
      await session.recovery.resume.submit();
      assert.equal(session.recovery.resume.getSnapshot().retryAvailable, true);
      await session.recovery.resume.submit();
      assert.deepEqual(sent[0], sent[1]);
      assert.deepEqual(
        objectRecoveryResumeRequestSchema.parse(sent[0]).rework?.relocation,
        archived ? relocation : undefined,
      );
      assert.deepEqual(
        archived
          ? objectRecoveryResumeRequestSchema.parse(sent[0]).rework
              ?.previewFrame
          : objectRecoveryResumeRequestSchema.parse(sent[0]).rework?.image,
        archived ? previewFrame : image,
      );
      assert.equal(
        objectRecoveryResumeRequestSchema.parse(sent[0]).rework
          ?.reviewRequestId,
        "review",
      );
    },
  );

test("pending rework restores without launching and uses its saved purpose and feedback", async () => {
  const view = verifiedView();
  const request = objectRecoveryResumeRequestSchema.parse({
    projectId: "p",
    requestId: "saved",
    target: view.target,
    verificationRequestId: "verify-1",
    rework: {
      reviewRequestId: "review",
      attemptId: attempt.target.attemptId,
      fineTaskId: "fine",
      feedback: "  Keep whitespace  ",
      previewFrame: { runId: "preview-run", frameId: "saved-frame" },
    },
  });
  let calls = 0;
  assert.equal(
    objectRecoveryResumeRequestSchema.safeParse({
      ...request,
      rework: { ...request.rework, image },
    }).success,
    false,
  );
  const session = new ObjectTaskResume(
    {
      projectId: "p",
      view: () => view,
      available: () => true,
      changed() {},
      refresh: async () => true,
      started() {},
    },
    async (method, input) => {
      calls++;
      assert.equal(method, "objectTask.reworkCandidate");
      assert.deepEqual(input, request);
      return receipt(input);
    },
    () => "unused",
  );
  session.observe({
    ...view,
    canDispose: false,
    resume: { schemaVersion: 1, request, result: null },
  });
  assert.equal(calls, 0);
  await session.submit();
  assert.equal(calls, 1);
  const wrong = receipt(request);
  if (wrong.result?.status === "started") wrong.result.fineTaskId = "other";
  assert.equal(
    objectRecoveryResumeOperationSchema.safeParse(wrong).success,
    false,
  );
  assert.equal(
    objectRecoveryResumeRequestSchema.safeParse({
      ...request,
      rework: { ...request.rework, feedback: "验".repeat(1334) },
    }).success,
    false,
  );
  assert.equal(
    objectRecoveryResumeRequestSchema.safeParse({
      ...request,
      advance: {
        attemptId: "a",
        checkRequestId: "c",
        nextFineTaskId: "next",
        nextFineRevision: 0,
        acceptanceNote: "Accept",
      },
    }).success,
    false,
  );
});
