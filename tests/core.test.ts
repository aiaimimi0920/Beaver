import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { Files, ProjectLocks, safePath } from "../src/core/files";
import { Store } from "../src/core/store";
import { Projects } from "../src/core/projects";
import { Preferences, validBase } from "../src/core/settings";
import { defaultSettings, type Task } from "../src/shared/types";

async function fixture() {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), "beaver-test-"));
  const project = path.join(root, "project");
  await fs.mkdir(project);
  return { root, project, files: new Files(path.join(root, "state")) };
}
test("binary snapshot and deterministic rollback preserve exact bytes", async () => {
  const f = await fixture();
  try {
    const binary = Buffer.from([0, 255, 12, 128, 1]);
    await fs.writeFile(path.join(f.project, "image.png"), binary);
    const before = await f.files.capture(f.project);
    await fs.writeFile(path.join(f.project, "image.png"), Buffer.from([4, 5]));
    await fs.writeFile(path.join(f.project, "new.gd"), "new");
    const after = await f.files.capture(f.project);
    const changes = f.files.changes(before, after);
    await f.files.apply(
      f.project,
      changes.map((c) => ({ path: c.path, before: c.after, after: c.before })),
    );
    assert.deepEqual(
      await fs.readFile(path.join(f.project, "image.png")),
      binary,
    );
    assert.deepEqual(await f.files.capture(f.project), before);
  } finally {
    await fs.rm(f.root, { recursive: true, force: true });
  }
});
test("parallel task disjoint merges and rollback retain unrelated results", async () => {
  const f = await fixture();
  try {
    await fs.writeFile(path.join(f.project, "main.gd"), "base");
    const base = await f.files.capture(f.project);
    const a = path.join(f.root, "a"),
      b = path.join(f.root, "b");
    await f.files.restoreCopy(base, a);
    await f.files.restoreCopy(base, b);
    await fs.writeFile(path.join(a, "sprite.png"), "sprite");
    await fs.writeFile(path.join(b, "sound.wav"), "sound");
    const ca = f.files.changes(base, await f.files.capture(a)),
      cb = f.files.changes(base, await f.files.capture(b));
    await f.files.apply(f.project, ca);
    await f.files.apply(f.project, cb);
    await f.files.apply(
      f.project,
      ca.map((c) => ({ path: c.path, before: c.after, after: c.before })),
    );
    assert.equal(
      await fs.readFile(path.join(f.project, "sound.wav"), "utf8"),
      "sound",
    );
  } finally {
    await fs.rm(f.root, { recursive: true, force: true });
  }
});
test("same-file conflicts reject the whole merge before any writes", async () => {
  const f = await fixture();
  try {
    await fs.writeFile(path.join(f.project, "shared.gd"), "base");
    const base = await f.files.capture(f.project);
    const workspace = path.join(f.root, "task");
    await f.files.restoreCopy(base, workspace);
    await fs.writeFile(path.join(workspace, "shared.gd"), "task change");
    await fs.writeFile(path.join(workspace, "a-new.txt"), "new");
    const changes = f.files.changes(base, await f.files.capture(workspace));
    await fs.writeFile(path.join(f.project, "shared.gd"), "other task");
    await assert.rejects(f.files.apply(f.project, changes), /冲突/);
    assert.equal(
      await fs.readFile(path.join(f.project, "shared.gd"), "utf8"),
      "other task",
    );
    await assert.rejects(fs.access(path.join(f.project, "a-new.txt")));
  } finally {
    await fs.rm(f.root, { recursive: true, force: true });
  }
});
test("retained files stay while other task changes roll back", async () => {
  const f = await fixture();
  try {
    const before = await f.files.capture(f.project);
    await fs.writeFile(path.join(f.project, "keep.png"), "keep");
    await fs.writeFile(path.join(f.project, "discard.gd"), "discard");
    const changes = f.files.changes(before, await f.files.capture(f.project));
    await f.files.apply(
      f.project,
      changes
        .filter((c) => c.path !== "keep.png")
        .map((c) => ({ path: c.path, before: c.after, after: c.before })),
    );
    assert.equal(
      await fs.readFile(path.join(f.project, "keep.png"), "utf8"),
      "keep",
    );
    await assert.rejects(fs.access(path.join(f.project, "discard.gd")));
  } finally {
    await fs.rm(f.root, { recursive: true, force: true });
  }
});
test("path traversal, absolute paths and junction escapes are rejected", async () => {
  const f = await fixture();
  try {
    for (const p of [
      "../escape",
      "C:/secret",
      "/absolute",
      "a/../b",
      "a\\b",
      "a:stream",
    ])
      await assert.rejects(safePath(f.project, p));
    const external = path.join(f.root, "outside");
    await fs.mkdir(external);
    await fs.symlink(external, path.join(f.project, "linked"), "junction");
    await assert.rejects(safePath(f.project, "linked/file"));
    await assert.rejects(f.files.capture(f.project));
  } finally {
    await fs.rm(f.root, { recursive: true, force: true });
  }
});
test("project locks serialize writes but not unrelated projects", async () => {
  const lock = new ProjectLocks();
  const order: string[] = [];
  let release!: () => void;
  const gate = new Promise<void>((r) => {
    release = r;
  });
  const a = lock.run("same", async () => {
    order.push("a");
    await gate;
    order.push("a-end");
  });
  const b = lock.run("same", async () => {
    order.push("b");
  });
  await lock.run("other", async () => {
    order.push("other");
  });
  assert.ok(!order.includes("b"));
  release();
  await Promise.all([a, b]);
  assert.ok(order.indexOf("b") > order.indexOf("a-end"));
});
test("existing projects imported without template overwrite; duplicate import stable", async () => {
  const f = await fixture();
  const store = new Store(path.join(f.root, "db"));
  try {
    const projects = new Projects(store, path.resolve("resources"));
    await assert.rejects(projects.import(f.project));
    await fs.writeFile(
      path.join(f.project, "project.godot"),
      "config_version=5\n",
    );
    const p = await projects.import(f.project);
    assert.equal((await projects.import(f.project)).id, p.id);
    assert.equal(
      await fs.readFile(path.join(f.project, "project.godot"), "utf8"),
      "config_version=5\n",
    );
    const created = await projects.create(f.root, "Original", "nightbar");
    assert.ok(
      (await projects.assets(created.id)).some((a) => a.path === "main.gd"),
    );
    await assert.rejects(projects.create(f.root, "Original", "nightbar"));
  } finally {
    store.close();
    await fs.rm(f.root, { recursive: true, force: true });
  }
});
test("settings do not return keys and preserve key on empty update", async () => {
  const f = await fixture();
  const store = new Store(path.join(f.root, "db"));
  try {
    const prefs = new Preferences(store, {
      encrypt: (t) => Buffer.from(t).toString("base64"),
      decrypt: (t) => Buffer.from(t, "base64").toString(),
    });
    const s = defaultSettings();
    s.local.code = {
      baseUrl: "http://127.0.0.1:9999/v1",
      model: "test-model",
      route: "",
    };
    const result = prefs.save(s, { code: "test-secret" });
    assert.equal(result.local.code.hasKey, true);
    assert.ok(!JSON.stringify(result).includes("test-secret"));
    prefs.save(result, { code: "" });
    assert.equal(prefs.resolve("code").key, "test-secret");
    assert.equal(prefs.redact("test-secret"), "[REDACTED_SECRET]");
    prefs.clearKey("code");
    assert.equal(prefs.key("code"), "");
    assert.throws(() => validBase("https://user:pass@example.com"));
    assert.throws(() => validBase("http://example.com"));
  } finally {
    store.close();
    await fs.rm(f.root, { recursive: true, force: true });
  }
});
test("unfinished execution survives process restart as resumable", async () => {
  const f = await fixture();
  let store = new Store(path.join(f.root, "db"));
  try {
    store.put("task", "t", {
      id: "t",
      status: "running",
      threadId: "saved-thread",
      workspace: "saved-workspace",
    });
    store.event("t", "user", "goal");
    store.close();
    store = new Store(path.join(f.root, "db"));
    store.recover();
    const task = store.get<Task>("task", "t")!;
    assert.equal(task.status, "interrupted");
    assert.equal(task.threadId, "saved-thread");
    assert.equal(store.events("t")[0]?.text, "goal");
  } finally {
    store.close();
    await fs.rm(f.root, { recursive: true, force: true });
  }
});
