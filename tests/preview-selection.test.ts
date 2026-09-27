import assert from "node:assert/strict";
import test from "node:test";
import { LiveScenePreview } from "../src/ui/object-preview/live-scene-preview";
import { previewSelectionSchema } from "../src/shared/preview-selection";

test("frozen annotation retries preserve the first payload and definitive rejection unlocks resume", async () => {
  let revision = 0;
  let frozen = false;
  let failure = "connection lost after commit";
  const writes: unknown[] = [];
  const session = new LiveScenePreview(
    "p",
    "r",
    "snapshot",
    async (method, input) => {
      if (method.endsWith("view")) {
        ({ revision, frozen } = input as { revision: number; frozen: boolean });
        return {};
      }
      if (method.endsWith("capture")) {
        writes.push(structuredClone(input));
        if (failure) throw new Error(failure);
        return { id: "saved" };
      }
      return {
        projectId: "p",
        runId: "r",
        snapshotId: "snapshot",
        sessionId: "s",
        status: "ready",
        error: null,
        frame: {
          sessionId: "s",
          sequence: revision + 1,
          revision,
          frozen,
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
    },
  );
  await session.refresh();
  const selection = previewSelectionSchema.parse({
    kind: "image-regions",
    sequence: 2,
    sha256: "a".repeat(64),
    regions: [{ x: 0.1, y: 0.2, width: 0.3, height: 0.4, prompt: "move" }],
    prompt: "keep rest",
    coordinateSpace: "normalized-image",
    hitCapability: "unavailable",
  });
  await assert.rejects(session.capture(selection), /FRAME_MISMATCH/);
  session.setFrozen(true);
  await new Promise<void>((done) => setImmediate(done));
  await session.refresh();
  await assert.rejects(
    session.capture({ ...selection, sequence: 3 }),
    /FRAME_MISMATCH/,
  );
  await assert.rejects(session.capture(selection), /connection lost/);
  selection.regions[0]!.prompt = "mutated by caller";
  session.setFrozen(false);
  assert.equal(session.getSnapshot().frozen, true);
  assert.equal(session.getSnapshot().capturePending, true);
  failure = "";
  await session.capture(selection);
  assert.deepEqual(writes[0], writes[1]);
  assert.equal(session.getSnapshot().capturePending, false);
  failure = "PREVIEW_SAVED_FRAME_LIMIT";
  await assert.rejects(session.capture(selection), /FRAME_LIMIT/);
  assert.equal(session.getSnapshot().capturePending, false);
  session.setFrozen(false);
  assert.equal(session.getSnapshot().frozen, false);
  session.close();
});

test("image regions reject forged hit capability and out-of-image rectangles", () => {
  const input = {
    kind: "image-regions",
    sequence: 1,
    sha256: "a".repeat(64),
    regions: [{ x: 0.8, y: 0.2, width: 0.3, height: 0.4, prompt: "" }],
    prompt: "",
    coordinateSpace: "normalized-image",
    hitCapability: "unavailable",
  };
  assert.equal(previewSelectionSchema.safeParse(input).success, false);
  input.regions[0]!.width = 0.1;
  assert.equal(previewSelectionSchema.safeParse(input).success, true);
  assert.equal(
    previewSelectionSchema.safeParse({ ...input, hitCapability: "mesh" })
      .success,
    false,
  );
});
