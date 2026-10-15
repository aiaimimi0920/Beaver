import assert from "node:assert/strict";
import { mkdtemp, mkdir, rm, symlink, writeFile } from "node:fs/promises";
import { createRequire } from "node:module";
import os from "node:os";
import path from "node:path";
import test from "node:test";

const require = createRequire(import.meta.url);
const { HeadPreview } = require("../src/direct/preview.cjs");

async function fixture() {
  const root = await mkdtemp(path.join(os.tmpdir(), "beaver-head-preview-"));
  const workspace = path.join(root, "workspace");
  const engine = path.join(root, "fake-engine");
  await mkdir(path.join(workspace, "showcase"), { recursive: true });
  await writeFile(
    path.join(workspace, "showcase/aster_head_review.tscn"),
    "scene",
  );
  // Deliberately mismatched identity: accepted paths reach the hash guard,
  // and tests can never spawn an application or GUI.
  await writeFile(engine, "not the pinned MiDot executable");
  const preview = new HeadPreview(engine, path.join(root, "logs"));
  const context = { workspace, taskId: "test-task", runId: "test-run" };
  async function definition(name: string) {
    const relative = `assets/aster/${name}/aster_definition.tres`;
    await mkdir(path.dirname(path.join(workspace, relative)), {
      recursive: true,
    });
    await writeFile(path.join(workspace, relative), "definition");
    return relative;
  }
  return { root, workspace, preview, context, definition };
}

test("preview accepts established and safe general face names, retaining engine identity", async () => {
  const f = await fixture();
  try {
    for (const name of [
      "head_recovery_664",
      "general_face_667_base",
      "general_face_667_soft",
      "general_face_667_slender",
      "general_face_Test_09",
    ]) {
      const definition = await f.definition(name);
      await assert.rejects(
        f.preview.start(f.context, f.root, definition),
        /MiDot executable identity changed/,
      );
      assert.equal(f.preview.child, null);
    }
  } finally {
    await rm(f.root, { recursive: true, force: true });
  }
});

test("preview rejects traversal, unsupported names and malformed resource paths", async () => {
  const f = await fixture();
  try {
    for (const definition of [
      null,
      "assets/aster/general_face_/aster_definition.tres",
      "assets/aster/general_face_667-soft/aster_definition.tres",
      "assets/aster/general_face_../aster_definition.tres",
      "assets/aster/general_face_667/../aster_definition.tres",
      "assets/aster/general_face_667/other.tres",
      "assets/aster/head_recovery_abc/aster_definition.tres",
      "assets/aster/general_face_你好/aster_definition.tres",
      "assets/aster/general_face_667/aster_definition.tres\nextra",
      "assets/aster/general_face_667/aster_definition.tres\n",
      "assets/aster/general_face_667/aster_definition.tres\r",
      "res://assets/aster/general_face_667/aster_definition.tres",
      "/assets/aster/general_face_667/aster_definition.tres",
      "assets\\aster\\general_face_667\\aster_definition.tres",
    ]) {
      await assert.rejects(
        f.preview.start(f.context, f.root, definition),
        /Unsupported preview definition/,
      );
    }
  } finally {
    await rm(f.root, { recursive: true, force: true });
  }
});

test("preview retains realpath containment for workspace, definition and scene", async () => {
  const f = await fixture();
  const outside = await mkdtemp(
    path.join(os.tmpdir(), "beaver-preview-outside-"),
  );
  try {
    const definition = await f.definition("general_face_667_soft");
    await symlink(outside, path.join(f.root, "outside-workspace"), "dir");
    await assert.rejects(
      f.preview.start(
        { ...f.context, workspace: path.join(f.root, "outside-workspace") },
        f.root,
        definition,
      ),
      /Preview workspace outside project root/,
    );
    const definitionPath = path.join(f.workspace, definition);
    const externalFile = path.join(outside, "external.tres");
    await writeFile(externalFile, "external");
    await rm(definitionPath);
    await symlink(externalFile, definitionPath, "file");
    await assert.rejects(
      f.preview.start(f.context, f.root, definition),
      /Definition outside task workspace/,
    );
    await rm(definitionPath);
    await writeFile(definitionPath, "definition");
    const scenePath = path.join(f.workspace, "showcase/aster_head_review.tscn");
    await rm(scenePath);
    await symlink(externalFile, scenePath, "file");
    await assert.rejects(
      f.preview.start(f.context, f.root, definition),
      /Preview scene outside workspace/,
    );
  } finally {
    await rm(f.root, { recursive: true, force: true });
    await rm(outside, { recursive: true, force: true });
  }
});
