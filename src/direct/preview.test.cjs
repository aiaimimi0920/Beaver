const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs/promises");
const path = require("node:path");
const os = require("node:os");
const { HeadPreview } = require("./preview.cjs");
async function fixture(fn) {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), "beaver-head-preview-"));
  try {
    const workspace = path.join(root, "projects", "task");
    await fs.mkdir(path.join(workspace, "showcase"), { recursive: true });
    const scene = path.join(workspace, "showcase", "aster_head_review.tscn");
    await fs.writeFile(scene, "[gd_scene format=3]");
    const engine = path.join(root, "wrong-engine");
    await fs.writeFile(engine, "not MiDot");
    const definition = "assets/aster/head_recovery_05/aster_definition.tres";
    await fs.mkdir(path.dirname(path.join(workspace, definition)), {
      recursive: true,
    });
    await fs.writeFile(
      path.join(workspace, definition),
      "[gd_resource format=3]",
    );
    await fn({
      root,
      workspace,
      scene,
      definition,
      preview: new HeadPreview(engine, path.join(root, "logs")),
    });
  } finally {
    await fs.rm(root, { recursive: true, force: true });
  }
}
test("rejects workspaces outside the project root", () =>
  fixture(async (f) => {
    await assert.rejects(
      f.preview.start(
        { workspace: f.root },
        path.join(f.root, "projects"),
        f.definition,
      ),
      /outside project root/,
    );
  }));
test("rejects a scene symlink leaving the task workspace", () =>
  fixture(async (f) => {
    await fs.unlink(f.scene);
    const external = path.join(f.root, "external.tscn");
    await fs.writeFile(external, "test");
    await fs.symlink(external, f.scene);
    await assert.rejects(
      f.preview.start(
        { workspace: f.workspace },
        path.join(f.root, "projects"),
        f.definition,
      ),
      /outside workspace/,
    );
  }));
test("rejects a changed engine before spawning", () =>
  fixture(async (f) => {
    await assert.rejects(
      f.preview.start(
        { workspace: f.workspace },
        path.join(f.root, "projects"),
        f.definition,
      ),
      /identity changed/,
    );
    assert.equal(f.preview.child, null);
  }));
test("repeated start cannot create a second owned process", () =>
  fixture(async (f) => {
    f.preview.child = {};
    await assert.rejects(
      f.preview.start(
        { workspace: f.workspace },
        path.join(f.root, "projects"),
        f.definition,
      ),
      /already running/,
    );
  }));
test("status and close before start are harmless", () =>
  fixture(async (f) => {
    assert.deepEqual(await f.preview.status(), { status: "not_started" });
    await f.preview.close();
  }));

test("rejects unsupported definition paths", () =>
  fixture(async (f) => {
    await assert.rejects(
      f.preview.start(
        { workspace: f.workspace },
        path.join(f.root, "projects"),
        "../other.tres",
      ),
      /Unsupported preview definition/,
    );
  }));
test("rejects a definition symlink outside the task", () =>
  fixture(async (f) => {
    const target = path.join(f.workspace, f.definition);
    await fs.unlink(target);
    const external = path.join(f.root, "external.tres");
    await fs.writeFile(external, "external");
    await fs.symlink(external, target);
    await assert.rejects(
      f.preview.start(
        { workspace: f.workspace },
        path.join(f.root, "projects"),
        f.definition,
      ),
      /Definition outside task workspace/,
    );
  }));
