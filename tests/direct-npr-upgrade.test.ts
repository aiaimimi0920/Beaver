import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import vm from "node:vm";

const factory = vm.runInNewContext(
  readFileSync("src/direct/npr-upgrade.js", "utf8") +
    "; createNprUpgradeActions",
);
const plain = (value: unknown) => JSON.parse(JSON.stringify(value));
const resolveTarget = vm.runInNewContext(
  readFileSync("src/direct/npr-upgrade.js", "utf8") +
    "; resolveNprUpgradeTarget",
);
type Target = { id: string; taskId?: string };
test("multiple projects resolve through selected task, never the first project", async () => {
  const state = {
    projects: [{ id: "first" }, { id: "second" }],
    tasks: [{ id: "ear", projectId: "second" }],
  };
  assert.throws(() => resolveTarget(state, "project", "ear"), /唯一/);
  assert.throws(() => resolveTarget(state, "task", "missing"), /不匹配/);
  const target = resolveTarget(state, "task", "ear");
  assert.deepEqual(plain(target), { id: "second", taskId: "ear" });
  const calls: unknown[] = [];
  const actions = factory(
    async (method: string, input: Target) => {
      calls.push([method, plain(input)]);
      return receipt(input);
    },
    () => resolveTarget(state, "task", "ear"),
  );
  await actions.inspect();
  await actions.upgrade();
  assert.equal((calls[1] as any[])[1].id, "second");
});
const receipt = (target: Target, eligible = true) => ({
  target: { projectId: target.id, taskId: target.taskId ?? null },
  eligible,
  sourceCommit: "a".repeat(40),
  runtimeSha256: "b".repeat(64),
});

test("upgrade requires a matching eligible inspection and uses exact identities once", async () => {
  const target = { id: "project", taskId: "ear" };
  const calls: unknown[] = [];
  const actions = factory(
    async (method: string, input: unknown) => {
      calls.push([method, plain(input)]);
      return method.endsWith("inspectUpgrade")
        ? receipt(target)
        : { status: "completed" };
    },
    () => target,
  );
  await assert.rejects(actions.upgrade(), /先检查/);
  assert.equal(calls.length, 0);
  await actions.inspect();
  assert.equal(actions.canUpgrade(), true);
  await actions.upgrade();
  assert.deepEqual(calls[1], [
    "project.npr.upgrade",
    {
      ...target,
      expectedOldCommit: "a".repeat(40),
      expectedRuntimeSha256: "b".repeat(64),
    },
  ]);
  await assert.rejects(actions.upgrade(), /先检查/);
  assert.equal(calls.length, 2);
});

test("target changes and blocked inspections cannot authorize an upgrade", async () => {
  let target: Target = { id: "project" };
  const actions = factory(
    async () => receipt(target),
    () => target,
  );
  await actions.inspect();
  target = { id: "project", taskId: "ear" };
  assert.equal(actions.canUpgrade(), false);
  await assert.rejects(actions.upgrade(), /先检查/);
  const blocked = factory(
    async () => receipt(target, false),
    () => target,
  );
  await blocked.inspect();
  await assert.rejects(blocked.upgrade(), /先检查/);
});

test("uncertain native outcome consumes inspection and requires reinspection", async () => {
  const target = { id: "project" };
  const actions = factory(
    async (method: string) => {
      if (method.endsWith("inspectUpgrade")) return receipt(target);
      throw Error("transport interrupted; outcome uncertain");
    },
    () => target,
  );
  await actions.inspect();
  await assert.rejects(actions.upgrade(), /outcome uncertain/);
  assert.equal(actions.canUpgrade(), false);
});

test("foreign response or target change during inspection never enables apply", async () => {
  let target: Target = { id: "project" };
  const foreign = factory(
    async () => receipt({ id: "other" }),
    () => target,
  );
  await assert.rejects(foreign.inspect(), /不同目标/);
  assert.equal(foreign.canUpgrade(), false);
  let complete!: (result: unknown) => void;
  const actions = factory(
    () =>
      new Promise((resolve) => {
        complete = resolve;
      }),
    () => target,
  );
  const inspecting = actions.inspect();
  target = { id: "project", taskId: "ear" };
  complete(receipt({ id: "project" }));
  await inspecting;
  assert.equal(actions.canUpgrade(), false);
});
