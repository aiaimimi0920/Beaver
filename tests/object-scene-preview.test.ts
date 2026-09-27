import assert from "node:assert/strict";
import test from "node:test";
import {
  ObjectScenePreview,
  type SceneTarget,
} from "../src/ui/object-preview/object-scene-preview";

const target: SceneTarget = {
  projectId: "p",
  objectId: "o",
  versionId: "v",
  path: "scene.blend",
  sha256: "hash",
};
test("lost render response retries the same request; close rejects late reads", async () => {
  const requests: unknown[] = [];
  let release: (value: unknown) => void = () => {};
  const session = new ObjectScenePreview(target, async (method, input) => {
    if (method.endsWith(".run")) {
      requests.push(input);
      if (requests.length === 1) throw new Error("lost response");
      return { runId: "r" };
    }
    return new Promise((resolve) => {
      release = resolve;
    });
  });
  session.setResolution("720p");
  await session.render();
  assert.match(session.getSnapshot().error, /lost response/);
  assert.equal(session.getSnapshot().retryPending, true);
  session.setResolution("1080p");
  assert.equal(session.getSnapshot().resolution, "720p");
  const retry = session.render();
  await new Promise((resolve) => setImmediate(resolve));
  assert.deepEqual(requests[0], requests[1]);
  assert.equal((requests[1] as { resolution: string }).resolution, "720p");
  assert.equal(session.getSnapshot().retryPending, false);
  session.close();
  const before = session.getSnapshot();
  release(null);
  await retry;
  assert.equal(session.getSnapshot(), before);
});

test("a pending refresh cannot overwrite a newer mutation error", async () => {
  let release: (value: unknown) => void = () => {};
  const session = new ObjectScenePreview(target, async (method) => {
    if (method.endsWith(".get"))
      return new Promise((resolve) => {
        release = resolve;
      });
    throw new Error("render failed");
  });
  const read = session.refresh();
  await session.render();
  release(null);
  await read;
  assert.match(session.getSnapshot().error, /render failed/);
});

test("successful render refreshes after an older in-flight read", async () => {
  let release: (value: unknown) => void = () => {};
  let reads = 0;
  const session = new ObjectScenePreview(target, async (method) => {
    if (method.endsWith(".run")) return { runId: "r" };
    reads++;
    if (reads === 1)
      return new Promise((resolve) => {
        release = resolve;
      });
    throw new Error("fresh read reached");
  });
  const read = session.refresh();
  await session.render();
  release(null);
  await read;
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(reads, 2);
  assert.match(session.getSnapshot().error, /fresh read reached/);
  session.close();
});
test("attempt capture uses its own API and rejects another checkpoint result", async () => {
  const attempt: SceneTarget = {
    projectId: "p",
    runId: "r",
    attemptId: "a",
    checkpoint: "output",
    path: "scene.blend",
    sha256: "hash",
  };
  const methods: string[] = [];
  const session = new ObjectScenePreview(attempt, async (method) => {
    methods.push(method);
    if (method.endsWith(".run")) return { runId: "preview" };
    return {
      target: { ...attempt, checkpoint: "input" },
      sourceDigest: "digest",
      projectConfig: "frozen",
      integrityError: null,
      run: {
        id: "preview",
        projectId: "p",
        kind: "objectPreview",
        status: "queued",
        phase: "queued",
        error: null,
        engineVersion: "",
        runnerVersion: "",
        snapshotId: "snapshot",
        createdAt: "",
        finishedAt: null,
        evidence: [],
      },
    };
  });
  await session.render();
  assert.deepEqual(methods, [
    "object.attemptScenePreview.run",
    "object.attemptScenePreview.get",
  ]);
  assert.match(session.getSnapshot().error, /其他版本或尝试/);
  assert.equal(session.getSnapshot().result, null);
  session.close();
});
