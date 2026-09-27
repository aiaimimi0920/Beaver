import assert from "node:assert/strict";
import test from "node:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { ObjectTaskRevision } from "../src/ui/object-tasks/object-task-revision";
import { ObjectTaskRevisionDialog } from "../src/ui/object-tasks/ObjectTaskRevisionDialog";
import {
  objectTaskDefinition,
  revisePlannedObjectTaskSchema,
} from "../src/shared/object-task-revisions";
import { objectTaskSnapshot } from "./fixtures/object-tasks";
import { execution, deferred } from "./fixtures/object-attempts";

function fixture() {
  const snapshot = objectTaskSnapshot();
  const medium = snapshot.tasks.find((task) => task.id === "medium")!;
  const fine = snapshot.tasks.find((task) => task.id === "fine")!;
  snapshot.tasks.push({
    ...medium,
    id: "old-medium",
    runId: "old-run",
    status: "cancelled",
  });
  snapshot.tasks.push({
    ...fine,
    id: "old-fine",
    parentTaskId: "old-medium",
    runId: "old-run",
    status: "cancelled",
    identity: {
      schemaVersion: 1,
      layer: "fine",
      objectId: "hero",
      mediumTaskId: "old-medium",
      runId: "old-run",
      stageId: "movement",
    },
  });
  snapshot.runs.push({
    ...snapshot.runs[0]!,
    id: "old-run",
    mediumTaskId: "old-medium",
    status: "cancelled",
  });
  const entry = execution("interrupted");
  Object.assign(entry.attempt.target, {
    taskId: "old-medium",
    fineTaskId: "old-fine",
    runId: "old-run",
  });
  entry.definition.prompt = "Historical <script> prompt\nSecond line";
  entry.definition.acceptance = "";
  const make = (api: (method: string, input: unknown) => Promise<unknown>) =>
    new ObjectTaskRevision(
      "p",
      snapshot,
      "fine",
      api,
      async () => true,
      () => "reuse-1",
    );
  return { snapshot, fine, entry, make };
}

test("historical prompts load locally, preserve target identity and persist only after review and confirmation", async () => {
  const { make, fine, entry } = fixture();
  const original = structuredClone(entry);
  const calls: string[] = [];
  let saved: unknown;
  const session = make(async (method, input) => {
    calls.push(method);
    if (method === "objectTask.attempts") {
      assert.deepEqual(input, { projectId: "p", runId: "old-run" });
      return [entry];
    }
    if (method === "objectTask.revisions") return [saved];
    assert.equal(method, "objectTask.revisePlanned");
    const request = revisePlannedObjectTaskSchema.parse(input);
    assert.equal(request.taskId, "fine");
    assert.equal(request.expectedTaskRevision, 0);
    assert.equal(request.expectedPlanRevision, 1);
    assert.equal(
      request.reason,
      "Reuse controls\n历史提示词来源：run old-run / fine old-fine / attempt attempt-1 / definition revision 1",
    );
    saved = {
      projectId: "p",
      taskId: "fine",
      requestId: request.requestId,
      previousTaskRevision: 0,
      taskRevision: 1,
      previousPlanRevision: 1,
      planRevision: 2,
      before: objectTaskDefinition(fine),
      after: request.definition,
      reason: request.reason,
      adoptedBy: "owner",
      createdAt: "2026-09-25T12:00:00Z",
      affectedTaskIds: ["fine"],
    };
    return saved;
  });
  session.updateDefinition({ title: "Local title" });
  session.updateReason("Reuse controls");
  assert.equal(await session.promptHistory.load("old-medium"), true);
  assert.equal(session.loadHistoricalPrompt("attempt-1"), true);
  assert.deepEqual(session.getSnapshot().definition, {
    title: "Local title",
    prompt: entry.definition.prompt,
    acceptance: "",
    dependsOn: fine.dependsOn,
  });
  assert.equal(session.getSnapshot().reason, "Reuse controls");
  assert.deepEqual(entry, original);
  assert.equal(await session.submit(), null);
  assert.deepEqual(calls, ["objectTask.attempts"]);
  assert.equal(session.review(), true);
  assert.equal(session.loadHistoricalPrompt("attempt-1"), false);
  const html = renderToStaticMarkup(
    createElement(ObjectTaskRevisionDialog, { session, close: () => {} }),
  );
  assert.match(html, /Historical &lt;script&gt; prompt/);
  assert.match(html, /old-run/);
  assert.match(html, /确认修订/);
  assert.ok(await session.submit());
  assert.equal(session.getSnapshot().phase, "succeeded");
  assert.deepEqual(entry, original);
});

test("wrong-project, object, run, fine and duplicate responses cannot populate history", async () => {
  for (const kind of [
    "project",
    "object",
    "run",
    "fine",
    "duplicate",
  ] as const) {
    const { make, entry } = fixture();
    if (kind === "project") entry.attempt.projectId = "other";
    if (kind === "object") entry.attempt.target.objectId = "other";
    if (kind === "run") entry.attempt.target.runId = "other";
    if (kind === "fine") entry.attempt.target.fineTaskId = "fine";
    const session = make(async () =>
      kind === "duplicate" ? [entry, entry] : [entry],
    );
    const original = structuredClone(session.getSnapshot().definition);
    assert.equal(await session.promptHistory.load("old-medium"), false, kind);
    assert.equal(session.loadHistoricalPrompt("attempt-1"), false, kind);
    assert.deepEqual(session.getSnapshot().definition, original);
    assert.deepEqual(session.promptHistory.getSnapshot().entries, []);
  }
});

test("late history responses after replacement or closing cannot restore stale selection", async () => {
  const { make, entry } = fixture();
  const pending = deferred<unknown>();
  let calls = 0;
  const session = make(async () => (++calls === 1 ? pending.promise : []));
  const first = session.promptHistory.load("old-medium");
  assert.equal(await session.promptHistory.load("old-medium"), true);
  pending.resolve([entry]);
  assert.equal(await first, false);
  assert.deepEqual(session.promptHistory.getSnapshot().entries, []);
  const closing = deferred<unknown>();
  const closed = make(async () => closing.promise);
  const waiting = closed.promptHistory.load("old-medium");
  closed.cancel();
  closing.resolve([entry]);
  assert.equal(await waiting, false);
  assert.equal(closed.loadHistoricalPrompt("attempt-1"), false);
});

test("only same-object historical iterations are offered; started targets and empty reasons stay blocked", async () => {
  const { make, entry, snapshot, fine } = fixture();
  const session = make(async () => [entry]);
  assert.deepEqual(
    session.promptHistory.sources().map((task) => task.id),
    ["old-medium"],
  );
  assert.equal(await session.promptHistory.load("medium"), false);
  await session.promptHistory.load("old-medium");
  assert.equal(session.loadHistoricalPrompt("attempt-1"), true);
  assert.equal(session.review(), false);
  assert.equal(session.getSnapshot().phase, "editing");
  fine.status = "running";
  snapshot.tasks.find((task) => task.id === "medium")!.status = "running";
  snapshot.runs.find((run) => run.id === "run-hero")!.status = "running";
  const readonly = make(async () => [entry]);
  await readonly.promptHistory.load("old-medium");
  assert.equal(readonly.loadHistoricalPrompt("attempt-1"), false);
  assert.equal(readonly.getSnapshot().phase, "readonly");
  const html = renderToStaticMarkup(
    createElement(ObjectTaskRevisionDialog, {
      session: readonly,
      close: () => {},
    }),
  );
  assert.doesNotMatch(html, /<summary>载入历史提示词/);
});
