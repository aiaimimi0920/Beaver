import assert from "node:assert/strict";
import test from "node:test";
import type { ImportPreparationRequest } from "../src/shared/object-import";
import { ObjectImportSession } from "../src/ui/object-preview/object-import-session";
import { deferred, inspection, receipt } from "./fixtures/object-import";

type Api = ConstructorParameters<typeof ObjectImportSession>[1];
function session(api: Api, target: string | undefined = "target-1") {
  let sequence = 0;
  const result = new ObjectImportSession(
    target,
    api,
    () => `request-${++sequence}`,
  );
  result.setSource("path", " C:\\source ");
  result.setSource("sourceProjectId", " source-1 ");
  return result;
}

test("preparation pins the original target and inspected accepted version, skipping candidates", async () => {
  const calls: Array<{ method: string; input: unknown }> = [];
  let currentTarget = "target-at-open";
  const operation = session(async (method, input) => {
    calls.push({ method, input });
    return method === "object.inspectExternal"
      ? inspection()
      : receipt(input as ImportPreparationRequest);
  }, currentTarget);
  currentTarget = "new-current-project";
  await operation.inspect();
  assert.equal(operation.selectedVersion?.manifest?.name, "Frozen hero 2");
  assert.deepEqual(
    operation.selectedVersion?.manifest?.files.map((file) => file.path),
    ["hero-2.tscn"],
  );
  const result = await operation.prepare();
  assert.equal(result?.targetProjectId, "target-at-open");
  assert.notEqual(result?.targetProjectId, currentTarget);
  assert.deepEqual(calls[1], {
    method: "object.prepareImport",
    input: {
      requestId: "request-1",
      targetProjectId: "target-at-open",
      source: { path: "C:\\source", projectId: "source-1" },
      objectId: "hero",
      baseline: { kind: "pinnedVersion", versionId: "hero-v2" },
      sourceDigest: "b".repeat(64),
    },
  });
  await operation.prepare();
  assert.equal(calls.length, 2, "a prepared operation must not submit again");
  assert.equal(operation.getSnapshot().phase, "prepared");
});

test("a lost preparation response retries the exact same request identity", async () => {
  const requests: ImportPreparationRequest[] = [];
  const operation = session(async (method, input) => {
    if (method === "object.inspectExternal") return inspection();
    const request = input as ImportPreparationRequest;
    requests.push(request);
    if (requests.length === 1)
      throw new Error("connection lost after server persisted receipt");
    return receipt(request);
  });
  await operation.inspect();
  await operation.prepare();
  assert.match(operation.getSnapshot().error, /connection lost/);
  assert.equal(operation.canPrepare, true);
  const result = await operation.prepare();
  assert.equal(requests[0], requests[1]);
  assert.equal(result?.requestId, "request-1");
  assert.equal(operation.getSnapshot().error, "");
});

test("selecting a registered source atomically replaces its identity and clears the prior receipt", async () => {
  const requests: ImportPreparationRequest[] = [];
  const operation = session(async (method, input) => {
    if (method === "object.inspectExternal")
      return inspection((input as { projectId: string }).projectId);
    const request = input as ImportPreparationRequest;
    requests.push(request);
    return receipt(request);
  });
  await operation.inspect();
  await operation.prepare();
  const changes: Array<{ path: string; sourceProjectId: string }> = [];
  const unsubscribe = operation.subscribe(() =>
    changes.push(operation.getSnapshot()),
  );
  operation.selectSource({ path: "C:\\registered", projectId: "registered-1" });
  unsubscribe();
  assert.equal(changes.length, 1);
  assert.equal(changes[0]?.path, "C:\\registered");
  assert.equal(changes[0]?.sourceProjectId, "registered-1");
  assert.equal(operation.getSnapshot().receipt, null);
  assert.equal(operation.getSnapshot().error, "");
  assert.equal(operation.getSnapshot().objectId, "");
  assert.equal(operation.getSnapshot().versionId, "");
  assert.deepEqual(operation.getSnapshot().objects, []);
  assert.equal(operation.canPrepare, false);
  await operation.inspect();
  const next = await operation.prepare();
  assert.equal(next?.requestId, "request-2");
  assert.deepEqual(requests[1]?.source, {
    path: "C:\\registered",
    projectId: "registered-1",
  });
  operation.selectSource({ path: "C:\\registered", projectId: "registered-1" });
  assert.equal(operation.getSnapshot().receipt, next);
});

test("version, object and source edits clear receipts and allocate new import identities", async () => {
  const requests: ImportPreparationRequest[] = [];
  const operation = session(async (method, input) => {
    if (method === "object.inspectExternal")
      return inspection((input as { projectId: string }).projectId);
    const request = input as ImportPreparationRequest;
    requests.push(request);
    return receipt(request);
  });
  await operation.inspect();
  await operation.prepare();
  operation.selectVersion("hero-v1");
  assert.equal(operation.getSnapshot().receipt, null);
  await operation.prepare();
  operation.selectObject("prop");
  assert.equal(operation.getSnapshot().receipt, null);
  await operation.prepare();
  for (const [field, value] of [
    ["path", "C:\\other-source"],
    ["sourceProjectId", "source-2"],
  ] as const) {
    operation.setSource(field, value);
    assert.equal(operation.getSnapshot().receipt, null);
    assert.deepEqual(operation.getSnapshot().objects, []);
    assert.equal(operation.canPrepare, false);
    await operation.inspect();
    await operation.prepare();
  }
  assert.deepEqual(
    requests.map((request) => request.requestId),
    ["request-1", "request-2", "request-3", "request-4", "request-5"],
  );
  assert.equal(requests[1]?.baseline.versionId, "hero-v1");
  assert.equal(requests[1]?.sourceDigest, "a".repeat(64));
  assert.equal(requests[2]?.objectId, "prop");
  assert.equal(requests[4]?.source.projectId, "source-2");
});

test("late inspection success and failure cannot overwrite a newer source selection", async () => {
  for (const fail of [false, true]) {
    const old = deferred();
    const current = deferred();
    let calls = 0;
    const operation = session(async () =>
      ++calls === 1 ? old.promise : current.promise,
    );
    const first = operation.inspect();
    operation.selectSource({ path: "C:\\registered", projectId: "source-2" });
    const second = operation.inspect();
    if (fail) {
      old.reject(new Error("stale inspection error"));
      await first;
      assert.equal(operation.getSnapshot().phase, "inspecting");
    }
    current.resolve(inspection("source-2"));
    await second;
    if (!fail) {
      old.resolve(inspection());
      await first;
    }
    assert.equal(operation.getSnapshot().phase, "ready");
    assert.equal(operation.selectedVersion?.manifest?.projectId, "source-2");
    assert.equal(operation.getSnapshot().error, "");
  }
});

test("changing context or closing while preparation is pending discards its late receipt", async () => {
  const edits = [
    (operation: ObjectImportSession) =>
      operation.setSource("path", "C:\\other"),
    (operation: ObjectImportSession) =>
      operation.selectSource({ path: "C:\\registered", projectId: "source-2" }),
    (operation: ObjectImportSession) => operation.selectVersion("hero-v1"),
    (operation: ObjectImportSession) => operation.selectObject("prop"),
    (operation: ObjectImportSession) => operation.cancel(),
  ];
  for (const edit of edits) {
    const pending = deferred();
    let request: ImportPreparationRequest | undefined;
    const operation = session(async (method, input) => {
      if (method === "object.inspectExternal") return inspection();
      request = input as ImportPreparationRequest;
      return pending.promise;
    });
    await operation.inspect();
    const result = operation.prepare();
    assert.equal(operation.getSnapshot().phase, "preparing");
    assert.equal(
      await operation.prepare(),
      undefined,
      "double submit is ignored",
    );
    edit(operation);
    assert.ok(request);
    pending.resolve(receipt(request));
    assert.equal(await result, undefined);
    assert.equal(operation.getSnapshot().receipt, null);
    assert.notEqual(operation.getSnapshot().phase, "prepared");
  }
});

test("blocked versions and unavailable targets cannot submit preparations", async () => {
  for (const target of [undefined, "source-1", "target-1"]) {
    let calls = 0;
    const operation = new ObjectImportSession(target, async () => {
      calls++;
      return inspection();
    });
    operation.setSource("path", "C:\\source");
    operation.setSource("sourceProjectId", "source-1");
    await operation.inspect();
    if (target === "target-1") operation.selectVersion("hero-v3");
    assert.equal(operation.canPrepare, false);
    assert.equal(await operation.prepare(), undefined);
    assert.equal(calls, 1);
  }
});

test("malformed inspection and mismatched receipt fail visibly without claiming success", async () => {
  let invalidInspection = true;
  const operation = session(async (method, input) => {
    if (method === "object.inspectExternal")
      return invalidInspection ? { objects: [] } : inspection();
    return {
      ...receipt(input as ImportPreparationRequest),
      targetProjectId: "wrong-target",
    };
  });
  await operation.inspect();
  assert.equal(operation.canPrepare, false);
  assert.ok(operation.getSnapshot().error);
  invalidInspection = false;
  await operation.inspect();
  assert.equal(await operation.prepare(), undefined);
  assert.match(operation.getSnapshot().error, /回执与当前请求不一致/);
  assert.equal(operation.getSnapshot().receipt, null);
  assert.equal(operation.getSnapshot().phase, "ready");
});
