import assert from "node:assert/strict";
import test from "node:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { ObjectAttemptFile } from "../src/ui/object-tasks/object-attempt-file";
import { ObjectAttemptFilePreview } from "../src/ui/object-tasks/ObjectAttemptFilePreview";

const identity = { projectId: "p", runId: "r", attemptId: "a" };
const hash = "a".repeat(64);
test("frozen media reaches production preview with bounded passive MIME content", async () => {
  for (const content of [
    { kind: "image", mime: "image/png", base64: "iVBORw==" },
    { kind: "audio", mime: "audio/wav", base64: "UklGRg==" },
  ]) {
    const viewer = new ObjectAttemptFile(identity, async (_, request) => ({
      request,
      byteCount: 4,
      content,
    }));
    await viewer.open("output", "media", hash);
    assert.deepEqual(viewer.getSnapshot().response?.content, content);
    assert.match(
      renderToStaticMarkup(
        createElement(ObjectAttemptFilePreview, { session: viewer }),
      ),
      /内容摘要已校验/,
    );
    viewer.cancel();
    assert.equal(
      renderToStaticMarkup(
        createElement(ObjectAttemptFilePreview, { session: viewer }),
      ),
      "",
    );
  }
  for (const content of [
    { kind: "image", mime: "image/svg+xml", base64: "PHN2Zz4=" },
    { kind: "image", mime: "image/png", base64: "!!!!" },
    {
      kind: "audio",
      mime: "audio/wav",
      base64: "A".repeat(4 * Math.ceil((1024 * 1024) / 3) + 4),
    },
  ]) {
    const viewer = new ObjectAttemptFile(identity, async (_, request) => ({
      request,
      byteCount: 4,
      content,
    }));
    await viewer.open("input", "media", hash);
    assert.equal(viewer.getSnapshot().response, null);
    assert.ok(viewer.getSnapshot().error);
  }
});

test("selection changes and closing ignore late responses", async () => {
  const pending: { input: unknown; resolve: (value: unknown) => void }[] = [];
  const viewer = new ObjectAttemptFile(identity, async (method, input) => {
    assert.equal(method, "objectTask.attemptFile");
    return new Promise((resolve) => pending.push({ input, resolve }));
  });
  const first = viewer.open("input", "old.txt", hash);
  const second = viewer.open("output", "new.txt", hash);
  assert.ok(pending[0] && pending[1]);
  pending[1].resolve({
    request: pending[1].input,
    byteCount: 3,
    content: { kind: "text", text: "new" },
  });
  await second;
  pending[0].resolve({
    request: pending[0].input,
    byteCount: 3,
    content: { kind: "text", text: "old" },
  });
  await first;
  assert.equal(viewer.getSnapshot().request?.path, "new.txt");
  const third = viewer.open("input", "closed.txt", hash);
  viewer.cancel();
  assert.ok(pending[2]);
  pending[2].resolve({
    request: pending[2].input,
    byteCount: 3,
    content: { kind: "text", text: "old" },
  });
  await third;
  assert.equal(viewer.getSnapshot().request, null);
});

test("foreign response identity fails closed and backend errors remain visible", async () => {
  const viewer = new ObjectAttemptFile(identity, async (_, input) => ({
    request: { ...(input as object), attemptId: "foreign" },
    byteCount: 0,
    content: { kind: "text", text: "" },
  }));
  await viewer.open("output", "hero.gd", hash);
  assert.match(viewer.getSnapshot().error, /不匹配/);
  assert.equal(viewer.getSnapshot().response, null);
  const failed = new ObjectAttemptFile(identity, async () => {
    throw new Error("HASH_MISMATCH");
  });
  await failed.open("input", "hero.gd", hash);
  assert.match(
    renderToStaticMarkup(
      createElement(ObjectAttemptFilePreview, { session: failed }),
    ),
    /HASH_MISMATCH/,
  );
});

test("text is escaped and binary or oversized content is explicit", async () => {
  for (const [content, expected] of [
    [{ kind: "text", text: "<script>alert(1)</script>" }, /&lt;script&gt;/],
    [{ kind: "binary" }, /二进制或非 UTF-8/],
    [{ kind: "tooLarge" }, /未校验文件完整性/],
  ] as const) {
    const viewer = new ObjectAttemptFile(identity, async (_, request) => ({
      request,
      byteCount: 10,
      content,
    }));
    await viewer.open("output", "hero.txt", hash);
    const html = renderToStaticMarkup(
      createElement(ObjectAttemptFilePreview, { session: viewer }),
    );
    assert.match(html, expected);
    assert.doesNotMatch(html, /<script>/);
  }
});
