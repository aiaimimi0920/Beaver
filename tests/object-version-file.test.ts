import assert from "node:assert/strict";
import test from "node:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import {
  ObjectVersionFile,
  type VersionFileRequest,
} from "../src/ui/object-preview/object-version-file";
import { ObjectCatalogDetails } from "../src/ui/object-preview/ObjectCatalogDetails";
import { VersionFileView } from "../src/ui/object-preview/ObjectVersionFilePreview";
import type { ObjectCatalogRecord } from "../src/shared/object-catalog";
import {
  readObjectVersion,
  readFrozenObjectVersion,
} from "../src/shared/object-catalog";

const request: VersionFileRequest = {
  projectId: "project",
  objectId: "object",
  versionId: "v1",
  path: "old.txt",
  sha256: "a".repeat(64),
};
function response(input = request, text = "frozen") {
  return {
    request: input,
    byteCount: text.length,
    content: { kind: "text", text },
  };
}
function deferred() {
  let resolve!: (value: unknown) => void;
  const promise = new Promise<unknown>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}

test("preview reads exact identity and rejects mismatched responses", async () => {
  const calls: unknown[] = [];
  const session = new ObjectVersionFile(async (method, input) => {
    calls.push([method, input]);
    return response();
  });
  await session.open(request);
  assert.deepEqual(calls, [["object.versionFile", request]]);
  assert.equal(session.getSnapshot().response?.content.kind, "text");
  for (const key of Object.keys(request) as (keyof VersionFileRequest)[]) {
    const wrong = new ObjectVersionFile(async () =>
      response({
        ...request,
        [key]: key === "sha256" ? "b".repeat(64) : "wrong",
      }),
    );
    await wrong.open(request);
    assert.equal(wrong.getSnapshot().response, null);
    assert.match(wrong.getSnapshot().error, /不匹配/);
  }
});

test("switching version and closing cannot display a late response", async () => {
  const first = deferred();
  const second = deferred();
  let count = 0;
  const session = new ObjectVersionFile(() =>
    count++ === 0 ? first.promise : second.promise,
  );
  const old = session.open(request);
  const next = { ...request, versionId: "v2", sha256: "b".repeat(64) };
  const current = session.open(next);
  second.resolve(response(next, "new"));
  await current;
  first.resolve(response());
  await old;
  assert.deepEqual(session.getSnapshot().response?.request, next);
  const pending = deferred();
  const closing = new ObjectVersionFile(() => pending.promise);
  const loading = closing.open(request);
  closing.cancel();
  pending.resolve(response());
  await loading;
  assert.equal(closing.getSnapshot().request, null);
  assert.equal(closing.getSnapshot().response, null);
});

test("production content escapes active text and explains unsupported or oversized content", async () => {
  for (const [content, expected] of [
    [{ kind: "text", text: "<script>alert(1)</script>" }, /&lt;script&gt;/],
    [{ kind: "unsupported" }, /Godot 场景/],
    [{ kind: "tooLarge" }, /未校验内容摘要/],
  ] as const) {
    const session = new ObjectVersionFile(async () => ({
      ...response(),
      content,
    }));
    await session.open(request);
    const html = renderToStaticMarkup(
      createElement(VersionFileView, { session }),
    );
    assert.match(html, expected);
    assert.doesNotMatch(html, /<script>/);
  }
  const unsafe = new ObjectVersionFile(async () => ({
    ...response(),
    content: { kind: "image", mime: "image/svg+xml", base64: "AAAA" },
  }));
  await unsafe.open(request);
  assert.equal(unsafe.getSnapshot().response, null);
  assert.ok(unsafe.getSnapshot().error);
});

test("object details lists frozen version members rather than current registration files", () => {
  const object: ObjectCatalogRecord = {
    id: "object",
    projectId: "project",
    name: "Object",
    category: "其他",
    tags: [],
    thumbnailPath: null,
    parentObjectId: null,
    revision: 1,
    components: [],
    files: [{ path: "current.txt", role: "source" }],
    references: [],
    versions: [
      {
        versionId: "v1",
        manifest: {
          schemaVersion: 1,
          projectId: "project",
          objectId: "object",
          versionId: "v1",
          status: "accepted",
          name: "Old name",
          category: "其他",
          tags: [],
          thumbnailPath: null,
          parentObjectId: null,
          components: [],
          references: [],
          files: [
            {
              path: "old.txt",
              role: "source",
              bytes: 6,
              sha256: request.sha256,
            },
          ],
        },
      },
    ],
  };
  object.category = "Changed category";
  object.tags = ["changed"];
  object.parentObjectId = "new-parent";
  assert.equal(readObjectVersion(object, object.versions[0]!), null);
  assert.equal(
    readFrozenObjectVersion(object, object.versions[0]!)?.category,
    "其他",
  );
  const html = renderToStaticMarkup(
    createElement(ObjectCatalogDetails, {
      object,
      objects: [object],
      currentProjectId: "project",
      select: () => {},
    }),
  );
  assert.match(html, /版本文件预览/);
  assert.match(html, /<button type="button">old.txt<\/button>/);
  assert.doesNotMatch(html, /<button type="button">current.txt<\/button>/);
});
