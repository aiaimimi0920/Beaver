import assert from "node:assert/strict";
import test from "node:test";
import {
  objectAttemptTargetSchema,
  objectExecutionSchema,
} from "../src/shared/object-attempts";
import { ObjectTaskExecution } from "../src/ui/object-tasks/object-task-execution";
import { objectTaskSnapshot } from "./fixtures/object-tasks";
import { deferred, execution, session } from "./fixtures/object-attempts";

test("execution query is scoped to the reviewed medium and uses live task revisions", async () => {
  const calls: unknown[] = [];
  const active = execution();
  const controller = session(async (method, input) => {
    calls.push({ method, input });
    return [active];
  });
  assert.equal(await controller.refresh(), true);
  assert.deepEqual(calls, [
    {
      method: "objectTask.attempts",
      input: { projectId: "p", runId: "run-hero" },
    },
  ]);
  assert.deepEqual(controller.getSnapshot().executions, [active]);
  active.attempt.taskRevision = 99;
  assert.equal(controller.getSnapshot().executions[0]?.attempt.taskRevision, 2);
  for (const taskId of ["coarse", "fine", "missing"]) {
    assert.throws(
      () =>
        new ObjectTaskExecution(
          "p",
          objectTaskSnapshot(),
          taskId,
          async () => [],
          async () => true,
        ),
    );
  }
  assert.throws(
    () =>
      new ObjectTaskExecution(
        "other",
        objectTaskSnapshot(),
        "medium",
        async () => [],
        async () => true,
      ),
  );
});

test("execution queries reject foreign lineage and duplicate attempt identities", async () => {
  for (const key of ["taskId", "fineTaskId", "runId", "objectId"] as const) {
    const foreign = execution();
    foreign.attempt.target[key] = "other";
    const controller = session(async () => [foreign]);
    assert.equal(await controller.refresh(), false, key);
    assert.equal(controller.getSnapshot().phase, "failed");
    assert.deepEqual(controller.getSnapshot().executions, []);
  }
  const foreign = execution();
  foreign.attempt.projectId = "other";
  for (const response of [[foreign], [execution(), execution()]]) {
    const controller = session(async () => response);
    assert.equal(await controller.refresh(), false);
    assert.match(
      controller.getSnapshot().error,
      /执行记录与当前项目或任务不一致/,
    );
  }
});

test("execution contracts require explicit nullable bindings and honest availability", () => {
  const active = execution();
  for (const key of Object.keys(active.attempt.target)) {
    const target: Record<string, unknown> = { ...active.attempt.target };
    delete target[key];
    assert.equal(
      objectAttemptTargetSchema.safeParse(target).success,
      false,
      key,
    );
  }
  for (const invalid of [
    { ...active, availability: "finished" },
    { ...active, attempt: { ...active.attempt, outputCaptured: true } },
    execution("failed", "active"),
    execution("interrupted", "recoveryRequired"),
    { ...active, execute: true },
    { ...active, definition: undefined },
    { ...active, definition: { ...active.definition, revision: -1 } },
    { ...active, definition: { ...active.definition, prompt: null } },
    { ...active, attempt: { ...active.attempt, taskRevision: -1 } },
    {
      ...active,
      attempt: {
        ...active.attempt,
        target: { ...active.attempt.target, generation: 0 },
      },
    },
  ])
    assert.equal(objectExecutionSchema.safeParse(invalid).success, false);
  assert.equal(
    objectExecutionSchema.safeParse(execution("running", "recoveryRequired"))
      .success,
    true,
  );
});

test("only an active live attempt can start interruption", async () => {
  for (const item of [
    execution("running", "recoveryRequired"),
    execution("awaitingGate"),
    execution("failed"),
    execution("interrupted"),
  ]) {
    let calls = 0;
    const controller = session(async () => {
      calls++;
      return [item];
    });
    assert.equal(await controller.interrupt("attempt-1"), null);
    await controller.refresh();
    assert.equal(await controller.interrupt("attempt-1"), null);
    assert.equal(await controller.interrupt("unknown"), null);
    assert.equal(calls, 1);
  }
});

test("cancelled queries cannot overwrite a later refresh or switched project", async () => {
  const old = deferred<unknown>();
  let reads = 0;
  const controller = session(async () =>
    ++reads === 1 ? old.promise : [execution("failed")],
  );
  const pending = controller.refresh();
  assert.equal(await controller.refresh(), false);
  controller.cancel();
  assert.equal(await controller.refresh(), true);
  old.resolve([execution()]);
  assert.equal(await pending, false);
  assert.equal(controller.getSnapshot().executions[0]?.attempt.state, "failed");
  const next = session(
    async () => [],
    async () => true,
    "next",
  );
  assert.equal(await next.refresh(), true);
  assert.deepEqual(next.getSnapshot().executions, []);
});

test("cancellation at query notification prevents dispatch", async () => {
  let calls = 0;
  const controller = session(async () => {
    calls++;
    return [];
  });
  controller.subscribe(() => {
    if (controller.getSnapshot().refreshing) controller.cancel();
  });
  assert.equal(await controller.refresh(), false);
  assert.equal(calls, 0);
});
