import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";
import {
  parseFileSourceSnapshot,
  type FileSourceSnapshot,
} from "../src/shared/object-import";

function validSnapshot(): FileSourceSnapshot {
  return {
    source: { kind: "files", paths: ["C:/assets"] },
    files: [
      {
        sourcePath: "C:/assets",
        relativePath: "hero.glb",
        path: "C:/assets/hero.glb",
        kind: "model",
        bytes: 12,
        sha256: "a".repeat(64),
      },
    ],
    digest: "b".repeat(64),
  };
}

test("file source contract accepts a frozen read-only snapshot", () => {
  assert.deepEqual(parseFileSourceSnapshot(validSnapshot()), validSnapshot());
});

test("file source contract rejects missing, extra and malformed fields", () => {
  const input = validSnapshot();
  const invalid: unknown[] = [
    { ...input, digest: "wrong" },
    { ...input, source: { kind: "project", paths: ["C:/assets"] } },
    { ...input, source: { kind: "files", paths: [] } },
    { ...input, source: { kind: "files", paths: ["C:/assets"], extra: true } },
    {
      ...input,
      files: [{ ...input.files[0], bytes: 1.5 }],
    },
    {
      ...input,
      files: [{ ...input.files[0], extra: true }],
    },
    { ...input, files: [{ ...input.files[0], sha256: "not-a-digest" }] },
    { source: input.source, files: input.files },
  ];
  for (const value of invalid)
    assert.throws(() => parseFileSourceSnapshot(value));
});

test("object import UI exposes file pickers and honest manual grouping boundaries", () => {
  const source = fs.readFileSync(
    path.join(
      process.cwd(),
      "src",
      "ui",
      "object-preview",
      "ObjectImportDialog.tsx",
    ),
    "utf8",
  );
  for (const text of [
    "选择文件",
    "选择文件夹",
    "不会按目录、扩展名或相近名称自动归组",
    "普通文件可先预览和人工归组；保存准备后，在下方历史中打开记录并确认正式导入。",
    'type="submit"',
  ])
    assert.match(
      source,
      new RegExp(text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")),
    );
  assert.doesNotMatch(source, /单独文件和文件夹导入尚未接通/);
});
