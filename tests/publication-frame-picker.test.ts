import assert from "node:assert/strict";
import test from "node:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { parsePublicationFrames } from "../src/ui/object-tasks/PublicationFramePicker";
import { parseAttemptFrames } from "../src/ui/object-tasks/AttemptFramePicker";
import { PreviewFrameRegions } from "../src/ui/object-preview/PreviewFrameRegions";
import { ObjectPublicationFollowup } from "../src/ui/object-tasks/object-publication-followup";
import type { PublicationOperation } from "../src/shared/object-publication";

const archive = {
  id: "frozen-frame",
  projectId: "p",
  runId: "preview-run",
  snapshotId: "snapshot",
  savedAt: "today",
  source: {
    runId: "preview-run",
    sourceDigest: "source",
    projectConfig: "project.godot",
    target: {
      projectId: "p",
      objectId: "hero",
      versionId: "v1",
      path: "hero.tscn",
      sha256: "b".repeat(64),
    },
  },
  frame: {
    sessionId: "s",
    sequence: 2,
    revision: 1,
    frozen: true,
    width: 64,
    height: 64,
    sha256: "a".repeat(64),
    dataUrl: "data:image/png;base64,AA==",
    engine: "4.4",
    camera: {
      transform: Array.from({ length: 4 }, () => [0, 0, 0]),
      projection: Array.from({ length: 4 }, () => [0, 0, 0, 0]),
      near: 0.1,
      far: 100,
      mode: 0,
    },
  },
  selection: {
    kind: "image-regions",
    sequence: 2,
    sha256: "a".repeat(64),
    regions: [{ x: 0.1, y: 0.2, width: 0.3, height: 0.4, prompt: "move this" }],
    prompt: "keep the rest",
    coordinateSpace: "normalized-image",
    hitCapability: "unavailable",
  },
};

test("attempt picker replays original regions and rejects another attempt, checkpoint or reference", () => {
  const item = {
    ...archive,
    source: {
      ...archive.source,
      target: {
        projectId: "p",
        runId: "r",
        attemptId: "a",
        checkpoint: "output",
        path: "hero.tscn",
        sha256: "b".repeat(64),
      },
    },
  };
  const reference = { runId: item.runId, frameId: item.id };
  const [parsed] = parseAttemptFrames([item], "p", "r", "a", reference);
  const html = renderToStaticMarkup(
    createElement(PreviewFrameRegions, {
      frame: parsed!.frame,
      selection: parsed!.selection,
    }),
  );
  assert.match(html, /move this/);
  assert.match(html, /keep the rest/);
  for (const field of ["projectId", "runId", "attemptId", "checkpoint"]) {
    const invalid = structuredClone(item);
    Object.assign(invalid.source.target, { [field]: "foreign" });
    assert.throws(() => parseAttemptFrames([invalid], "p", "r", "a"));
  }
  assert.throws(
    () =>
      parseAttemptFrames([item], "p", "r", "a", {
        ...reference,
        frameId: "other",
      }),
    /REFERENCE_MISMATCH/,
  );
  assert.throws(
    () => parseAttemptFrames([], "p", "r", "a", reference),
    /REFERENCE_MISMATCH/,
  );
});
// Only publication identity is consumed by this read-only picker.
const publication = {
  versionId: "v1",
  request: { projectId: "p", requestId: "pub", target: { objectId: "hero" } },
} as PublicationOperation;
const session = () =>
  new ObjectPublicationFollowup(
    publication,
    async (method, input) => {
      assert.equal(method, "objectTask.publicationFrames");
      assert.deepEqual(input, { projectId: "p", publicationRequestId: "pub" });
      return [archive];
    },
    async () => {},
    undefined,
    { getItem: () => null, setItem: () => {} },
  );

test("published-frame picker reads exact-version archive and replays its numbered regions", async () => {
  const view = session();
  const [item] = parsePublicationFrames(await view.frames(), view);
  assert.ok(item);
  const html = renderToStaticMarkup(
    createElement(PreviewFrameRegions, {
      frame: item.frame,
      selection: item.selection,
    }),
  );
  assert.match(html, /move this/);
  assert.match(html, /keep the rest/);
  assert.ok(html.includes("data:image/png;base64,AA=="));
  assert.match(html, /区域 1 意见/);
});

test("picker rejects foreign versions, runs, source types and unfrozen or mismatched regions", () => {
  for (const change of [
    (v: typeof archive) => {
      v.source.target.versionId = "v2";
    },
    (v: typeof archive) => {
      v.source.target.objectId = "other";
    },
    (v: typeof archive) => {
      v.source.runId = "other";
    },
    (v: typeof archive) => {
      v.projectId = "other";
    },
    (v: typeof archive) => {
      v.frame.frozen = false;
    },
    (v: typeof archive) => {
      v.selection.sequence = 3;
    },
    (v: typeof archive) => {
      v.selection.sha256 = "c".repeat(64);
    },
  ]) {
    const invalid = structuredClone(archive);
    change(invalid);
    assert.throws(
      () => parsePublicationFrames([invalid], session()),
      /MISMATCH/,
    );
  }
  const invalid = {
    ...archive,
    source: {
      ...archive.source,
      target: {
        projectId: "p",
        runId: "r",
        attemptId: "a",
        checkpoint: "output",
        path: "hero.tscn",
        sha256: "b".repeat(64),
      },
    },
  };
  assert.throws(() => parsePublicationFrames([invalid], session()), /MISMATCH/);
});
