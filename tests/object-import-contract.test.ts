import assert from "node:assert/strict";
import test from "node:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import {
  parseImportInspection,
  parseImportReceipt,
  type ImportPreparationRequest,
} from "../src/shared/object-import";
import { ObjectImportDialog } from "../src/ui/object-preview/ObjectImportDialog";
import { inspection, receipt } from "./fixtures/object-import";

test("inspection previews frozen files and keeps unavailable versions blocked", () => {
  const { objects } = parseImportInspection(inspection(), "source-1");
  const first = objects[0]?.versions[0];
  assert.equal(first?.manifest?.name, "Frozen hero 1");
  assert.deepEqual(
    first?.manifest?.files.map((file) => file.path),
    ["hero-1.tscn"],
  );
  assert.equal(objects[0]?.versions[2]?.manifest, null);
  assert.equal(objects[0]?.versions[2]?.blocker, "IMPORT_VERSION_NOT_ACCEPTED");
});

test("inspection discovers identity even when the source catalog is empty", () => {
  const input = inspection("external-source");
  const discovered = parseImportInspection(input);
  assert.equal(discovered.projectId, "external-source");
  assert.equal(discovered.objects[0]?.id, "hero");
  input.objects = [];
  input.importVersions = [];
  assert.deepEqual(parseImportInspection(input), {
    projectId: "external-source",
    objects: [],
  });
  assert.throws(() => parseImportInspection(input, "registered-source"));
});

test("inspection rejects ambiguous, missing and cross-project version options", () => {
  type Snapshot = ReturnType<typeof inspection>;
  const mutations: Array<(input: Snapshot) => void> = [
    (input) => {
      input.project.id = "other";
    },
    (input) => {
      input.objects[0]!.projectId = "other";
    },
    (input) => {
      input.objects.push(input.objects[0]!);
    },
    (input) => {
      input.objects[0]!.versions.push(input.objects[0]!.versions[0]!);
    },
    (input) => {
      input.importVersions.pop();
    },
    (input) => {
      input.importVersions.push(input.importVersions[0]!);
    },
    (input) => {
      input.importVersions[0]!.versionId = "unknown";
    },
    (input) => {
      input.importVersions[0]!.sourceDigest = null;
    },
    (input) => {
      input.importVersions[0]!.blocker = "conflicting blocker";
    },
    (input) => {
      input.importVersions[0]!.sourceDigest = "invalid digest";
    },
    (input) => {
      input.objects[0]!.versions[0]!.manifest.objectId = "other";
    },
    (input) => {
      input.objects[0]!.versions[0]!.manifest.versionId = "other";
    },
    (input) => {
      input.objects[0]!.versions[0]!.manifest.status = "candidate";
    },
  ];
  for (const mutate of mutations) {
    const input = inspection();
    mutate(input);
    for (const expectedProjectId of ["source-1", undefined])
      assert.throws(() => parseImportInspection(input, expectedProjectId));
  }
});

test("receipt must match request, original target and the exact inspected selection", () => {
  const request: ImportPreparationRequest = {
    requestId: "request-1",
    targetProjectId: "target-1",
    source: { path: "C:\\source", projectId: "source-1" },
    objectId: "hero",
    baseline: { kind: "pinnedVersion", versionId: "hero-v2" },
    sourceDigest: "b".repeat(64),
  };
  const input = receipt(request);
  assert.equal(parseImportReceipt(input, request).readyToCommit, false);
  for (const patch of [
    { requestId: "other" },
    { targetProjectId: "other" },
    { sourceProjectId: "other" },
    { sourceObjectId: "other" },
    { acceptedVersionId: "other" },
    { sourceDigest: "c".repeat(64) },
    { baseline: { kind: "pinnedVersion", versionId: "other" } },
    { baseline: { kind: "latestAccepted" } },
    { readyToCommit: true },
    { schemaVersion: 2 },
  ])
    assert.throws(() => parseImportReceipt({ ...input, ...patch }, request));
  assert.throws(() => parseImportReceipt({ preparationId: "legacy" }, request));
});

test("import dialog labels preparation honestly and requires a bound target", () => {
  const markup = renderToStaticMarkup(
    createElement(ObjectImportDialog, {
      close: () => {},
      notify: () => {},
    }),
  );
  assert.match(markup, /尚未选择项目/);
  assert.match(markup, /目标对象目录尚不改变/);
  assert.match(markup, /再确认正式导入/);
  assert.match(markup, /type="submit"[^>]*disabled=""/);
  assert.doesNotMatch(markup, /type="file"|标签|补充说明/);
});

test("import offers three source entrances and excludes the bound target", () => {
  const markup = renderToStaticMarkup(
    createElement(ObjectImportDialog, {
      close: () => {},
      notify: () => {},
      projectId: "target-1",
      projects: [
        { id: "source-1", name: "Registered source", path: "C:\\source" },
        { id: "target-1", name: "Bound target", path: "C:\\target" },
      ],
    }),
  );
  assert.match(markup, /<option value="source-1">Registered source/);
  assert.doesNotMatch(markup, /<option value="target-1"/);
  assert.match(markup, /已登记项目/);
  assert.match(markup, /外部 Beaver 项目/);
  assert.match(markup, /文件和文件夹/);
  assert.match(markup, /选择已登记项目/);
  assert.match(markup, /aria-label="源项目目录"/);
  assert.match(markup, /aria-label="源项目 ID"[^>]*readonly=""/i);
  assert.match(markup, /已打开项目使用已提交快照/);
  assert.doesNotMatch(markup, /源项目需关闭/);
});
