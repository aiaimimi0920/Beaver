import assert from "node:assert/strict";
import test from "node:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { objectAttemptCheckReportSchema } from "../src/shared/object-attempt-checks";
import {
  objectRecoveryResumeOperationSchema,
  objectRecoveryResumeRequestSchema,
} from "../src/shared/object-recovery";
import { ObjectCandidateReviewPanel } from "../src/ui/object-tasks/ObjectCandidateReviewPanel";
import { prepareCandidateRework } from "../src/ui/object-tasks/ObjectCandidateReworkPanel";
import { ObjectTaskResumePanel } from "../src/ui/object-tasks/ObjectTaskResumePanel";
import { execution, session } from "./fixtures/object-attempts";
import { candidateReport } from "./fixtures/object-candidate-review";
import { verifiedView } from "./fixtures/object-recovery";
import { ProjectDerivationPanel } from "../src/ui/ProjectDerivationPanel";

for (const schemaVersion of [1, 2] as const)
  for (const staged of [false, true])
    test(`derived v${schemaVersion} ${staged ? "cross-stage" : "single-stage"} review stays historical and explicit rework selects the fresh record`, async () => {
      const attempt = { ...execution("awaitingGate").attempt, taskRevision: 3 };
      const saved = candidateReport(
        {
          projectId: "p",
          requestId: "derived-history",
          checkRequestId: "derived-check",
          target: attempt.target,
        },
        schemaVersion,
      );
      if (staged)
        saved.stages.unshift({
          taskId: "derived-accepted-fine",
          title: "Accepted foundation",
          revision: 9,
          status: "accepted",
          acceptance: "Foundation remains unchanged",
        });
      const older = structuredClone(saved);
      older.request.requestId = "derived-earlier-history";
      older.request.checkRequestId = "derived-earlier-check";
      older.time = "earlier-review";
      const historical = structuredClone([older, saved]);
      const original = structuredClone(saved);
      const reports = [older, saved];
      const calls: string[] = [];
      const submitted: unknown[] = [];
      let launches = 0;
      let sequence = 0;
      const owner = session(
        async (method, input) => {
          calls.push(method);
          if (method === "objectTask.candidateReviews") return reports;
          if (method === "objectTask.recovery") return verifiedView();
          if (method === "objectTask.attempts") return [];
          if (method === "objectTask.checkAttempt")
            return objectAttemptCheckReportSchema.parse({
              request: input,
              runnerVersion: 1,
              attemptDigest: "f".repeat(64),
              inputDigest: "c".repeat(64),
              outputDigest: saved.outputDigest,
              time: "fresh-check",
              passed: true,
              rules: saved.rules,
            });
          if (method === "objectTask.prepareCandidateReview") {
            const fresh = candidateReport(input);
            fresh.stages = structuredClone(saved.stages);
            fresh.time = "fresh-review";
            fresh.sourceDigest = "e".repeat(64);
            reports.push(fresh);
            return fresh;
          }
          assert.equal(method, "objectTask.reworkCandidate");
          const request = objectRecoveryResumeRequestSchema.parse(input);
          submitted.push(structuredClone(request));
          if (
            historical.some(
              (report) =>
                request.rework?.reviewRequestId === report.request.requestId,
            )
          )
            throw Error("OBJECT_CANDIDATE_REWORK_SOURCE_MISMATCH");
          assert.ok(request.rework);
          const fresh = reports.at(-1);
          assert.ok(fresh);
          assert.equal(request.rework.reviewRequestId, fresh.request.requestId);
          assert.equal(request.rework.feedback, "Reduce movement speed");
          launches++;
          return objectRecoveryResumeOperationSchema.parse({
            schemaVersion: 1,
            request,
            result: {
              status: "started",
              report: verifiedView().operation!.result,
              attemptId: "fresh-rework",
              fineTaskId: "fine",
            },
          });
        },
        async () => true,
        "p",
        () => `fresh-${++sequence}`,
      );
      const review = owner.candidateFor(attempt);
      await review.refresh();
      assert.deepEqual(calls, ["objectTask.candidateReviews"]);
      assert.deepEqual(review.getSnapshot().reports, historical);
      assert.equal(
        prepareCandidateRework(owner, attempt, saved, "Feedback"),
        false,
      );
      assert.equal(launches, 0);

      await owner.recovery.refresh();
      assert.equal(
        prepareCandidateRework(owner, attempt, saved, "Feedback"),
        true,
      );
      assert.equal(submitted.length, 0);
      await owner.recovery.resume.submit();
      assert.equal(
        owner.recovery.resume.getSnapshot().error,
        "OBJECT_CANDIDATE_REWORK_SOURCE_MISMATCH",
      );
      assert.equal(owner.recovery.resume.getSnapshot().retryAvailable, false);
      assert.equal(launches, 0);

      const checks = owner.checksFor(attempt);
      await checks.run();
      const check = checks.getSnapshot().reports.at(-1)!;
      assert.equal(check.passed, true);
      await review.prepare(check.request.requestId);
      const fresh = review.getSnapshot().reports.at(-1)!;
      assert.equal(fresh.request.checkRequestId, check.request.requestId);
      assert.notEqual(fresh.request.requestId, saved.request.requestId);
      assert.deepEqual(review.getSnapshot().reports, [...historical, fresh]);
      assert.deepEqual([older, saved], historical);
      const html = renderToStaticMarkup(
        createElement(ObjectCandidateReviewPanel, {
          session: review,
          checks,
          execution: owner,
          attempt,
        }),
      );
      assert.ok(html.includes("历史审阅不是新的返工授权"));
      for (const report of [...historical, fresh])
        assert.ok(html.includes(report.request.requestId));
      if (staged) {
        assert.ok(html.includes("Accepted foundation"));
        assert.ok(html.includes("Foundation remains unchanged"));
        assert.deepEqual(fresh.stages, original.stages);
      }
      assert.equal(launches, 0);

      assert.equal(
        prepareCandidateRework(owner, attempt, fresh, "Reduce movement speed"),
        true,
      );
      const confirmation = owner.recovery.resume.getSnapshot().confirmation!;
      assert.equal(
        confirmation.rework?.reviewRequestId,
        fresh.request.requestId,
      );
      assert.equal(confirmation.verificationRequestId, "verify-1");
      assert.match(
        renderToStaticMarkup(
          createElement(ObjectTaskResumePanel, { session: owner.recovery }),
        ),
        /Reduce movement speed/,
      );
      assert.equal(submitted.length, 1);
      await owner.recovery.resume.submit();
      assert.equal(launches, 1);
      assert.deepEqual(submitted[1], confirmation);
      await owner.recovery.resume.submit();
      assert.equal(submitted.length, 2);

      const reopenCalls: string[] = [];
      const reopened = session(async (method) => {
        reopenCalls.push(method);
        assert.equal(method, "objectTask.candidateReviews");
        return reports;
      }).candidateFor(attempt);
      await reopened.refresh();
      assert.deepEqual(reopened.getSnapshot().reports, [...historical, fresh]);
      assert.deepEqual(reopenCalls, ["objectTask.candidateReviews"]);
      assert.equal(launches, 1);
      owner.cancel();
      reopened.cancel();
    });

test("production derivation panel supports text rework history without treating it as new authority", () => {
  const html = renderToStaticMarkup(
    createElement(ProjectDerivationPanel, {
      busy: false,
      perform: async () => {
        throw Error("Rendering must not execute");
      },
    }),
  );
  for (const text of [
    "按顺序显式接受并跨细修推进的历史",
    "保留已接受细修、推进回执",
    "一次或多次纯文本候选返工历史",
    "用户反馈原文保持",
    "只转换系统生成反馈尾缀中的身份引用",
    "历史审阅不是新的返工授权",
    "重新核验",
    "包含图片或预览帧或区域重定位的返工历史",
    "顺序不明的并列细修",
  ])
    assert.ok(html.includes(text), text);
});
