import assert from "node:assert/strict";
import test from "node:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import {
  parseImportHistory,
  parseImportHistoryReceipt,
} from "../src/shared/object-import-history";
import { ObjectImportHistory } from "../src/ui/object-preview/object-import-history";
import { ImportPreparationDetails } from "../src/ui/object-preview/ObjectImportHistoryPanel";
import { ObjectImportDialog } from "../src/ui/object-preview/ObjectImportDialog";
import { deferred } from "./fixtures/object-import";
import {
  historyEntry,
  projectReceipt,
  filesReceipt,
  page,
} from "./fixtures/object-import-history";

test("new dialog session discovers both persisted kinds and only reads saved target receipts", async () => {
  const entries = [historyEntry("files"), historyEntry("project")];
  const methods: string[] = [];
  const api = async (method: string, input: unknown) => {
    methods.push(method);
    if (method === "object.importPreparations") {
      assert.deepEqual(input, { projectId: "target" });
      return page(entries);
    }
    assert.deepEqual(input, {
      [method === "object.getImportPreparation"
        ? "projectId"
        : "targetProjectId"]: "target",
      preparationId: "receipt-one",
    });
    if (method === "object.getImportPreparation") return projectReceipt();
    if (method === "object.getFileImportPreparation") return filesReceipt();
    throw new Error("source is offline: no inspection or writes allowed");
  };
  for (const entry of entries) {
    const history = new ObjectImportHistory("target", api);
    await history.refresh();
    await history.open(
      history.getSnapshot().entries.find((e) => e.kind === entry.kind)!,
    );
    assert.equal(history.getSnapshot().receipt?.kind, entry.kind);
    history.cancel();
  }
  assert.deepEqual(methods, [
    "object.importPreparations",
    "object.getFileImportPreparation",
    "object.importPreparations",
    "object.getImportPreparation",
  ]);
});

test("pagination retries preserve entries and reject foreign or repeated pages", async () => {
  const first = historyEntry("files", "one"),
    second = historyEntry("project", "two");
  let calls = 0;
  const history = new ObjectImportHistory("target", async (_method, input) => {
    calls++;
    if (calls === 1) return page([first], first.cursor);
    assert.deepEqual(input, { projectId: "target", after: first.cursor });
    if (calls === 2) throw new Error("connection lost");
    return page([second]);
  });
  await history.refresh();
  await history.refresh(true);
  assert.equal(history.getSnapshot().entries.length, 1);
  assert.match(history.getSnapshot().error, /connection lost/);
  await history.refresh(true);
  assert.equal(history.getSnapshot().entries.length, 2);
  assert.equal(history.getSnapshot().next, null);
  assert.throws(() => parseImportHistory(page([first]), "other"));
  assert.throws(() =>
    parseImportHistory(page([first]), "target", first.cursor),
  );
  assert.throws(() =>
    parseImportHistory(page([first], second.cursor), "target"),
  );
});

test("late detail cannot replace a newer selection or a closed session", async () => {
  const pending = deferred();
  const entries = [historyEntry("files"), historyEntry("project")];
  const history = new ObjectImportHistory("target", async (method) =>
    method === "object.importPreparations"
      ? page(entries)
      : method === "object.getFileImportPreparation"
        ? pending.promise
        : projectReceipt(),
  );
  await history.refresh();
  const [file, project] = history.getSnapshot().entries;
  const old = history.open(file!);
  await history.open(project!);
  pending.resolve(filesReceipt());
  await old;
  assert.equal(history.getSnapshot().receipt?.kind, "project");
  const late = deferred();
  const closed = new ObjectImportHistory("target", async () => late.promise);
  const loading = closed.refresh();
  closed.cancel();
  late.resolve(page(entries));
  await loading;
  assert.equal(closed.getSnapshot().loaded, false);
  assert.equal(closed.getSnapshot().entries.length, 0);
});

test("invalid or missing receipts stay errors; refresh remains recoverable", async () => {
  const entry = historyEntry("project");
  let response: unknown = null;
  const history = new ObjectImportHistory("target", async (method) =>
    method === "object.importPreparations" ? page([entry]) : response,
  );
  await history.refresh();
  const selected = history.getSnapshot().entries[0]!;
  await history.open(selected);
  assert.match(history.getSnapshot().error, /已不存在/);
  response = { ...projectReceipt(), targetProjectId: "other" };
  await history.open(selected);
  assert.equal(history.getSnapshot().receipt, null);
  assert.match(history.getSnapshot().error, /不一致/);
  response = projectReceipt();
  await history.open(selected);
  assert.equal(history.getSnapshot().error, "");
});

test("frozen details show file hashes, identity mappings and pending status in production dialog", () => {
  const project = parseImportHistoryReceipt(
    projectReceipt(),
    "target",
    historyEntry("project"),
  );
  const files = parseImportHistoryReceipt(
    filesReceipt(),
    "target",
    historyEntry("files"),
  );
  for (const [receipt, labels] of [
    [project, ["Frozen hero", "hero.tscn", "target-v1", "target-mesh"]],
    [files, ["Saved group", "image.png", "target-group", "target-file"]],
  ] as const) {
    const html = renderToStaticMarkup(
      createElement(ImportPreparationDetails, { receipt }),
    );
    for (const label of labels) assert.ok(html.includes(label));
    assert.ok(html.includes("准备记录本身不表示正式导入已完成"));
    assert.ok(html.includes("未重新检查源文件"));
  }
  const html = renderToStaticMarkup(
    createElement(ObjectImportDialog, {
      projectId: "target",
      close() {},
      notify() {},
    }),
  );
  assert.ok(html.includes('aria-label="导入准备历史"'));
});

test("rejects cross-source closure and missing file ownership mappings", () => {
  const project = projectReceipt();
  project.versions[0]!.references = [
    { projectId: "other", objectId: "hero", versionId: "v1" },
  ];
  assert.throws(() =>
    parseImportHistoryReceipt(project, "target", historyEntry("project")),
  );
  const files = filesReceipt();
  files.identityMap.files = {} as typeof files.identityMap.files;
  assert.throws(() =>
    parseImportHistoryReceipt(files, "target", historyEntry("files")),
  );
});
