import assert from "node:assert/strict";
import test from "node:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { ObjectCandidateReview } from "../src/ui/object-tasks/object-candidate-review";
import { ObjectCandidateReviewPanel } from "../src/ui/object-tasks/ObjectCandidateReviewPanel";
import { ObjectTaskExecutionDialog } from "../src/ui/object-tasks/ObjectTaskExecutionDialog";
import { objectCandidateReportSchema } from "../src/shared/object-candidate-review";
import { execution, deferred, session } from "./fixtures/object-attempts";
import { candidateReport as report } from "./fixtures/object-candidate-review";

test("production candidate review preserves an uncertain request, shows baseline and blockers, and only queries on reopen", async () => {
  const sent: unknown[] = [];
  const attempt = execution("awaitingGate").attempt;
  const owner = session(async (method, input) => {
    if (method === "objectTask.attempts") return [execution("awaitingGate")];
    assert.equal(method, "objectTask.prepareCandidateReview");
    sent.push(structuredClone(input));
    if (sent.length === 1) throw new Error("response lost");
    return report(input);
  });
  await owner.refresh();
  assert.match(
    renderToStaticMarkup(
      createElement(ObjectTaskExecutionDialog, { session: owner, close() {} }),
    ),
    /对象整体候选审阅/,
  );
  const review = owner.candidateFor(attempt);
  await review.prepare("check-1");
  assert.equal(review.getSnapshot().retry, true);
  await review.prepare("check-2");
  assert.deepEqual(sent[1], sent[0]);
  const html = renderToStaticMarkup(
    createElement(ObjectCandidateReviewPanel, {
      session: review,
      checks: owner.checksFor(attempt),
      execution: owner,
      attempt,
    }),
  );
  for (const text of [
    "historic",
    "new-current",
    "hero.gd",
    "material-v1",
    "尚未接受或发布",
    "Movement works",
    "当前接受版本已偏离领取时版本",
    "核对文件归属与返工反馈并提交人工接受决定",
    "刷新发布状态与差异",
  ])
    assert.ok(html.includes(text), text);
  assert.ok(!html.includes("尚未接入"));
  const reopened = new ObjectCandidateReview("p", attempt, async (method) => {
    assert.equal(method, "objectTask.candidateReviews");
    return review.getSnapshot().reports;
  });
  await reopened.refresh();
  assert.deepEqual(
    reopened.getSnapshot().reports,
    review.getSnapshot().reports,
  );
});

test("closing prevents late review writes and synchronous close prevents dispatch", async () => {
  const pending = deferred<unknown>();
  let input: unknown;
  const review = new ObjectCandidateReview(
    "p",
    execution("awaitingGate").attempt,
    async (_method, value) => {
      input = value;
      return pending.promise;
    },
    () => "review",
  );
  const work = review.prepare("check");
  review.cancel();
  pending.resolve(report(input));
  await work;
  assert.equal(review.getSnapshot().reports.length, 0);
  assert.equal(review.getSnapshot().retry, true);
  let calls = 0;
  const closed = new ObjectCandidateReview(
    "p",
    execution("awaitingGate").attempt,
    async () => {
      calls++;
      return [];
    },
  );
  const unsubscribe = closed.subscribe(() => {
    if (closed.getSnapshot().busy) closed.cancel();
  });
  await closed.prepare("check");
  unsubscribe();
  assert.equal(calls, 0);
});

test("reopening a cached review completes while a cancelled read is still pending", async () => {
  const old = deferred<unknown>();
  const current = deferred<unknown>();
  let calls = 0;
  const review = new ObjectCandidateReview(
    "p",
    execution("awaitingGate").attempt,
    async () => (++calls === 1 ? old.promise : current.promise),
  );
  const closedRead = review.refresh();
  review.cancel();
  const reopenedRead = review.refresh();
  assert.equal(calls, 2);
  old.resolve([]);
  await closedRead;
  assert.equal(review.getSnapshot().busy, true);
  const saved = report({
    projectId: "p",
    requestId: "saved",
    checkRequestId: "check",
    target: execution("awaitingGate").attempt.target,
  });
  current.resolve([saved]);
  await reopenedRead;
  assert.equal(review.getSnapshot().busy, false);
  assert.deepEqual(review.getSnapshot().reports, [saved]);
});

test("candidate review rejects a foreign receipt and malformed final-stage evidence", async () => {
  const review = new ObjectCandidateReview(
    "p",
    execution("awaitingGate").attempt,
    async (_method, input) => {
      const wrong = report(input);
      wrong.request.projectId = "other";
      return wrong;
    },
  );
  await review.prepare("check");
  assert.equal(review.getSnapshot().retry, true);
  assert.equal(review.getSnapshot().reports.length, 0);
  assert.match(review.getSnapshot().error, /当前尝试不一致/);
  const valid = report({
    projectId: "p",
    requestId: "r",
    checkRequestId: "check",
    target: execution("awaitingGate").attempt.target,
  });
  assert.equal(
    objectCandidateReportSchema.safeParse({ ...valid, blockers: [] }).success,
    false,
  );
  for (const invalid of [
    { ...valid, schemaVersion: 3 },
    { ...valid, schemaVersion: 1 },
    { ...valid, blockers: [...valid.blockers, "PUBLICATION_NOT_IMPLEMENTED"] },
    { ...valid, blockers: [...valid.blockers, "FEEDBACK_REVIEW_UNAVAILABLE"] },
  ])
    assert.equal(objectCandidateReportSchema.safeParse(invalid).success, false);
  assert.equal(
    objectCandidateReportSchema.safeParse({
      ...valid,
      stages: [{ ...valid.stages[0], taskId: "old-fine" }],
    }).success,
    false,
  );
});

test("old frozen review remains readable alongside current publication controls", async () => {
  const attempt = execution("awaitingGate").attempt;
  const saved = report(
    {
      projectId: "p",
      requestId: "old-review",
      checkRequestId: "check",
      target: attempt.target,
    },
    1,
  );
  const calls: string[] = [];
  const owner = session(async (method) => {
    calls.push(method);
    assert.equal(method, "objectTask.candidateReviews");
    return [saved];
  });
  const review = owner.candidateFor(attempt);
  await review.refresh();
  const html = renderToStaticMarkup(
    createElement(ObjectCandidateReviewPanel, {
      session: review,
      checks: owner.checksFor(attempt),
      execution: owner,
      attempt,
    }),
  );
  assert.deepEqual(review.getSnapshot().reports, [saved]);
  assert.deepEqual(calls, ["objectTask.candidateReviews"]);
  assert.ok(html.includes("此为旧版冻结审阅"));
  assert.ok(html.includes("旧版记录：生成时受管发布尚未接入"));
  assert.ok(html.includes("刷新发布状态与差异"));
  assert.ok(html.includes("最终接受与发布"));
});
