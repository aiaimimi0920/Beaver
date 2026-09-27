import assert from "node:assert/strict";
import test from "node:test";
import {
  catalogSceneTarget,
  readCatalogThumbnail,
} from "../src/ui/object-preview/object-thumbnail-cache";
import { objectCatalogRecordSchema } from "../src/shared/object-catalog";
const target = {
  projectId: "p",
  objectId: "o",
  versionId: "v",
  path: "a.tscn",
  sha256: "a".repeat(64),
};
function result() {
  return {
    target,
    resolution: { width: 1280, height: 720 },
    sourceDigest: "digest",
    projectConfig: "frozen",
    integrityError: null,
    run: {
      id: "r",
      projectId: "p",
      kind: "objectPreview",
      status: "completed",
      phase: "complete",
      error: null,
      engineVersion: "4",
      runnerVersion: "1",
      snapshotId: "s",
      createdAt: "",
      finishedAt: "",
      evidence: [{ id: "e", kind: "image", sha256: "b".repeat(64) }],
    },
  };
}
test("catalog cache uses the latest frozen manifest, never live files or older versions", () => {
  const manifest = {
    schemaVersion: 1,
    projectId: "p",
    objectId: "o",
    versionId: "v",
    status: "captured",
    name: "Scene",
    components: [],
    files: [
      { path: "a.tscn", role: "source", bytes: 10, sha256: target.sha256 },
    ],
    references: [],
  };
  const object = objectCatalogRecordSchema.parse({
    id: "o",
    projectId: "p",
    name: "Scene",
    components: [],
    files: [{ path: "live.tscn", role: "source" }],
    references: [],
    versions: [{ versionId: "v", manifest }],
  });
  assert.deepEqual(catalogSceneTarget(object), target);
  object.versions.push({
    versionId: "new",
    manifest: { ...manifest, versionId: "new", files: [] },
  });
  assert.equal(catalogSceneTarget(object), null);
});
test("cache read verifies target and integrity without starting a render", async () => {
  const signal = new AbortController().signal;
  const methods: string[] = [];
  const cached = await readCatalogThumbnail(
    target,
    async (method) => {
      methods.push(method);
      return result();
    },
    signal,
  );
  assert.equal(cached?.image.id, "e");
  assert.deepEqual(methods, ["object.scenePreview.get"]);
  await assert.rejects(
    readCatalogThumbnail(
      target,
      async () => ({
        ...result(),
        target: { ...target, versionId: "foreign" },
      }),
      signal,
    ),
    /其他版本/,
  );
  await assert.rejects(
    readCatalogThumbnail(
      target,
      async () => ({ ...result(), integrityError: "changed evidence" }),
      signal,
    ),
    /changed evidence/,
  );
  assert.equal(
    await readCatalogThumbnail(
      target,
      async () => ({ ...result(), run: { ...result().run, status: "queued" } }),
      signal,
    ),
    null,
  );
});
test("catalog reads are bounded and disposed queued cards never call the API", async () => {
  const releases: (() => void)[] = [];
  let calls = 0;
  let inflight = 0;
  let maximum = 0;
  const call = async () => {
    calls++;
    inflight++;
    maximum = Math.max(maximum, inflight);
    await new Promise<void>((resolve) => releases.push(resolve));
    inflight--;
    return result();
  };
  const controllers = Array.from({ length: 7 }, () => new AbortController());
  const reads = controllers.map((controller) =>
    readCatalogThumbnail(target, call, controller.signal),
  );
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(calls, 3);
  controllers[0]!.abort();
  controllers[3]!.abort();
  while (releases.length || inflight) {
    releases.splice(0).forEach((release) => release());
    await new Promise((resolve) => setImmediate(resolve));
  }
  const values = await Promise.all(reads);
  assert.equal(maximum, 3);
  assert.equal(calls, 6);
  assert.equal(values[0], null);
  assert.equal(values[3], null);
});
