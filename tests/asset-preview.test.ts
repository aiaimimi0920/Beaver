import assert from "node:assert/strict";
import test from "node:test";
import { setImmediate } from "node:timers/promises";
import type { AssetFrame } from "../src/shared/asset-task";
import {
  defaultView,
  imagePoint,
  LatestView,
  moveView,
  newerFrame,
  zoomView,
} from "../src/ui/asset-task/preview-controls";

const frame = (overrides: Partial<AssetFrame> = {}): AssetFrame => ({
  id: "frame-1",
  sessionId: "session-1",
  generation: "scene-1",
  sceneRevision: 3,
  viewRevision: 4,
  capturedAt: 1000,
  width: 720,
  height: 540,
  viewMatrix: Array<number>(16).fill(0),
  projectionMatrix: Array<number>(16).fill(0),
  ...overrides,
});

test("annotations use contained image coordinates across resize and display density", () => {
  const rect = { left: 20, top: 30, width: 800, height: 600 };
  assert.deepEqual(imagePoint(rect, 1920, 1080, 220, 442.5), [0.25, 0.75]);
  assert.deepEqual(imagePoint(rect, 3840, 2160, 220, 442.5), [0.25, 0.75]);
  assert.deepEqual(
    imagePoint({ ...rect, width: 400, height: 300 }, 1920, 1080, 120, 236.25),
    [0.25, 0.75],
  );
  assert.equal(imagePoint(rect, 1920, 1080, 400, 50), null);
  assert.equal(imagePoint(rect, 600, 800, 30, 200), null);
  assert.deepEqual(imagePoint(rect, 600, 800, 420, 330), [0.5, 0.5]);
  assert.equal(imagePoint(rect, 0, 1080, 420, 330), null);
  assert.equal(imagePoint(rect, 1920, 1080, NaN, 330), null);
  assert.equal(
    imagePoint({ ...rect, width: Infinity }, 1920, 1080, 420, 330),
    null,
  );
});

test("orbit and pan preserve the input while zoom stays within supported range", () => {
  const original = structuredClone({ ...defaultView, yaw: 0, pitch: 0 });
  const copy = structuredClone(original);
  const moved = moveView(original, 20, 10, true);
  assert.deepEqual(moved.target, [-0.2, 0, 1.1]);
  assert.equal(moved.yaw, original.yaw);
  assert.equal(moveView(original, 20, 1000, false).pitch, 1.5);
  assert.equal(moveView(original, 20, -1000, false).pitch, -1.5);
  assert.equal(moveView(original, 20, 0, false).yaw, -0.16);
  assert.equal(
    zoomView({ ...original, distance: 0.02 }, -10000).distance,
    0.02,
  );
  assert.equal(
    zoomView({ ...original, distance: 100000 }, 10000).distance,
    100000,
  );
  assert.ok(zoomView(original, -100).distance < original.distance);
  assert.deepEqual(original, copy);
});

test("display rejects duplicate and regressing frames but accepts a fresh session", () => {
  const previous = frame();
  const next = frame({ id: "frame-2", capturedAt: 1001 });
  assert.equal(newerFrame(undefined, previous), true);
  assert.equal(newerFrame(previous, next), true);
  assert.equal(newerFrame(previous, frame({ capturedAt: 1001 })), false);
  for (const change of [
    { capturedAt: 999 },
    { viewRevision: 3 },
    { sceneRevision: 2 },
  ]) {
    assert.equal(newerFrame(previous, { ...next, ...change }), false);
  }
  assert.equal(
    newerFrame(previous, frame({ sessionId: "session-2", sceneRevision: 0 })),
    true,
  );
  assert.equal(
    newerFrame(previous, frame({ generation: "reloaded", sceneRevision: 0 })),
    true,
  );
});

test("a slow view transport sends only the first and latest drag positions", async () => {
  let release!: () => void;
  const gate = new Promise<void>((resolve) => {
    release = resolve;
  });
  const sent: number[] = [];
  const errors: unknown[] = [];
  const queue = new LatestView(
    async (view) => {
      sent.push(view.seq);
      if (view.seq === 1) await gate;
    },
    (error) => errors.push(error),
  );
  for (let seq = 1; seq <= 1000; seq++) queue.set({ ...defaultView, seq });
  assert.deepEqual(sent, [1]);
  release();
  await setImmediate();
  assert.deepEqual(sent, [1, 1000]);
  assert.deepEqual(errors, []);
  queue.dispose();
});

test("closing the window drops queued commands and late transport failures", async () => {
  let reject!: (error: Error) => void;
  const gate = new Promise<void>((_resolve, fail) => {
    reject = fail;
  });
  const sent: number[] = [];
  const errors: unknown[] = [];
  const queue = new LatestView(
    async (view) => {
      sent.push(view.seq);
      await gate;
    },
    (error) => errors.push(error),
  );
  queue.set({ ...defaultView, seq: 1 });
  queue.set({ ...defaultView, seq: 2 });
  queue.dispose();
  queue.set({ ...defaultView, seq: 3 });
  reject(new Error("window closed"));
  await setImmediate();
  assert.deepEqual(sent, [1]);
  assert.deepEqual(errors, []);
});

test("a failed view request reports the error and still sends the latest target", async () => {
  const failure = new Error("busy transport");
  const sent: number[] = [];
  const errors: unknown[] = [];
  const queue = new LatestView(
    async (view) => {
      sent.push(view.seq);
      if (view.seq === 1) throw failure;
    },
    (error) => errors.push(error),
  );
  queue.set({ ...defaultView, seq: 1 });
  queue.set({ ...defaultView, seq: 2 });
  await setImmediate();
  assert.deepEqual(sent, [1, 2]);
  assert.deepEqual(errors, [failure]);
  queue.dispose();
});
