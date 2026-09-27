import assert from "node:assert/strict";
import test from "node:test";
import { parseSavedFrames } from "../src/ui/object-preview/saved-preview-frames";
import {
  LiveScenePreview,
  initialCamera,
} from "../src/ui/object-preview/live-scene-preview";

function response(sequence = 1) {
  return {
    projectId: "p",
    runId: "r",
    snapshotId: "snapshot",
    sessionId: "s",
    status: "ready",
    error: null,
    frame: {
      sessionId: "s",
      sequence,
      revision: 0,
      width: 960,
      height: 540,
      sha256: "a".repeat(64),
      dataUrl: "data:image/png;base64,AA==",
      engine: "4",
      camera: {
        transform: Array.from({ length: 4 }, () => [0, 0, 0]),
        projection: Array.from({ length: 4 }, () => [0, 0, 0, 0]),
        near: 0.1,
        far: 100,
        mode: 0,
      },
    },
  };
}
function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}
const settle = () => new Promise<void>((done) => setImmediate(done));

test("freeze waits for engine acknowledgement, blocks camera edits, and resumes with a new revision", async () => {
  const writes: Record<string, unknown>[] = [];
  let current = {
    ...response(),
    frame: { ...response().frame, frozen: false },
  };
  const session = new LiveScenePreview(
    "p",
    "r",
    "snapshot",
    async (method, input) => {
      if (method.endsWith("view"))
        writes.push(input as Record<string, unknown>);
      return current;
    },
  );
  await session.refresh();
  session.setFrozen(true);
  await settle();
  await assert.rejects(session.capture(), /PREVIEW_VIEW_PENDING/);
  session.setCamera({ ...initialCamera(), yaw: 10 });
  session.setResolution("720p");
  assert.equal(writes.length, 1);
  assert.equal(writes[0]?.frozen, true);
  current = {
    ...current,
    frame: { ...current.frame, sequence: 2, revision: 1, frozen: true },
  };
  await session.refresh();
  assert.equal(session.getSnapshot().frame?.frozen, true);
  session.setFrozen(false);
  await settle();
  await session.refresh();
  await assert.rejects(session.capture(), /PREVIEW_VIEW_PENDING/);
  assert.equal(writes[1]?.revision, 2);
  assert.equal(writes[1]?.frozen, false);
  current = {
    ...current,
    frame: { ...current.frame, sequence: 3, revision: 2, frozen: false },
  };
  await session.refresh();
  assert.equal(session.getSnapshot().frame?.frozen, false);
  session.close();
});

test("capture retries an uncertain write with the same identity; pending views cannot be saved", async () => {
  const writes: unknown[] = [];
  const session = new LiveScenePreview(
    "p",
    "r",
    "snapshot",
    async (method, input) => {
      if (method.endsWith("capture")) {
        writes.push(input);
        if (writes.length === 1)
          throw new Error("connection lost after commit");
        return { id: "saved" };
      }
      return response();
    },
  );
  await session.refresh();
  await assert.rejects(session.capture(), /connection lost/);
  assert.deepEqual(await session.capture(), { id: "saved" });
  assert.deepEqual(writes[0], writes[1]);
  session.setResolution("720p");
  await assert.rejects(session.capture(), /PREVIEW_VIEW_PENDING/);
  assert.equal(writes.length, 2);
  session.close();
});

test("saved replay rejects another target, checkpoint or snapshot without retargeting", () => {
  const target = {
    projectId: "p",
    runId: "task-run",
    attemptId: "attempt",
    checkpoint: "output" as const,
    path: "scene.tscn",
    sha256: "a".repeat(64),
  };
  const item = {
    id: "saved",
    projectId: "p",
    runId: "r",
    snapshotId: "snapshot",
    savedAt: "now",
    frame: response().frame,
    source: {
      target,
      runId: "r",
      sourceDigest: "digest",
      projectConfig: "frozen",
    },
  };
  const raw = {
    projectId: "p",
    runId: "r",
    snapshotId: "snapshot",
    frames: [item],
  };
  assert.equal(
    parseSavedFrames(raw, target, "r", "snapshot")[0]?.frame.sha256,
    item.frame.sha256,
  );
  assert.throws(
    () =>
      parseSavedFrames(
        raw,
        { ...target, checkpoint: "input" },
        "r",
        "snapshot",
      ),
    /SOURCE_MISMATCH/,
  );
  assert.throws(
    () =>
      parseSavedFrames(
        raw,
        { ...target, attemptId: "another" },
        "r",
        "snapshot",
      ),
    /SOURCE_MISMATCH/,
  );
  assert.throws(
    () => parseSavedFrames(raw, target, "r", "new-snapshot"),
    /IDENTITY_MISMATCH/,
  );
});

test("resolution and camera coalesce atomically; late old-size frames retain the last image", async () => {
  let next = response();
  const pending = deferred<unknown>();
  const writes: {
    revision: number;
    width: number;
    height: number;
    camera: ReturnType<typeof initialCamera>;
  }[] = [];
  const session = new LiveScenePreview(
    "p",
    "r",
    "snapshot",
    async (method, input) => {
      if (!method.endsWith("view")) return next;
      writes.push(input as (typeof writes)[number]);
      return writes.length === 1 ? pending.promise : {};
    },
  );
  await session.refresh();
  session.setCamera({ ...initialCamera(), yaw: 25 });
  session.setResolution("720p");
  session.setResolution("1080p");
  pending.resolve({});
  await settle();
  assert.deepEqual(
    writes.map(({ revision, width, height, camera }) => [
      revision,
      width,
      height,
      camera.yaw,
    ]),
    [
      [1, 960, 540, 25],
      [3, 1920, 1080, 25],
    ],
  );
  for (const frame of [
    { ...response(2).frame, revision: 1 },
    { ...response(3).frame, revision: 3 },
    { ...response(4).frame, revision: 2, width: 1920, height: 1080 },
  ]) {
    next = { ...response(), frame };
    await session.refresh();
    assert.equal(session.getSnapshot().frame?.sequence, 1);
  }
  next = {
    ...response(),
    frame: { ...response(5).frame, revision: 3, width: 1920, height: 1080 },
  };
  await session.refresh();
  assert.equal(session.getSnapshot().frame?.width, 1920);
  assert.equal(session.getSnapshot().resolution, "1080p");
  session.close();
});

test("lost open response retries the same request; late open after close releases its session", async () => {
  const inputs: unknown[] = [];
  const pending = deferred<unknown>();
  const closed: unknown[] = [];
  const session = new LiveScenePreview(
    "p",
    "r",
    "snapshot",
    async (method, input) => {
      if (method.endsWith("close")) {
        closed.push(input);
        return {};
      }
      inputs.push(input);
      if (inputs.length === 1) throw new Error("lost response");
      return pending.promise;
    },
  );
  await session.refresh();
  const opening = session.refresh();
  session.close();
  pending.resolve(response());
  await opening;
  assert.deepEqual(inputs[0], inputs[1]);
  assert.deepEqual(closed, [{ projectId: "p", sessionId: "s" }]);
  assert.equal(session.getSnapshot().frame, null);
});

test("foreign sources, foreign frame sessions and stale frames cannot replace the displayed frame", async () => {
  let next = response(3);
  const session = new LiveScenePreview("p", "r", "snapshot", async () => next);
  await session.refresh();
  for (const invalid of [
    { ...response(4), snapshotId: "other" },
    { ...response(4), sessionId: "other" },
    { ...response(4), frame: { ...response(4).frame, sessionId: "other" } },
    response(2),
  ]) {
    next = invalid;
    await session.refresh();
    assert.equal(session.getSnapshot().frame?.sequence, 3);
    assert.match(session.getSnapshot().error, /MISMATCH|STALE/);
  }
  session.close();
});

test("camera writes coalesce and a failed latest write is retried by polling", async () => {
  const pending = deferred<unknown>();
  const writes: {
    revision: number;
    camera: ReturnType<typeof initialCamera>;
  }[] = [];
  const session = new LiveScenePreview(
    "p",
    "r",
    "snapshot",
    async (method, input) => {
      if (!method.endsWith("view")) return response();
      writes.push(input as (typeof writes)[number]);
      if (writes.length === 1) return pending.promise;
      if (writes.length === 2) throw new Error("write failed");
      return {};
    },
  );
  await session.refresh();
  session.setCamera({ ...initialCamera(), yaw: 1 });
  session.setCamera({ ...initialCamera(), yaw: 2 });
  session.setCamera({ ...initialCamera(), yaw: 3 });
  pending.resolve({});
  await settle();
  assert.deepEqual(
    writes.map((write) => write.revision),
    [1, 3],
  );
  assert.match(session.getSnapshot().error, /write failed/);
  await session.refresh();
  await settle();
  assert.deepEqual(
    writes.map((write) => write.revision),
    [1, 3, 3],
  );
  assert.equal(writes[2]?.camera.yaw, 3);
  assert.equal(session.getSnapshot().error, "");
  session.close();
});
