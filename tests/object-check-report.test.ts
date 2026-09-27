import assert from "node:assert/strict";
import test from "node:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import {
  objectAttemptCheckReportSchema,
  type ObjectAttemptCheckReport,
  type ObjectAttemptCheckRequest,
} from "../src/shared/object-attempt-checks";
import { ObjectCheckReportQuery } from "../src/ui/validation/object-check-report";
import { ObjectCheckReportContent } from "../src/ui/validation/ObjectCheckReportView";
import { ObjectCheckNavigation } from "../src/ui/validation/object-check-navigation";
import { ObjectAttemptChecks } from "../src/ui/object-tasks/object-attempt-checks";
import { ObjectAttemptCheckPanel } from "../src/ui/object-tasks/ObjectAttemptCheckPanel";
import { execution, deferred } from "./fixtures/object-attempts";

function report(requestId: string) {
  return objectAttemptCheckReportSchema.parse({
    request: {
      projectId: "p",
      requestId,
      target: execution("failed").attempt.target,
    },
    runnerVersion: 1,
    attemptDigest: "a".repeat(64),
    inputDigest: "b".repeat(64),
    outputDigest: "c".repeat(64),
    time: "2026-09-25T12:00:00Z",
    passed: false,
    rules: [
      {
        id: "checkpoint-integrity",
        version: 1,
        passed: true,
        issues: [],
        filesChecked: 2,
      },
      {
        id: "code-structure",
        version: 1,
        passed: false,
        issues: ["source.gd: 701 effective lines"],
        filesChecked: 1,
      },
    ],
  });
}

function evidence(saved: ObjectAttemptCheckReport) {
  return {
    source: {
      kind: "objectAttempt",
      projectId: saved.request.projectId,
      target: saved.request.target,
      stageId: "stage-b",
      candidateReviews: [
        {
          requestId: "candidate-review",
          sourceDigest: "d".repeat(64),
          outputDigest: saved.outputDigest,
        },
      ],
    },
    report: saved,
  };
}

test("test page reopens the exact persisted report and source; reading never runs checks", async () => {
  const saved = report("historical");
  const calls: { method: string; input: unknown }[] = [];
  const query = new ObjectCheckReportQuery(
    "p",
    saved.request,
    async (method, input) => {
      calls.push({ method, input });
      return evidence(saved);
    },
  );
  await query.refresh();
  assert.deepEqual(query.getSnapshot().report, saved);
  query.cancel();
  await query.refresh();
  assert.deepEqual(
    calls,
    Array(2).fill({
      method: "validation.objectReport.get",
      input: {
        projectId: "p",
        attemptId: saved.request.target.attemptId,
        requestId: "historical",
      },
    }),
  );
  const html = renderToStaticMarkup(
    createElement(ObjectCheckReportContent, {
      session: query,
      openManufacture: () => {},
      openWorkbench: () => {},
    }),
  );
  for (const text of [
    "historical",
    "source.gd: 701 effective lines",
    "返回对应制造记录",
    "不代表当前工作区状态",
    saved.request.target.objectId,
    saved.outputDigest,
    "stage-b",
    "candidate-review",
    "d".repeat(64),
  ])
    assert.ok(html.includes(text), text);
  assert.ok(!html.includes("latest"));
  assert.ok(!html.includes("发起新检查"));
});

test("missing reports and changed source identities fail closed without falling back", async () => {
  const saved = report("exact");
  for (const response of [
    null,
    evidence(report("different-request")),
    evidence({ ...saved, request: { ...saved.request, projectId: "other" } }),
    evidence({
      ...saved,
      request: {
        ...saved.request,
        target: { ...saved.request.target, runId: "other" },
      },
    }),
    evidence({
      ...saved,
      request: {
        ...saved.request,
        target: { ...saved.request.target, fineTaskId: "other" },
      },
    }),
    { ...evidence(saved), source: { ...evidence(saved).source, stageId: "" } },
    {
      ...evidence(saved),
      source: { ...evidence(saved).source, projectId: "other" },
    },
    {
      ...evidence(saved),
      source: {
        ...evidence(saved).source,
        candidateReviews: Array(2).fill(
          evidence(saved).source.candidateReviews[0],
        ),
      },
    },
    {
      ...evidence(saved),
      source: {
        ...evidence(saved).source,
        candidateReviews: [
          {
            ...evidence(saved).source.candidateReviews[0],
            outputDigest: "e".repeat(64),
          },
        ],
      },
    },
  ]) {
    const query = new ObjectCheckReportQuery(
      "p",
      saved.request,
      async () => response,
    );
    await query.refresh();
    assert.equal(query.getSnapshot().report, undefined);
    assert.ok(query.getSnapshot().error);
  }
  assert.throws(
    () => new ObjectCheckReportQuery("other", saved.request, async () => []),
  );
});

test("late project results are discarded and disconnected refresh clears displayed evidence", async () => {
  const saved = report("exact");
  const pending = deferred<unknown>();
  let count = 0;
  const query = new ObjectCheckReportQuery("p", saved.request, async () => {
    if (++count === 1) return pending.promise;
    if (count === 2) return evidence(saved);
    throw new Error("disconnected");
  });
  const reading = query.refresh();
  query.cancel();
  pending.resolve(evidence(saved));
  await reading;
  assert.equal(query.getSnapshot().report, undefined);
  await query.refresh();
  assert.deepEqual(query.getSnapshot().report, saved);
  await query.refresh();
  assert.equal(query.getSnapshot().report, undefined);
  assert.match(query.getSnapshot().error, /disconnected/);
  assert.equal(query.getSnapshot().source, undefined);
});

test("manufacturing report exposes test navigation and expands the selected immutable request", async () => {
  const saved = report("exact");
  const session = new ObjectAttemptChecks(
    "p",
    execution("failed").attempt,
    async () => [saved],
  );
  await session.refresh();
  const html = renderToStaticMarkup(
    createElement(
      ObjectCheckNavigation.Provider,
      {
        value: (_request: ObjectAttemptCheckRequest) => {},
      },
      createElement(ObjectAttemptCheckPanel, {
        session,
        selectedRequestId: "exact",
      }),
    ),
  );
  assert.match(html, /在测试页查看此报告/);
  assert.match(html, /<details open="">/);
  assert.ok(html.includes(saved.outputDigest));
  assert.ok(html.includes("source.gd: 701 effective lines"));
});
