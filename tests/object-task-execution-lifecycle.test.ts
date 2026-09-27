import assert from "node:assert/strict";
import test from "node:test";
import {
  deferred,
  execution,
  interruptReceipt,
  session,
} from "./fixtures/object-attempts";

test("closing an old project session ignores its late interruption receipt", async () => {
  const response = deferred<unknown>();
  const projects: string[] = [];
  let oldInput: unknown;
  const old = session(
    async (method, input) => {
      if (method === "objectTask.attempts") return [execution()];
      oldInput = input;
      return response.promise;
    },
    async () => {
      projects.push("p");
      return true;
    },
  );
  await old.refresh();
  const pending = old.interrupt("attempt-1");
  old.cancel();
  const next = session(
    async (method, input) => {
      if (method === "objectTask.attempts") {
        const active = execution();
        active.attempt.projectId = "next";
        return [active];
      }
      return interruptReceipt(input);
    },
    async () => {
      projects.push("next");
      return true;
    },
    "next",
  );
  await next.refresh();
  await next.interrupt("attempt-1");
  response.resolve(interruptReceipt(oldInput));
  assert.equal(await pending, null);
  assert.equal(old.getSnapshot().receipt, null);
  assert.equal(next.getSnapshot().receipt?.result.projectId, "next");
  assert.deepEqual(projects, ["next"]);
});

test("late interruption failures cannot change a cancelled session", async () => {
  const response = deferred<unknown>();
  const controller = session(async (method) =>
    method === "objectTask.attempts" ? [execution()] : response.promise,
  );
  await controller.refresh();
  const pending = controller.interrupt("attempt-1");
  controller.cancel();
  const after = controller.getSnapshot();
  response.reject(new Error("Late failure"));
  assert.equal(await pending, null);
  assert.equal(controller.getSnapshot(), after);
});

test("cancellation at submit notification prevents a new mutation", async () => {
  let writes = 0;
  const controller = session(async (method, input) => {
    if (method === "objectTask.attempts") return [execution()];
    writes++;
    return interruptReceipt(input);
  });
  await controller.refresh();
  const unsubscribe = controller.subscribe(() => {
    if (controller.getSnapshot().phase === "submitting") controller.cancel();
  });
  assert.equal(await controller.interrupt("attempt-1"), null);
  assert.equal(writes, 0);
  unsubscribe();
  assert.ok(await controller.interrupt());
  assert.equal(writes, 1);
});

test("closing on receipt or refresh notification prevents a stale workspace refresh", async () => {
  for (const atRefresh of [false, true]) {
    let refreshes = 0;
    const controller = session(
      async (method, input) =>
        method === "objectTask.attempts"
          ? [execution()]
          : interruptReceipt(input),
      async () => {
        refreshes++;
        return true;
      },
    );
    await controller.refresh();
    controller.subscribe(() => {
      const state = controller.getSnapshot();
      if (state.receipt && (!atRefresh || state.refreshing))
        controller.cancel();
    });
    assert.equal(await controller.interrupt("attempt-1"), null);
    assert.ok(controller.getSnapshot().receipt);
    assert.equal(refreshes, 0);
  }
});

test("a late workspace refresh cannot overwrite state after close", async () => {
  const refreshed = deferred<boolean>();
  const started = deferred<void>();
  const controller = session(
    async (method, input) =>
      method === "objectTask.attempts"
        ? [execution()]
        : interruptReceipt(input),
    async () => {
      started.resolve();
      return refreshed.promise;
    },
  );
  await controller.refresh();
  const pending = controller.interrupt("attempt-1");
  await started.promise;
  controller.cancel();
  const after = controller.getSnapshot();
  refreshed.reject(new Error("Late refresh error"));
  assert.equal(await pending, null);
  assert.equal(controller.getSnapshot(), after);
  assert.equal(after.phase, "succeeded");
  assert.ok(after.receipt);
});
