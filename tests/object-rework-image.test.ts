import assert from "node:assert/strict";
import test from "node:test";
import {
  normalizedRegion,
  objectReworkImageSchema,
} from "../src/shared/object-rework-image";
import { ObjectTaskExecution } from "../src/ui/object-tasks/object-task-execution";
import { execution } from "./fixtures/object-attempts";
import { objectTaskSnapshot } from "./fixtures/object-tasks";

test("reverse drag clips to image bounds; click and nonfinite coordinates cannot submit regions", () => {
  assert.deepEqual(normalizedRegion({ x: 0.8, y: 0.9 }, { x: -1, y: 0.2 }), {
    x: 0,
    y: 0.2,
    width: 0.8,
    height: 0.7,
    prompt: "",
  });
  assert.equal(normalizedRegion({ x: 0.5, y: 0.5 }, { x: 0.5, y: 0.5 }), null);
  assert.equal(normalizedRegion({ x: NaN, y: 0 }, { x: 1, y: 1 }), null);
  const image = {
    path: "preview.png",
    sha256: "a".repeat(64),
    width: 20,
    height: 10,
    regions: [{ x: 0, y: 0, width: 1, height: 1, prompt: "" }],
  };
  assert.equal(objectReworkImageSchema.safeParse(image).success, true);
  for (const invalid of [
    { ...image, width: 16384, height: 16384 },
    { ...image, regions: [] },
    { ...image, path: "preview.svg" },
    { ...image, regions: [{ ...image.regions[0], x: 0.5 }] },
    { ...image, regions: [{ ...image.regions[0], prompt: "验".repeat(334) }] },
  ]) {
    assert.equal(objectReworkImageSchema.safeParse(invalid).success, false);
  }
});

test("feedback readers isolate reviews and general preview; stale file responses cannot replace new selection", async () => {
  const pending: (() => void)[] = [];
  const session = new ObjectTaskExecution(
    "p",
    objectTaskSnapshot(),
    "medium",
    async (_method, input) => {
      await new Promise<void>((resolve) => pending.push(resolve));
      return { request: input, byteCount: 0, content: { kind: "binary" } };
    },
    async () => true,
    () => "r",
  );
  const attempt = execution("awaitingGate").attempt;
  const viewer = session.filesFor(attempt, "feedback:review-a");
  assert.notEqual(viewer, session.filesFor(attempt));
  assert.notEqual(viewer, session.filesFor(attempt, "feedback:review-b"));
  const first = viewer.open("output", "old.png", "a".repeat(64));
  const second = viewer.open("output", "new.png", "b".repeat(64));
  pending[1]!();
  await second;
  pending[0]!();
  await first;
  assert.equal(viewer.getSnapshot().response?.request.path, "new.png");
});
