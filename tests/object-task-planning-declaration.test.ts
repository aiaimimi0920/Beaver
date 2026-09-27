import assert from "node:assert/strict";
import test from "node:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import type { PlanningState } from "../src/shared/object-task-planning-declaration";
import { ObjectTaskBoard } from "../src/ui/object-tasks/ObjectTaskBoard";
import { PlanningDeclarationSubmission } from "../src/ui/object-tasks/object-task-planning-declaration";
import { objectTaskSnapshot } from "./fixtures/object-tasks";

const state = (): PlanningState => ({
  taskId: "coarse",
  scopeHash: "a".repeat(64),
  blockers: [],
  declaration: null,
  current: false,
});
test("lost response retries frozen scope and owner reason after background changes", async () => {
  const requests: unknown[] = [];
  const session = new PlanningDeclarationSubmission(
    "p",
    "coarse",
    async (method, input) => {
      assert.equal(method, "objectTask.declarePlanningComplete");
      requests.push(input);
      if (requests.length === 1) throw new Error("connection lost");
      return {
        request: input,
        declaredBy: "owner",
        createdAt: "today",
        taskIds: ["coarse", "medium", "fine"],
      };
    },
    () => "request-1",
  );
  await assert.rejects(
    session.submit(state(), 1, "Reviewed scope"),
    /connection lost/,
  );
  assert.equal(session.pending, true);
  assert.equal(
    await session.submit(
      { ...state(), scopeHash: "b".repeat(64), current: true },
      2,
      "Changed note",
    ),
    true,
  );
  assert.deepEqual(requests[0], requests[1]);
  assert.equal(session.pending, false);
});
test("gaps and overlong reasons do not send requests; reset permits a fresh scope", async () => {
  let calls = 0;
  const session = new PlanningDeclarationSubmission(
    "p",
    "coarse",
    async () => {
      calls++;
      throw new Error("stale scope");
    },
    () => `request-${calls}`,
  );
  await assert.rejects(
    session.submit({ ...state(), blockers: ["Unplanned"] }, 1, "Reviewed"),
  );
  await assert.rejects(session.submit(state(), 1, "界".repeat(667)));
  assert.equal(calls, 0);
  await assert.rejects(session.submit(state(), 1, "Reviewed"), /stale scope/);
  session.reset();
  assert.equal(session.pending, false);
  await assert.rejects(
    session.submit({ ...state(), scopeHash: "b".repeat(64) }, 2, "New scope"),
    /stale scope/,
  );
  assert.equal(calls, 2);
});
test("production board shows current and stale declarations independently from acceptance", () => {
  const snapshot = objectTaskSnapshot();
  snapshot.planningStates = [state()];
  const render = () =>
    renderToStaticMarkup(
      createElement(ObjectTaskBoard, {
        snapshot,
        disabled: false,
        execute: () => {},
        refresh: async () => {},
      }),
    );
  assert.match(render(), /尚未声明规划完整/);
  snapshot.planningStates[0] = {
    ...state(),
    current: true,
    declaration: {
      request: {
        projectId: "p",
        taskId: "coarse",
        requestId: "r",
        expectedPlanRevision: 1,
        expectedScopeHash: state().scopeHash,
        reason: "All requirements reviewed",
      },
      declaredBy: "owner",
      createdAt: "today",
      taskIds: ["coarse", "medium", "fine"],
    },
  };
  assert.match(render(), /owner 已确认当前范围规划完整/);
  assert.match(render(), /任务验收、对象发布和目标完成仍需分别满足门槛/);
  assert.match(render(), /All requirements reviewed/);
  snapshot.planningStates[0].current = false;
  assert.match(render(), /规划声明已失效/);
  assert.equal(
    snapshot.tasks.find((task) => task.id === "coarse")?.status,
    "planned",
  );
});
