import assert from "node:assert/strict";
import test from "node:test";
import { ObjectImportHistory } from "../src/ui/object-preview/object-import-history";
import { FileImportSession } from "../src/ui/object-preview/object-file-import";
import { deferred } from "./fixtures/object-import";
import {
  historyEntry,
  projectReceipt,
  filesReceipt,
  page,
} from "./fixtures/object-import-history";

test("new preparation outside the first history page opens for explicit commit and reopens", async () => {
  for (const kind of ["project", "files"] as const) {
    const prepared = historyEntry(kind, "new");
    const older = historyEntry("files", "first");
    const methods: string[] = [];
    const receipt =
      kind === "files" ? filesReceipt(prepared) : projectReceipt(prepared);
    const query =
      kind === "files"
        ? "object.fileImportOperation"
        : "object.importOperation";
    const commit =
      kind === "files" ? "object.commitFileImport" : "object.commitImport";
    let imported = false;
    const api = async (method: string, input: unknown) => {
      methods.push(method);
      if (method === "object.importPreparations")
        return page([older], older.cursor);
      if (
        method === "object.getImportPreparation" ||
        method === "object.getFileImportPreparation"
      )
        return receipt;
      assert.deepEqual(input, {
        projectId: "target",
        preparationId: prepared.preparationId,
      });
      assert.ok(method === query || method === commit);
      if (method === commit) imported = true;
      return imported
        ? {
            projectId: "target",
            preparationId: prepared.preparationId,
            state: "importedPendingValidation",
            objectIds: ["imported-object"],
            writes: [],
            error: null,
          }
        : null;
    };
    const history = new ObjectImportHistory("target", api);
    await history.refresh(false, prepared);
    assert.equal(
      history.getSnapshot().receipt?.preparationId,
      prepared.preparationId,
    );
    assert.equal(history.getSnapshot().next, older.cursor);
    assert.equal(history.getSnapshot().entries.length, 1);
    assert.equal(imported, false);
    const session = new FileImportSession(
      "target",
      prepared.preparationId,
      api,
      kind,
    );
    await session.refresh();
    assert.equal(imported, false);
    await session.commit();
    assert.equal(
      session.getSnapshot().operation?.state,
      "importedPendingValidation",
    );
    const reopened = new FileImportSession(
      "target",
      prepared.preparationId,
      api,
      kind,
    );
    await reopened.refresh();
    assert.deepEqual(reopened.getSnapshot().operation?.objectIds, [
      "imported-object",
    ]);
    assert.equal(methods.filter((m) => m === commit).length, 1);
  }
});

test("superseded or cancelled preparation reads cannot reopen the commit panel", async () => {
  const old = historyEntry("project", "old"),
    latest = historyEntry("project", "new");
  const pending = deferred();
  const history = new ObjectImportHistory("target", async (method, input) => {
    if (method === "object.importPreparations") return page([]);
    return (input as { preparationId: string }).preparationId ===
      old.preparationId
      ? pending.promise
      : projectReceipt(latest);
  });
  const previous = history.refresh(false, old);
  await new Promise((resolve) => setImmediate(resolve));
  await history.refresh(false, latest);
  pending.resolve(projectReceipt(old));
  await previous;
  assert.equal(
    history.getSnapshot().receipt?.preparationId,
    latest.preparationId,
  );
  const delayed = deferred();
  const closed = new ObjectImportHistory("target", async () => delayed.promise);
  const opening = closed.refresh(false, old);
  closed.cancel();
  delayed.resolve(page([]));
  await opening;
  assert.equal(closed.getSnapshot().receipt, null);
});
