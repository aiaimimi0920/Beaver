import assert from "node:assert/strict";
import test from "node:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import {
  parseObjectCatalog,
  type ObjectCatalogRecord,
} from "../src/shared/object-catalog";
import { ObjectCatalogDetails } from "../src/ui/object-preview/ObjectCatalogDetails";
import { ObjectCatalogGrid } from "../src/ui/object-preview/ObjectCatalogGrid";

(globalThis as unknown as { window: Record<string, unknown> }).window = {};

function record(
  overrides: Partial<ObjectCatalogRecord> = {},
): ObjectCatalogRecord {
  return {
    id: "object",
    projectId: "project",
    name: "Object",
    category: "其他",
    tags: [],
    thumbnailPath: null,
    parentObjectId: null,
    revision: 1,
    components: [],
    files: [],
    references: [],
    versions: [],
    ...overrides,
  };
}

function renderGrid(
  objects: ObjectCatalogRecord[],
  options: Partial<Parameters<typeof ObjectCatalogGrid>[0]> = {},
) {
  return renderToStaticMarkup(
    createElement(ObjectCatalogGrid, {
      objects,
      filter: "全部",
      search: "",
      tags: [],
      thumbnailSize: 240,
      select: () => {},
      ...options,
    }),
  );
}

test("old catalog records receive metadata defaults and nested records stay strict", () => {
  const oldRecord = {
    id: "legacy",
    projectId: "project",
    name: "Legacy",
    components: [],
    files: [],
    references: [],
    versions: [{ versionId: "v1", manifest: {} }],
  };
  const [parsed] = parseObjectCatalog([oldRecord], "project");
  assert.ok(parsed);
  assert.equal(parsed.category, "其他");
  assert.deepEqual(parsed.tags, []);
  assert.equal(parsed.thumbnailPath, null);
  assert.equal(parsed.parentObjectId, null);
  assert.throws(() =>
    parseObjectCatalog(
      [{ ...oldRecord, components: [{ id: "component", kind: "scene" }] }],
      "project",
    ),
  );
});

test("catalog grid filters real category and tags and searches metadata fields", () => {
  const image = record({
    id: "image",
    name: "Hero",
    category: "图像",
    tags: ["UI", "Hero"],
    thumbnailPath: "thumbs/hero.png",
    parentObjectId: "parent",
    files: [{ path: "assets/hero.png", role: "source" }],
  });
  const audio = record({
    id: "audio",
    name: "Music",
    category: "音频",
    tags: ["music"],
    files: [{ path: "audio/theme.wav", role: "source" }],
  });

  const filtered = renderGrid([image, audio], {
    filter: "图像",
    tags: ["ui", "hero"],
  });
  assert.match(filtered, /Hero/);
  assert.doesNotMatch(filtered, /Music/);

  for (const query of ["UI", "thumbs/hero", "parent", "assets/hero.png"]) {
    assert.match(renderGrid([image, audio], { search: query }), /Hero/);
  }
});

test("details keep parent and child navigation inside the current project", () => {
  const parent = record({ id: "parent", name: "Parent" });
  const child = record({
    id: "child",
    name: "Child",
    thumbnailPath: "thumbs/child.png",
    revision: 3,
    parentObjectId: "parent",
    references: [
      { projectId: "project", objectId: "parent", versionId: null },
      { projectId: "other", objectId: "foreign", versionId: null },
    ],
    versions: [{ versionId: "v3", manifest: { status: "accepted" } }],
  });
  const grandchild = record({
    id: "grandchild",
    name: "Grandchild",
    parentObjectId: "child",
  });
  const foreignChild = record({
    id: "foreign-child",
    projectId: "other",
    name: "Foreign child",
    parentObjectId: "child",
  });
  const markup = renderToStaticMarkup(
    createElement(ObjectCatalogDetails, {
      object: child,
      objects: [parent, child, grandchild, foreignChild],
      currentProjectId: "project",
      select: () => {},
    }),
  );
  assert.match(markup, /Parent/);
  assert.match(markup, /Grandchild/);
  assert.match(markup, /aria-label="显示的对象版本"/);
  assert.match(markup, /只读查看，不修改登记/);
  assert.match(markup, /beaver-asset:\/\/project\/thumbs\/child\.png\?v=3/);
  assert.match(markup, /已隐藏 1 个跨项目打开入口/);
  assert.doesNotMatch(markup, /Foreign child/);
});

test("objects without a thumbnail show an explicit placeholder", () => {
  const markup = renderGrid([record({ name: "No image" })]);
  assert.match(markup, /未引用缩略图/);
});
