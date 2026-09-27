import assert from "node:assert/strict";
import test from "node:test";
import type { FileImportPreparationRequest } from "../src/shared/object-import";
import { ObjectImportSession } from "../src/ui/object-preview/object-import-session";
import { deferred } from "./fixtures/object-import";

function fileReceipt(request: FileImportPreparationRequest) {
  return {
    schemaVersion: 1,
    preparationId: `file-import-${request.requestId}`,
    requestId: request.requestId,
    requestDigest: "a".repeat(64),
    targetProjectId: request.targetProjectId,
    source: request.snapshot,
    groups: request.groups,
    identityMap: { files: {}, groups: {} },
    readyToCommit: false,
  };
}

function snapshot(paths: string[], includeNested = true) {
  const files = paths.flatMap((sourcePath, index) => {
    const directPath = `${sourcePath}/file-${index}.txt`;
    const nested = includeNested
      ? [
          {
            sourcePath,
            relativePath: "nested/asset.png",
            path: `${sourcePath}/nested/asset.png`,
            kind: "image",
            bytes: 4,
            sha256: "b".repeat(64),
          },
        ]
      : [];
    return [
      {
        sourcePath,
        relativePath: `file-${index}.txt`,
        path: directPath,
        kind: "document",
        bytes: 3,
        sha256: "a".repeat(64),
      },
      ...nested,
    ];
  });
  return {
    source: { kind: "files" as const, paths },
    files,
    digest: "c".repeat(64),
  };
}

test("file source paths accumulate, normalize, deduplicate and remove", async () => {
  const operation = new ObjectImportSession(
    "target-1",
    async (_method, input) => snapshot((input as { paths: string[] }).paths),
  );
  operation.addFilePaths([" C:\\assets\\ ", "C:/assets", "D:\\other\\"]);
  assert.deepEqual(operation.getSnapshot().selectedPaths, [
    "C:/assets",
    "D:/other",
  ]);
  await operation.inspectFiles();
  assert.equal(operation.getSnapshot().fileSnapshot?.files.length, 4);
  operation.removeFilePath(" C:\\assets\\");
  assert.deepEqual(operation.getSnapshot().selectedPaths, ["D:/other"]);
  assert.equal(operation.getSnapshot().fileSnapshot, null);
});

test("file groups use snapshot paths, support splitting and survive added roots", async () => {
  const operation = new ObjectImportSession(
    "target-1",
    async (_method, input) => snapshot((input as { paths: string[] }).paths),
  );
  operation.addFilePaths(["C:/assets"]);
  await operation.inspectFiles();
  operation.addFileGroup(" Hero ");
  operation.addFileGroup(" Props ");
  operation.assignFileToGroup("C:/assets/nested/asset.png", "group-1");
  assert.deepEqual(operation.getSnapshot().fileGroups[0]?.paths, [
    "C:/assets/nested/asset.png",
  ]);
  operation.assignFileToGroup("C:/assets/nested/asset.png", "");
  assert.deepEqual(operation.getSnapshot().fileGroups[0]?.paths, []);
  operation.assignFileToGroup("C:/assets/nested/asset.png", "group-1");
  operation.addFilePaths(["C:/new-assets"]);
  assert.deepEqual(operation.getSnapshot().fileGroups[0]?.paths, [
    "C:/assets/nested/asset.png",
  ]);
  await operation.inspectFiles();
  operation.removeFilePath("C:/assets");
  assert.deepEqual(operation.getSnapshot().fileGroups[0]?.paths, []);
  operation.removeFileGroup("group-2");
  assert.equal(operation.getSnapshot().fileGroups.length, 1);
});

test("switching source modes clears the other source and allows empty project selection", () => {
  const operation = new ObjectImportSession("target-1", async () => ({}));
  operation.addFilePaths(["C:/assets"]);
  operation.addFileGroup("Assets");
  operation.selectSource({ path: "", projectId: "" });
  assert.equal(operation.getSnapshot().sourceMode, "project");
  assert.deepEqual(operation.getSnapshot().selectedPaths, []);
  assert.deepEqual(operation.getSnapshot().fileGroups, []);
  operation.setSource("path", "C:/project");
  operation.setSource("sourceProjectId", "source-1");
  operation.setSourceMode("files");
  assert.equal(operation.getSnapshot().path, "");
  assert.equal(operation.getSnapshot().sourceProjectId, "");
  assert.equal(operation.getSnapshot().sourceMode, "files");
});

test("late file inspection responses cannot replace a newer path collection", async () => {
  let resolveOld!: (value: unknown) => void;
  let resolveCurrent!: (value: unknown) => void;
  const old = new Promise((resolve) => {
    resolveOld = resolve;
  });
  const current = new Promise((resolve) => {
    resolveCurrent = resolve;
  });
  let calls = 0;
  const operation = new ObjectImportSession("target-1", async () =>
    ++calls === 1 ? old : current,
  );
  operation.addFilePaths(["C:/old"]);
  const first = operation.inspectFiles();
  operation.addFilePaths(["C:/new"]);
  const second = operation.inspectFiles();
  resolveCurrent(snapshot(["C:/old", "C:/new"]));
  await second;
  resolveOld(snapshot(["C:/old"]));
  await first;
  assert.deepEqual(operation.getSnapshot().selectedPaths, ["C:/old", "C:/new"]);
  assert.deepEqual(operation.getSnapshot().fileSnapshot?.source.paths, [
    "C:/old",
    "C:/new",
  ]);
});

test("file sources prepare a receipt and cancel clears only the snapshot", async () => {
  const calls: string[] = [];
  const operation = new ObjectImportSession(
    "target-1",
    async (method, input) => {
      calls.push(method);
      if (method === "object.inspectFiles")
        return snapshot((input as { paths: string[] }).paths);
      return fileReceipt(input as FileImportPreparationRequest);
    },
  );
  operation.addFilePaths(["C:/assets"]);
  await operation.inspectFiles();
  operation.addFileGroup("Assets");
  const snapshotBeforeCancel = operation.getSnapshot().fileSnapshot;
  assert.ok(snapshotBeforeCancel);
  assert.ok(await operation.prepare());
  assert.deepEqual(calls, ["object.inspectFiles", "object.prepareFileImport"]);
  operation.cancel();
  assert.equal(operation.getSnapshot().fileSnapshot, null);
  assert.deepEqual(operation.getSnapshot().selectedPaths, ["C:/assets"]);
  assert.equal(operation.getSnapshot().fileGroups[0]?.name, "Assets");
});

const groupEdits: Record<string, (session: ObjectImportSession) => void> = {
  add: (session) => session.addFileGroup("Props"),
  remove: (session) => session.removeFileGroup("group-1"),
  assign: (session) =>
    session.assignFileToGroup("C:/assets/file-0.txt", "group-1"),
};

for (const [name, edit] of Object.entries(groupEdits)) {
  test(`group ${name} invalidates prepared and in-flight receipts without blocking retry`, async () => {
    for (const phase of ["preparing", "prepared"] as const) {
      const pending = deferred();
      const requests: FileImportPreparationRequest[] = [];
      let sequence = 0;
      const operation = new ObjectImportSession(
        "target-1",
        async (method, input) => {
          if (method === "object.inspectFiles") return snapshot(["C:/assets"]);
          const request = input as FileImportPreparationRequest;
          requests.push(request);
          return requests.length === 1 ? pending.promise : fileReceipt(request);
        },
        () => `request-${++sequence}`,
      );
      operation.addFilePaths(["C:/assets"]);
      await operation.inspectFiles();
      operation.addFileGroup("Assets");
      const preparing = operation.prepare();
      if (phase === "prepared") {
        pending.resolve(fileReceipt(requests[0]!));
        assert.ok(await preparing);
      }
      assert.equal(operation.getSnapshot().phase, phase);
      edit(operation);
      assert.equal(operation.getSnapshot().phase, "ready");
      assert.equal(operation.getSnapshot().receipt, null);
      assert.equal(operation.canPrepare, true);
      if (phase === "preparing") {
        pending.resolve(fileReceipt(requests[0]!));
        assert.equal(await preparing, undefined);
        assert.equal(operation.getSnapshot().receipt, null);
      }
      assert.ok(await operation.prepare());
      assert.notEqual(requests[0]?.requestId, requests[1]?.requestId);
      assert.deepEqual(requests[1]?.groups, operation.getSnapshot().fileGroups);
      assert.equal(operation.getSnapshot().phase, "prepared");
    }
  });
}

test("editing groups during file inspection permits a fresh read", async () => {
  const pending = deferred();
  let calls = 0;
  const operation = new ObjectImportSession("target-1", async () =>
    ++calls === 1 ? pending.promise : snapshot(["C:/assets"]),
  );
  operation.addFilePaths(["C:/assets"]);
  const inspecting = operation.inspectFiles();
  operation.addFileGroup("Assets");
  assert.equal(operation.getSnapshot().phase, "editing");
  assert.equal(operation.busy, false);
  pending.resolve(snapshot(["C:/assets"]));
  await inspecting;
  assert.equal(operation.getSnapshot().fileSnapshot, null);
  await operation.inspectFiles();
  assert.equal(operation.canPrepare, true);
  assert.equal(operation.getSnapshot().fileGroups[0]?.name, "Assets");
});
