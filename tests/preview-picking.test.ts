import assert from "node:assert/strict";
import test from "node:test";
import { requestPreviewPick } from "../src/ui/object-preview/preview-picking";
import type { LiveFrame } from "../src/ui/object-preview/live-scene-preview";
import { previewSelectionSchema } from "../src/shared/preview-selection";
import { previewPickSchema } from "../src/shared/preview-pick";

const frame: LiveFrame = {
  sessionId: "s",
  sequence: 2,
  revision: 1,
  frozen: true,
  sha256: "a".repeat(64),
  width: 960,
  height: 540,
  dataUrl: "data:image/png;base64,AA==",
  engine: "4",
  camera: {
    transform: Array.from({ length: 4 }, () => [0, 0, 0]),
    projection: Array.from({ length: 4 }, () => [0, 0, 0, 0]),
    near: 0.1,
    far: 100,
    mode: 0,
  },
};
function receipt(input: unknown) {
  const { projectId: _project, ...identity } = input as Record<string, unknown>;
  return {
    ...identity,
    capability: "frozen-static-mesh-ray",
    triangles: 12,
    skipped: 1,
    hit: {
      nodePath: "Box",
      triangle: 0,
      position: [0, 0, 0.5],
      normal: [0, 0, 1],
      distance: 4,
    },
  };
}

test("box receipts bind exact drag bounds and preserve through-selection limits", async () => {
  const rectangle = { x: 0.1, y: 0.2, width: 0.3, height: 0.4 };
  const box = (input: unknown) => ({
    ...receipt(input),
    capability: "frozen-static-mesh-frustum",
    hit: null,
    nodePaths: ["Box", "Occluded"],
    truncated: true,
  });
  const result = await requestPreviewPick(
    async (_, input) => ({ status: "ready", result: box(input) }),
    "p",
    frame,
    { x: 0.2, y: 0.3 },
    () => true,
    rectangle,
  );
  assert.deepEqual(result.nodePaths, ["Box", "Occluded"]);
  assert.equal(result.truncated, true);
  const selection = {
    kind: "image-regions",
    sequence: 2,
    sha256: frame.sha256,
    regions: [{ ...rectangle, prompt: "move both" }],
    prompt: "",
    coordinateSpace: "normalized-image",
    hitCapability: "frozen-static-mesh",
    picks: [{ region: 0, result }],
  };
  assert.deepEqual(previewSelectionSchema.parse(selection), selection);
  assert.equal(
    previewSelectionSchema.safeParse({
      ...selection,
      regions: [{ ...rectangle, width: 0.4, prompt: "" }],
    }).success,
    false,
  );
  for (const changed of [
    { nodePaths: ["Box", "Box"] },
    { truncated: undefined },
    { hit: receipt({}).hit },
    { capability: "frozen-static-mesh-ray" },
  ]) {
    assert.equal(
      previewPickSchema.safeParse({ ...result, ...changed }).success,
      false,
    );
  }
  await assert.rejects(
    requestPreviewPick(
      async (_, input) => ({
        status: "ready",
        result: { ...box(input), rectangle: { ...rectangle, width: 0.4 } },
      }),
      "p",
      frame,
      { x: 0.2, y: 0.3 },
      () => true,
      rectangle,
    ),
    /FRAME_MISMATCH/,
  );
});

test("pick polling reuses identity and rejects stale or unrelated receipts", async () => {
  const requests: unknown[] = [];
  const result = await requestPreviewPick(
    async (method, input) => {
      assert.equal(method, "validation.preview.pick");
      requests.push(structuredClone(input));
      return requests.length === 1
        ? { status: "pending" }
        : { status: "ready", result: receipt(input) };
    },
    "p",
    frame,
    { x: 0.5, y: 0.5 },
    () => true,
  );
  assert.deepEqual(requests[0], requests[1]);
  assert.equal(result.hit?.nodePath, "Box");
  for (const changed of [
    { sequence: 3 },
    { sha256: "b".repeat(64) },
    { requestId: "other" },
  ]) {
    await assert.rejects(
      requestPreviewPick(
        async (_, input) => ({
          status: "ready",
          result: { ...receipt(input), ...changed },
        }),
        "p",
        frame,
        { x: 0.5, y: 0.5 },
        () => true,
      ),
      /FRAME_MISMATCH/,
    );
  }
  let current = true;
  await assert.rejects(
    requestPreviewPick(
      async (_, input) => {
        current = false;
        return { status: "ready", result: receipt(input) };
      },
      "p",
      frame,
      { x: 0.5, y: 0.5 },
      () => current,
    ),
    /FRAME_MISMATCH/,
  );
});

test("saved selection binds each hit or miss to its numbered image region", () => {
  const result = receipt({
    requestId: "r",
    sessionId: "s",
    revision: 1,
    sequence: 2,
    sha256: frame.sha256,
    point: { x: 0.5, y: 0.5 },
  });
  const selection = {
    kind: "image-regions",
    sequence: 2,
    sha256: frame.sha256,
    regions: [{ x: 0.49, y: 0.49, width: 0.02, height: 0.02, prompt: "move" }],
    prompt: "keep background",
    coordinateSpace: "normalized-image",
    hitCapability: "frozen-static-mesh-ray",
    picks: [{ region: 0, result }],
  };
  assert.deepEqual(previewSelectionSchema.parse(selection), selection);
  for (const picks of [
    [{ region: 1, result }],
    [
      { region: 0, result },
      { region: 0, result },
    ],
    [{ region: 0, result: { ...result, point: { x: 0.1, y: 0.1 } } }],
  ]) {
    assert.equal(
      previewSelectionSchema.safeParse({ ...selection, picks }).success,
      false,
    );
  }
  const miss = {
    ...selection,
    picks: [{ region: 0, result: { ...result, hit: null } }],
  };
  assert.equal(previewSelectionSchema.safeParse(miss).success, true);
  assert.equal(
    previewSelectionSchema.safeParse({
      ...selection,
      hitCapability: "unavailable",
    }).success,
    false,
  );
});
