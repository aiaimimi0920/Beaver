import assert from "node:assert/strict";
import test from "node:test";
import {
  loadImportDraft,
  saveImportDraft,
  type DraftStorage,
} from "../src/ui/object-preview/object-import-draft";
import { ObjectImportSession } from "../src/ui/object-preview/object-import-session";
import { inspection } from "./fixtures/object-import";

function storage() {
  const values = new Map<string, string>();
  return {
    values,
    getItem: (key: string) => values.get(key) ?? null,
    setItem: (key: string, value: string) => {
      values.set(key, value);
    },
  };
}

test("closed file draft restores groups and unfinished name without source access or readiness", async () => {
  const disk = storage();
  const first = new ObjectImportSession("target", async () => {
    throw new Error("offline");
  });
  first.selectFiles(["C:/art/a.png", "C:/art/b.png"]);
  first.addFileGroup("图片");
  first.assignFileToGroup("C:/art/a.png", "group-1");
  assert.equal(
    saveImportDraft("target", disk, first.getSnapshot(), "尚未添加"),
    "",
  );
  first.cancel();
  const loaded = loadImportDraft("target", disk);
  const calls: string[] = [];
  const reopened = new ObjectImportSession(
    "target",
    async (method) => {
      calls.push(method);
      throw new Error("offline");
    },
    undefined,
    loaded.draft,
  );
  assert.equal(loaded.draft?.groupName, "尚未添加");
  assert.equal(reopened.getSnapshot().phase, "editing");
  assert.equal(reopened.canPrepare, false);
  assert.equal(reopened.getSnapshot().fileSnapshot, null);
  assert.equal(reopened.getSnapshot().receipt, null);
  await reopened.prepare();
  assert.deepEqual(calls, []);
  reopened.addFileGroup("第二组");
  assert.deepEqual(
    reopened.getSnapshot().fileGroups.map((group) => group.id),
    ["group-1", "group-2"],
  );
  await reopened.inspect();
  assert.match(reopened.getSnapshot().error, /offline/);
  assert.deepEqual(reopened.getSnapshot().fileGroups[0]?.paths, [
    "C:/art/a.png",
  ]);
  assert.equal(loadImportDraft("other", disk).draft, undefined);
});

test("project selection survives restart but requires explicit fresh inspection", async () => {
  const disk = storage();
  for (const mode of ["project", "external"] as const) {
    const first = new ObjectImportSession("target", async () => inspection());
    first.selectSource({ path: "C:/source", projectId: "source-1" }, mode);
    await first.inspect();
    first.selectObject("prop");
    first.selectVersion("prop-v1");
    saveImportDraft("target", disk, first.getSnapshot(), "");
    const calls: string[] = [];
    const reopened = new ObjectImportSession(
      "target",
      async (method) => {
        calls.push(method);
        return inspection();
      },
      undefined,
      loadImportDraft("target", disk).draft,
    );
    assert.equal(reopened.getSnapshot().sourceMode, mode);
    assert.equal(Boolean(reopened.selectedVersion), false);
    assert.equal(reopened.canPrepare, false);
    assert.deepEqual(calls, []);
    await reopened.inspect();
    assert.equal(reopened.getSnapshot().objectId, "prop");
    assert.equal(reopened.selectedVersion?.versionId, "prop-v1");
    assert.equal(reopened.canPrepare, true);
    assert.deepEqual(calls, ["object.inspectExternal"]);
  }
  const payload = [...disk.values.values()][0]!;
  assert.equal(payload.includes("sourceDigest"), false);
  assert.equal(payload.includes("receipt"), false);
});

test("corrupt, foreign and inaccessible drafts do not load or erase saved data", () => {
  const disk = storage();
  const first = new ObjectImportSession("target", async () => null);
  saveImportDraft("target", disk, first.getSnapshot(), "");
  const [key, original] = [...disk.values][0]!;
  for (const bad of [
    "{",
    original.replace('"target"', '"other"'),
    original.replace('"schemaVersion":1', '"schemaVersion":2'),
  ]) {
    disk.values.set(key, bad);
    assert.equal(loadImportDraft("target", disk).draft, undefined);
    assert.match(loadImportDraft("target", disk).error, /无法恢复/);
    assert.equal(disk.values.get(key), bad);
  }
  const blocked: DraftStorage = {
    getItem() {
      throw new Error("denied");
    },
    setItem() {
      throw new Error("quota");
    },
  };
  assert.match(loadImportDraft("target", blocked).error, /无法恢复/);
  assert.match(
    saveImportDraft("target", blocked, first.getSnapshot(), ""),
    /无法保存/,
  );
  assert.equal(
    saveImportDraft(undefined, blocked, first.getSnapshot(), ""),
    "",
  );
});
