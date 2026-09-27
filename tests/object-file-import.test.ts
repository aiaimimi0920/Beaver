import assert from "node:assert/strict";
import test from "node:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { FileImportSession } from "../src/ui/object-preview/object-file-import";
import {
  parseFileImportOperation,
  type FileImportOperation,
} from "../src/shared/object-file-import";
import {
  acceptedObjectReferences,
  readObjectVersion,
  type ObjectCatalogRecord,
} from "../src/shared/object-catalog";
import { ObjectCatalogDetails } from "../src/ui/object-preview/ObjectCatalogDetails";
import { ObjectFileImportPanel } from "../src/ui/object-preview/ObjectFileImportPanel";
import { deferred } from "./fixtures/object-import";

function operation(
  state: FileImportOperation["state"] = "applying",
): FileImportOperation {
  return {
    projectId: "target",
    preparationId: "saved",
    state,
    objectIds: ["object"],
    writes: [],
    error: null,
  };
}

test("opening only queries; lost commit response and new session retain stable preparation identity", async () => {
  const calls: string[] = [];
  let saved: FileImportOperation | null = null;
  const api = async (method: string, input: unknown) => {
    calls.push(method);
    assert.deepEqual(input, { projectId: "target", preparationId: "saved" });
    if (method === "object.fileImportOperation") return saved;
    saved = operation("importedPendingValidation");
    if (calls.length === 2) throw new Error("response lost");
    return saved;
  };
  const first = new FileImportSession("target", "saved", api);
  await first.commit();
  assert.deepEqual(calls, []);
  await first.refresh();
  await first.commit();
  assert.match(first.getSnapshot().error, /response lost/);
  await first.commit();
  assert.equal(
    first.getSnapshot().operation?.state,
    "importedPendingValidation",
  );
  await first.commit();
  await first.abort();
  assert.equal(calls.length, 3);
  first.cancel();
  const reopened = new FileImportSession("target", "saved", api);
  await reopened.refresh();
  assert.equal(
    reopened.getSnapshot().operation?.state,
    "importedPendingValidation",
  );
  assert.equal(calls.at(-1), "object.fileImportOperation");
});

test("project imports reopen, retry and abort using the bound project receipt", async () => {
  const calls: string[] = [];
  let saved: FileImportOperation | null = null;
  const api = async (method: string, input: unknown) => {
    calls.push(method);
    assert.deepEqual(input, { projectId: "target", preparationId: "saved" });
    if (method === "object.commitImport") saved = operation();
    if (method === "object.abortImport") saved = operation("aborted");
    return saved;
  };
  const session = new FileImportSession("target", "saved", api, "project");
  await session.refresh();
  await session.commit();
  session.cancel();
  const reopened = new FileImportSession("target", "saved", api, "project");
  await reopened.refresh();
  assert.equal(reopened.getSnapshot().operation?.state, "applying");
  await reopened.commit();
  await reopened.abort();
  assert.equal(reopened.getSnapshot().operation?.state, "aborted");
  assert.deepEqual(calls, [
    "object.importOperation",
    "object.commitImport",
    "object.importOperation",
    "object.commitImport",
    "object.abortImport",
  ]);
  const panel = renderToStaticMarkup(
    createElement(ObjectFileImportPanel, {
      projectId: "target",
      preparationId: "saved",
      sourceKind: "project",
    }),
  );
  assert.match(panel, /保留项目内原相对路径/);
});

test("abort conflicts remain retryable and never permit commit while aborting", async () => {
  let saved = operation();
  const calls: string[] = [];
  const session = new FileImportSession("target", "saved", async (method) => {
    calls.push(method);
    if (method === "object.abortFileImport")
      saved = { ...operation("aborting"), error: "external edits" };
    return saved;
  });
  await session.refresh();
  await session.abort();
  await session.commit();
  assert.equal(calls.length, 2);
  assert.equal(session.getSnapshot().operation?.error, "external edits");
  await session.abort();
  assert.equal(calls.at(-1), "object.abortFileImport");
});

test("closed session ignores late commit results and rejects foreign query responses", async () => {
  const pending = deferred();
  let calls = 0;
  const session = new FileImportSession("target", "saved", async () =>
    ++calls === 1 ? null : pending.promise,
  );
  await session.refresh();
  const committing = session.commit();
  await session.commit();
  assert.equal(calls, 2);
  session.cancel();
  pending.resolve(operation("importedPendingValidation"));
  await committing;
  assert.equal(session.getSnapshot().operation, null);
  assert.equal(session.getSnapshot().busy, false);
  const foreign = new FileImportSession("target", "saved", async () => ({
    ...operation(),
    projectId: "other",
  }));
  await foreign.refresh();
  assert.equal(foreign.getSnapshot().loaded, false);
  assert.match(foreign.getSnapshot().error, /身份不匹配/);
  assert.throws(() =>
    parseFileImportOperation(
      { ...operation(), objectIds: ["object", "object"] },
      "target",
      "saved",
    ),
  );
});

test("pending imported versions display honestly and never enter accepted references", () => {
  const object: ObjectCatalogRecord = {
    id: "object",
    projectId: "target",
    name: "Imported",
    category: "其他",
    tags: [],
    thumbnailPath: null,
    parentObjectId: null,
    revision: 1,
    components: [],
    files: [],
    references: [],
    versions: [],
  };
  object.versions.push({
    versionId: "import-version",
    manifest: {
      schemaVersion: 1,
      projectId: object.projectId,
      objectId: object.id,
      versionId: "import-version",
      status: "importedPendingValidation",
      name: object.name,
      category: object.category,
      tags: [],
      thumbnailPath: null,
      parentObjectId: null,
      components: [],
      files: [],
      references: [],
    },
  });
  assert.equal(
    readObjectVersion(object, object.versions[0]!)?.status,
    "importedPendingValidation",
  );
  assert.deepEqual(acceptedObjectReferences([object], "target"), []);
  const html = renderToStaticMarkup(
    createElement(ObjectCatalogDetails, {
      object,
      objects: [object],
      currentProjectId: "target",
      select() {},
    }),
  );
  assert.match(html, /导入待验证，尚未接受/);
  const panel = renderToStaticMarkup(
    createElement(ObjectFileImportPanel, {
      projectId: "target",
      preparationId: "saved",
    }),
  );
  assert.match(panel, /确认正式导入/);
  assert.match(panel, /不改写文件内容或跨根目录引用/);
});
