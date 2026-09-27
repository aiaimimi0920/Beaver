import assert from "node:assert/strict";
import test from "node:test";
import type { ImportPreparationRequest } from "../src/shared/object-import";
import { ObjectImportSession } from "../src/ui/object-preview/object-import-session";
import { deferred, inspection, receipt } from "./fixtures/object-import";

test("external folder selection discovers identity and prepares against the bound target", async () => {
  const calls: Array<{ method: string; input: unknown }> = [];
  const operation = new ObjectImportSession(
    "target-at-open",
    async (method, input) => {
      calls.push({ method, input });
      if (method === "chooseImportDirectory") return " C:\\external ";
      if (method === "object.inspectExternal") return inspection("external-id");
      return receipt(input as ImportPreparationRequest);
    },
    () => "request-1",
  );
  await operation.choosePaths("project");
  assert.equal(operation.getSnapshot().sourceMode, "external");
  assert.equal(operation.getSnapshot().sourceProjectId, "external-id");
  assert.equal(operation.getSnapshot().path, "C:\\external");
  assert.deepEqual(calls[1], {
    method: "object.inspectExternal",
    input: { path: "C:\\external" },
  });
  assert.equal((await operation.prepare())?.targetProjectId, "target-at-open");
  const request = calls[2]?.input as ImportPreparationRequest;
  assert.deepEqual(request.source, {
    path: "C:\\external",
    projectId: "external-id",
  });
  operation.setSource("path", "C:\\next");
  assert.equal(operation.getSnapshot().sourceProjectId, "");
  assert.equal(operation.getSnapshot().receipt, null);
  assert.deepEqual(operation.getSnapshot().objects, []);
  assert.equal(operation.canPrepare, false);
});

test("registered identity mismatch fails visibly and empty discovery cannot prepare", async () => {
  const operation = new ObjectImportSession("target-1", async () =>
    inspection("unexpected"),
  );
  operation.selectSource({
    path: "C:\\registered",
    projectId: "registered-id",
  });
  await operation.inspect();
  assert.equal(operation.getSnapshot().sourceProjectId, "registered-id");
  assert.ok(operation.getSnapshot().error);
  assert.equal(operation.canPrepare, false);
  const empty = new ObjectImportSession("target-1", async () => ({
    project: { id: "empty-source" },
    objects: [],
    importVersions: [],
  }));
  empty.setSource("path", "C:\\empty");
  await empty.inspect();
  assert.equal(empty.getSnapshot().sourceProjectId, "empty-source");
  assert.equal(empty.getSnapshot().phase, "ready");
  assert.equal(empty.canPrepare, false);
});

test("canceling any picker preserves the current selection and preparation receipt", async () => {
  const operation = new ObjectImportSession(
    "target-1",
    async (method, input) => {
      if (method === "chooseImportFiles") return [];
      if (method === "chooseImportDirectory") return null;
      if (method === "object.inspectExternal") return inspection();
      return receipt(input as ImportPreparationRequest);
    },
    () => "request-1",
  );
  operation.setSource("path", "C:\\source");
  await operation.inspect();
  await operation.prepare();
  const before = operation.getSnapshot();
  assert.ok(before.receipt);
  for (const kind of ["project", "files", "directory"] as const) {
    await operation.choosePaths(kind);
    assert.equal(operation.getSnapshot(), before);
  }
});

test("late picker successes and errors cannot undo edits, mode changes or dismissal", async () => {
  const edits = [
    (operation: ObjectImportSession) => operation.setSource("path", "C:\\new"),
    (operation: ObjectImportSession) => operation.setSourceMode("files"),
    (operation: ObjectImportSession) => operation.cancel(),
  ];
  for (const edit of edits) {
    for (const fail of [false, true]) {
      const pending = deferred();
      const calls: string[] = [];
      const operation = new ObjectImportSession("target-1", async (method) => {
        calls.push(method);
        return pending.promise;
      });
      const choosing = operation.choosePaths("project");
      edit(operation);
      const current = operation.getSnapshot();
      if (fail) pending.reject(new Error("stale picker failure"));
      else pending.resolve("C:\\old");
      await choosing;
      assert.equal(operation.getSnapshot(), current);
      assert.deepEqual(calls, ["chooseImportDirectory"]);
    }
  }
});

test("the newest picker wins even when it is canceled", async () => {
  for (const result of ["C:\\new", null]) {
    const first = deferred();
    const second = deferred();
    let pickerCalls = 0;
    let inspections = 0;
    const operation = new ObjectImportSession("target-1", async (method) => {
      if (method === "chooseImportDirectory")
        return ++pickerCalls === 1 ? first.promise : second.promise;
      inspections++;
      return inspection();
    });
    const old = operation.choosePaths("project");
    const current = operation.choosePaths("project");
    second.resolve(result);
    await current;
    const state = operation.getSnapshot();
    first.resolve("C:\\old");
    await old;
    assert.equal(operation.getSnapshot(), state);
    assert.equal(state.path, result ?? "");
    assert.equal(inspections, result ? 1 : 0);
  }
});

test("malformed picker results and current picker failures are visible without changing selection", async () => {
  for (const kind of ["project", "directory", "files"] as const) {
    for (const result of [undefined, 1, ["C:\\valid", 1], "", [""]]) {
      const operation = new ObjectImportSession("target-1", async () => result);
      operation.setSource("path", "C:\\current");
      await operation.choosePaths(kind);
      assert.match(operation.getSnapshot().error, /文件选择器返回的路径无效/);
      assert.equal(operation.getSnapshot().path, "C:\\current");
    }
  }
  const operation = new ObjectImportSession("target-1", async () => {
    throw new Error("native picker unavailable");
  });
  await operation.choosePaths("project");
  assert.match(operation.getSnapshot().error, /native picker unavailable/);
});

test("ordinary file and folder pickers accumulate paths and ignore delayed selection after edits", async () => {
  const delayed = deferred();
  let filePickerCalls = 0;
  const operation = new ObjectImportSession(
    "target-1",
    async (method, input) => {
      if (method === "chooseImportFiles")
        return ++filePickerCalls === 1 ? ["C:\\first.txt"] : delayed.promise;
      if (method === "chooseImportDirectory") return "C:\\assets";
      assert.equal(method, "object.inspectFiles");
      return {
        source: { kind: "files", paths: (input as { paths: string[] }).paths },
        files: [],
        digest: "a".repeat(64),
      };
    },
  );
  await operation.choosePaths("files");
  await operation.choosePaths("directory");
  assert.deepEqual(operation.getSnapshot().selectedPaths, [
    "C:/first.txt",
    "C:/assets",
  ]);
  assert.deepEqual(operation.getSnapshot().fileSnapshot?.source.paths, [
    "C:/first.txt",
    "C:/assets",
  ]);
  const pending = operation.choosePaths("files");
  operation.removeFilePath("C:/first.txt");
  const state = operation.getSnapshot();
  delayed.resolve(["C:\\stale.txt"]);
  await pending;
  assert.equal(operation.getSnapshot(), state);
});

test("pickers do not launch while source inspection is pending", async () => {
  const pending = deferred();
  const calls: string[] = [];
  const operation = new ObjectImportSession("target-1", async (method) => {
    calls.push(method);
    return pending.promise;
  });
  operation.setSource("path", "C:\\source");
  const inspecting = operation.inspect();
  await operation.choosePaths("project");
  await operation.choosePaths("files");
  assert.deepEqual(calls, ["object.inspectExternal"]);
  pending.resolve(inspection());
  await inspecting;
});
