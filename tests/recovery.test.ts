import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { Store } from "../src/core/store";
import { Files } from "../src/core/files";
import { Journal, type FileOperation } from "../src/core/journal";
import { LocalExecutor } from "../src/core/executor";
import { CodexRpc } from "../src/core/rpc";
import { Preferences } from "../src/core/settings";
import { exportTarget } from "../src/core/game";
import { runCommand } from "../src/core/process";
import { defaultSettings, type Task, type Project } from "../src/shared/types";
import { Tasks } from "../src/core/tasks";
import { ProjectLocks } from "../src/core/files";
import { readDocument } from "../src/core/documents";
import { taskResources, taskResourceText } from "../src/core/task-resources";
import { taskAttention } from "../src/shared/task-board";
import { defaultBlueprint } from "../src/shared/project-blueprint";

test("execution freezes current blueprint, archives context and preserves it after project edits", async () => {
  const f = await fixture();
  const prefs = new Preferences(f.store, {
    encrypt: (s) => s,
    decrypt: (s) => s,
  });
  const settings = defaultSettings();
  settings.tools.codex = process.execPath;
  settings.local.code = {
    baseUrl: "http://127.0.0.1:9/v1",
    model: "fixture",
    route: "",
  };
  prefs.save(settings, {});
  const rpcs: FakeRpc[] = [];
  const tasks = new Tasks(
    f.store,
    f.files,
    new ProjectLocks(),
    prefs,
    path.resolve("resources"),
    path.resolve("dist/mcp.cjs"),
    () => {},
    () => {
      const rpc = new FakeRpc();
      rpcs.push(rpc);
      return rpc;
    },
  );
  try {
    await tasks.ready;
    f.project.blueprint = defaultBlueprint();
    f.project.blueprint.theme = { mode: "custom", value: "海底图书馆" };
    f.project.blueprintRevision = 4;
    f.store.put("project", f.project.id, f.project);
    const task = await tasks.create({
      projectId: f.project.id,
      prompt: "设计一位馆员",
    });
    await until(() => !!rpcs[0]?.turnInput);
    assert.match(JSON.stringify(rpcs[0]!.turnInput), /海底图书馆/);
    assert.match(
      await fs.readFile(
        path.join(task.workspace, ".beaver-context/project/brief.md"),
        "utf8",
      ),
      /规划版本 4/,
    );
    assert.equal(task.projectContext?.blueprint.audience, "all");
    f.project.blueprint.theme = { mode: "custom", value: "高山天文台" };
    f.project.blueprintRevision = 5;
    f.store.put("project", f.project.id, f.project);
    assert.equal(
      tasks.get(task.id).projectContext?.blueprint.theme.value,
      "海底图书馆",
    );
    const child = await tasks.delegate(task.id, "设计独立场景");
    assert.equal(child.projectContext?.revision, 5);
    assert.equal(child.projectContext?.blueprint.theme.value, "高山天文台");
    assert.ok(
      !Object.keys(await f.files.capture(task.workspace)).some((name) =>
        name.startsWith(".beaver-context"),
      ),
    );
  } finally {
    await tasks.shutdown();
    await f.cleanup();
  }
});

test("task attention labels distinguish decisions, acceptance and active execution", async () => {
  const f = await fixture();
  try {
    for (const [status, expected] of Object.entries({
      awaitingInput: "待回答",
      conflict: "待整合",
      failed: "待处理",
      interrupted: "待继续",
      completed: "待验收",
      running: "",
      queued: "",
      rolledBack: "",
    }))
      assert.equal(
        taskAttention({ ...f.task, status: status as Task["status"] }),
        expected,
      );
    assert.equal(
      taskAttention({ ...f.task, status: "completed", accepted: true }),
      "",
    );
  } finally {
    await f.cleanup();
  }
});

test("task resources include references and live changes but not unrelated baseline files", async () => {
  const f = await fixture();
  try {
    await fs.writeFile(path.join(f.project.path, "reference.md"), "reference");
    await fs.writeFile(path.join(f.project.path, "unchanged.md"), "unchanged");
    await fs.writeFile(path.join(f.project.path, "removed.md"), "removed");
    f.task.baseline = await f.files.capture(f.project.path);
    f.task.references = [{ path: "reference.md", note: "" }];
    await fs.unlink(path.join(f.project.path, "removed.md"));
    await fs.writeFile(path.join(f.project.path, "new.md"), "generated");
    const resources = await taskResources(f.task);
    assert.deepEqual(resources.map((item) => item.path).sort(), [
      "new.md",
      "reference.md",
      "removed.md",
    ]);
    assert.equal(
      resources.find((item) => item.path === "removed.md")?.exists,
      false,
    );
    assert.equal(await taskResourceText(f.task, "new.md"), "generated");
    await assert.rejects(taskResourceText(f.task, "../outside.md"));
  } finally {
    await f.cleanup();
  }
});

async function fixture() {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), "beaver-recovery-"));
  const store = new Store(path.join(root, "state"));
  const files = new Files(store.root);
  const project: Project = {
    id: "project",
    path: path.join(root, "project"),
    name: "test",
    createdAt: "",
  };
  await fs.mkdir(project.path);
  const task: Task = {
    id: "task",
    projectId: project.id,
    title: "test",
    prompt: "goal",
    stopConditions: "",
    status: "running",
    createdAt: "",
    updatedAt: "",
    workspace: project.path,
    references: [],
    baseline: {},
    changes: [],
    conflicts: [],
    maxMinutes: 0,
    capability: "code",
  };
  store.put("project", project.id, project);
  store.put("task", task.id, task);
  return {
    root,
    store,
    files,
    project,
    task,
    cleanup: async () => {
      store.close();
      await fs.rm(root, { recursive: true, force: true });
    },
  };
}

for (const kind of ["merge", "rollback"] as const)
  test(`${kind} crash journal completes a partially applied multi-file operation`, async () => {
    const f = await fixture();
    try {
      await fs.writeFile(path.join(f.project.path, "a"), "old-a");
      await fs.writeFile(path.join(f.project.path, "b"), "old-b");
      const before = await f.files.capture(f.project.path);
      await fs.writeFile(path.join(f.project.path, "a"), "new-a");
      await fs.writeFile(path.join(f.project.path, "b"), "new-b");
      const after = await f.files.capture(f.project.path);
      await f.files.restoreCopy(before, f.project.path);
      const changes = f.files.changes(before, after);
      f.task.status = kind === "merge" ? "completed" : "rolledBack";
      f.task.feature = { id: "feature", version: "1", snapshot: after };
      f.store.put("feature", "project:feature", { taskId: f.task.id });
      const op: FileOperation = {
        id: "op",
        taskId: f.task.id,
        projectId: f.project.id,
        changes,
        state: "applying",
        kind,
        taskAfter: f.task,
      };
      f.store.put("operation", "op", op);
      await f.files.apply(f.project.path, [changes[0]!]);
      await new Journal(f.store, f.files).recover();
      assert.deepEqual(await f.files.capture(f.project.path), after);
      assert.equal(f.store.get<Task>("task", "task")?.status, f.task.status);
      assert.equal(
        f.store.get<FileOperation>("operation", "op")?.state,
        "complete",
      );
      if (kind === "rollback")
        assert.equal(f.store.get("feature", "project:feature"), undefined);
      await new Journal(f.store, f.files).recover();
      assert.deepEqual(await f.files.capture(f.project.path), after);
    } finally {
      await f.cleanup();
    }
  });

test("crash recovery preserves external changes and compensates only known writes", async () => {
  const f = await fixture();
  try {
    await fs.writeFile(path.join(f.project.path, "a"), "old");
    await fs.writeFile(path.join(f.project.path, "b"), "old");
    const before = await f.files.capture(f.project.path);
    await fs.writeFile(path.join(f.project.path, "a"), "new");
    await fs.writeFile(path.join(f.project.path, "b"), "new");
    const after = await f.files.capture(f.project.path);
    const changes = f.files.changes(before, after);
    await fs.writeFile(path.join(f.project.path, "b"), "external-user-edit");
    f.store.put("operation", "op", {
      id: "op",
      taskId: f.task.id,
      projectId: f.project.id,
      changes,
      state: "applying",
      kind: "merge",
      taskAfter: { ...f.task, status: "completed" },
    });
    await new Journal(f.store, f.files).recover();
    assert.equal(
      await fs.readFile(path.join(f.project.path, "a"), "utf8"),
      "old",
    );
    assert.equal(
      await fs.readFile(path.join(f.project.path, "b"), "utf8"),
      "external-user-edit",
    );
    assert.equal(f.store.get<Task>("task", "task")?.status, "conflict");
    assert.deepEqual(f.store.get<Task>("task", "task")?.conflicts, ["b"]);
  } finally {
    await f.cleanup();
  }
});

class FakeRpc extends CodexRpc {
  override respond(_id: string | number, _result: unknown): void {}
  starts = 0;
  connected = false;
  closedCount = 0;
  turnInput: unknown;
  rejected: string[] = [];
  override rejectRequest(_id: string | number, message: string): void {
    this.rejected.push(message);
  }
  override async connect(): Promise<void> {
    this.connected = true;
  }
  override async request(method: string, params?: unknown): Promise<unknown> {
    if (method.startsWith("thread/")) return { thread: { id: "thread" } };
    if (method === "turn/start") {
      this.starts++;
      this.turnInput = params;
      return { turn: { id: "turn" } };
    }
    return {};
  }
  override async close(): Promise<void> {
    if (this.closedCount++ === 0) this.emit("exit");
  }
}
async function until(predicate: () => boolean) {
  const start = Date.now();
  while (!predicate()) {
    if (Date.now() - start > 30000) throw new Error("test condition timed out");
    await new Promise((resolve) => setTimeout(resolve, 10));
  }
}

test("structured plan creates child tasks and holds dependencies until manual approval", async () => {
  const f = await fixture();
  const prefs = new Preferences(f.store, {
    encrypt: (s) => s,
    decrypt: (s) => s,
  });
  const settings = defaultSettings();
  settings.tools.codex = process.execPath;
  settings.local.code = {
    baseUrl: "http://127.0.0.1:9/v1",
    model: "fixture",
    route: "",
  };
  prefs.save(settings, {});
  const rpcs: FakeRpc[] = [];
  const tasks = new Tasks(
    f.store,
    f.files,
    new ProjectLocks(),
    prefs,
    path.resolve("resources"),
    path.resolve("dist/mcp.cjs"),
    () => {},
    () => {
      const rpc = new FakeRpc();
      rpcs.push(rpc);
      return rpc;
    },
  );
  try {
    const parent = await tasks.create({
      projectId: f.project.id,
      prompt: "创作综合目标",
      decompose: true,
      autoAccept: false,
    });
    await until(() => !!rpcs[0]?.turnInput);
    rpcs[0]!.emit("serverRequest", 1, "item/tool/call", {
      tool: "beaver_submit_plan",
      threadId: "thread",
      turnId: "turn",
      arguments: {
        summary: "确认后分两步实现",
        steps: [
          {
            title: "前置内容",
            prompt: "写入前置内容",
            direction: "story",
            acceptance: "内容可读取",
          },
          {
            title: "验证内容",
            prompt: "检查前置结果",
            direction: "review",
            acceptance: "引用正确",
          },
        ],
      },
    });
    await until(() => !!rpcs[1]?.turnInput);
    const children = tasks.get(parent.id).subtaskIds!;
    assert.equal(children.length, 2);
    assert.equal(tasks.get(parent.id).status, "waitingChildren");
    assert.equal(tasks.get(children[1]!).workspacePrepared, false);
    const first = tasks.get(children[0]!);
    await fs.writeFile(
      path.join(first.workspace, "result.txt"),
      "approved context",
    );
    rpcs[1]!.emit("notification", "turn/completed", {
      turn: { id: "turn", status: "completed" },
    });
    await until(() => tasks.get(first.id).status === "completed");
    assert.equal(rpcs.length, 2);
    tasks.accept(first.id);
    await until(() => !!rpcs[2]?.turnInput);
    const second = tasks.get(children[1]!);
    assert.equal(
      await fs.readFile(path.join(second.workspace, "result.txt"), "utf8"),
      "approved context",
    );
    rpcs[2]!.emit("notification", "turn/completed", {
      turn: { id: "turn", status: "completed" },
    });
    await until(() => tasks.get(second.id).status === "completed");
    assert.equal(tasks.get(parent.id).status, "waitingChildren");
    tasks.accept(second.id);
    assert.equal(tasks.get(parent.id).status, "completed");
  } finally {
    await tasks.shutdown();
    await f.cleanup();
  }
});

test("terminal connection failure resumes once and preserves partial files", async () => {
  const f = await fixture();
  const prefs = new Preferences(f.store, {
    encrypt: (s) => s,
    decrypt: (s) => s,
  });
  const settings = defaultSettings();
  settings.tools.codex = process.execPath;
  settings.local.code = {
    baseUrl: "http://127.0.0.1:9/v1",
    model: "fixture",
    route: "",
  };
  prefs.save(settings, {});
  const rpc = new FakeRpc();
  const tasks = new Tasks(
    f.store,
    f.files,
    new ProjectLocks(),
    prefs,
    path.resolve("resources"),
    path.resolve("dist/mcp.cjs"),
    () => {},
    () => rpc,
  );
  try {
    await tasks.ready;
    const task = await tasks.create({
      projectId: f.project.id,
      prompt: "Recovery fixture",
    });
    await until(() => rpc.starts === 1);
    await fs.writeFile(path.join(task.workspace, "partial.md"), "preserve me");
    rpc.emit("notification", "turn/completed", {
      turn: {
        id: "turn",
        status: "failed",
        error: { message: "stream disconnected" },
      },
    });
    await until(() => rpc.starts === 2);
    assert.equal(
      await fs.readFile(path.join(task.workspace, "partial.md"), "utf8"),
      "preserve me",
    );
    rpc.emit("notification", "turn/completed", {
      turn: {
        id: "turn",
        status: "failed",
        error: { message: "stream disconnected" },
      },
    });
    await until(() => f.store.get<Task>("task", task.id)?.status === "failed");
    assert.equal(rpc.starts, 2);
    assert.equal(
      f.store.events(task.id).filter((e) => e.kind === "recovery").length,
      1,
    );
  } finally {
    await tasks.shutdown();
    await f.cleanup();
  }
});

test("AI question parks without merging, survives recovery, frees a slot and resumes once with answers", async () => {
  const f = await fixture();
  const prefs = new Preferences(f.store, {
    encrypt: (s) => s,
    decrypt: (s) => s,
  });
  const settings = defaultSettings();
  settings.tools.codex = process.execPath;
  settings.maxParallel = 1;
  settings.local.code = {
    baseUrl: "http://127.0.0.1:9/v1",
    model: "fixture",
    route: "",
  };
  prefs.save(settings, {});
  const rpcs: FakeRpc[] = [];
  const makeTasks = () =>
    new Tasks(
      f.store,
      f.files,
      new ProjectLocks(),
      prefs,
      path.resolve("resources"),
      path.resolve("dist/mcp.cjs"),
      () => {},
      () => {
        const rpc = new FakeRpc();
        rpcs.push(rpc);
        return rpc;
      },
    );
  let tasks = makeTasks();
  try {
    await tasks.ready;
    const task = await tasks.create({
      projectId: f.project.id,
      prompt: "设计角色",
    });
    await until(() => !!rpcs[0]?.turnInput);
    await fs.writeFile(path.join(task.workspace, "pending.md"), "not approved");
    rpcs[0]!.emit("serverRequest", 1, "item/tool/call", {
      threadId: "thread",
      turnId: "turn",
      tool: "beaver_ask_user",
      arguments: { questions: [{ id: "tone", question: "角色语气？" }] },
    });
    await until(
      () =>
        tasks.get(task.id).status === "awaitingInput" &&
        tasks.get(task.id).changes.length === 1,
    );
    await assert.rejects(fs.access(path.join(f.project.path, "pending.md")));
    const other = await tasks.create({
      projectId: f.project.id,
      prompt: "独立任务",
    });
    await until(() => !!rpcs[1]?.turnInput);
    assert.equal(tasks.get(other.id).status, "running");
    await tasks.shutdown();
    tasks = makeTasks();
    await tasks.ready;
    assert.equal(tasks.get(task.id).status, "awaitingInput");
    const question = tasks.get(task.id).clarifications![0]!;
    await assert.rejects(tasks.continue(task.id, "跳过"));
    await assert.rejects(tasks.answer(task.id, question.id, {}));
    const results = await Promise.allSettled([
      tasks.answer(task.id, question.id, { tone: "温柔" }),
      tasks.answer(task.id, question.id, { tone: "暴躁" }),
    ]);
    assert.equal(results.filter((r) => r.status === "fulfilled").length, 1);
    await until(() => !!rpcs[2]?.turnInput);
    assert.match(JSON.stringify(rpcs[2]!.turnInput), /温柔/);
    assert.equal(tasks.get(task.id).clarifications![0]!.answers?.tone, "温柔");
    await assert.rejects(
      tasks.answer(task.id, question.id, { tone: "重复提交" }),
    );
    assert.equal(tasks.get(task.id).status, "running");
    assert.equal(rpcs[2]!.closedCount, 0);
  } finally {
    await tasks.shutdown();
    await f.cleanup();
  }
});

test("queued amendments persist and delivered followups retain context with independent boundaries", async () => {
  const f = await fixture();
  const prefs = new Preferences(f.store, {
    encrypt: (s) => s,
    decrypt: (s) => s,
  });
  const settings = defaultSettings();
  settings.tools.codex = process.execPath;
  settings.local.code = {
    baseUrl: "http://127.0.0.1:9/v1",
    model: "fixture",
    route: "",
  };
  prefs.save(settings, {});
  const tasks = new Tasks(
    f.store,
    f.files,
    new ProjectLocks(),
    prefs,
    path.resolve("resources"),
    path.resolve("dist/mcp.cjs"),
    () => {},
    () => new FakeRpc(),
  );
  try {
    await tasks.ready;
    f.task.status = "queued";
    f.task.prompt = "原目标";
    f.task.report = "原汇报";
    f.store.put("task", f.task.id, f.task);
    await tasks.continue(f.task.id, "保留人物");
    assert.equal(tasks.get(f.task.id).status, "queued");
    assert.match(tasks.get(f.task.id).prompt, /保留人物/);
    await assert.rejects(tasks.followup(f.task.id, "过早创建后续任务"));
    const source = tasks.get(f.task.id);
    source.status = "completed";
    f.store.put("task", source.id, source);
    const next = await tasks.followup(source.id, "继续增加角色");
    assert.notEqual(next.id, source.id);
    assert.equal(next.parentTaskId, source.id);
    assert.equal(tasks.get(source.id).status, "completed");
    const context = JSON.parse(
      await fs.readFile(
        path.join(next.workspace, ".beaver-context/followup/task.json"),
        "utf8",
      ),
    );
    assert.match(context.prompt, /原目标[\s\S]*保留人物/);
    assert.equal(context.report, "原汇报");
    assert.match(next.prompt, /继续增加角色/);
  } finally {
    await tasks.shutdown();
    await f.cleanup();
  }
});

test("delegation keeps waiting parent unchanged and starts from integrated project only", async () => {
  const f = await fixture();
  const prefs = new Preferences(f.store, {
    encrypt: (s) => s,
    decrypt: (s) => s,
  });
  const settings = defaultSettings();
  settings.tools.codex = process.execPath;
  settings.local.code = {
    baseUrl: "http://127.0.0.1:9/v1",
    model: "fixture",
    route: "",
  };
  prefs.save(settings, {});
  const tasks = new Tasks(
    f.store,
    f.files,
    new ProjectLocks(),
    prefs,
    path.resolve("resources"),
    path.resolve("dist/mcp.cjs"),
    () => {},
    () => new FakeRpc(),
  );
  try {
    await tasks.ready;
    f.task.status = "awaitingInput";
    f.task.workspace = path.join(f.root, "parent-workspace");
    await fs.mkdir(f.task.workspace);
    await fs.writeFile(
      path.join(f.task.workspace, "unfinished.md"),
      "not integrated",
    );
    await fs.writeFile(path.join(f.project.path, "integrated.md"), "current");
    f.task.references = [{ path: "integrated.md", note: "必须保持一致" }];
    f.store.put("task", f.task.id, f.task);
    f.store.event(f.task.id, "user", "保留原角色");
    const child = await tasks.delegate(f.task.id, "增加一个独立角色");
    assert.equal(child.parentTaskId, f.task.id);
    assert.equal(child.relation, "child");
    assert.deepEqual(child.references, f.task.references);
    assert.equal(tasks.get(f.task.id).status, "awaitingInput");
    assert.notEqual(child.workspace, f.task.workspace);
    assert.equal(
      await fs.readFile(path.join(child.workspace, "integrated.md"), "utf8"),
      "current",
    );
    await assert.rejects(
      fs.access(path.join(child.workspace, "unfinished.md")),
    );
    const context = JSON.parse(
      await fs.readFile(
        path.join(child.workspace, ".beaver-context/parent/task.json"),
        "utf8",
      ),
    );
    assert.equal(context.id, f.task.id);
    assert.equal(context.conversation[0].text, "保留原角色");
  } finally {
    await tasks.shutdown();
    await f.cleanup();
  }
});

test("document saves reject stale drafts and support independent rollback", async () => {
  const f = await fixture();
  const prefs = new Preferences(f.store, {
    encrypt: (s) => s,
    decrypt: (s) => s,
  });
  const tasks = new Tasks(
    f.store,
    f.files,
    new ProjectLocks(),
    prefs,
    path.resolve("resources"),
    "",
    () => {},
  );
  try {
    await tasks.ready;
    await tasks.saveDocument(
      f.project.id,
      "docs/world.md",
      "# 世界观\n原创",
      null,
    );
    const first = await readDocument(f.project.path, "docs/world.md");
    await tasks.saveDocument(
      f.project.id,
      "docs/world.md",
      "人工调整",
      first.revision,
    );
    await assert.rejects(
      tasks.saveDocument(
        f.project.id,
        "docs/world.md",
        "过期覆盖",
        first.revision,
      ),
      /资料已被/,
    );
    assert.equal(
      (await readDocument(f.project.path, "docs/world.md")).text,
      "人工调整",
    );
    const latest = f.store.list<Task>("task")[0]!;
    await tasks.rollback(latest.id, []);
    assert.equal(
      (await readDocument(f.project.path, "docs/world.md")).text,
      first.text,
    );
    for (const relative of [
      "../outside.md",
      ".beaver/state.md",
      "release/x.md",
      "script.gd",
    ])
      await assert.rejects(
        tasks.saveDocument(f.project.id, relative, "bad", null),
      );
  } finally {
    await tasks.shutdown();
    await f.cleanup();
  }
});
for (const duringStartup of [true, false])
  test(`interrupt ${duringStartup ? "before connection" : "without terminal event"} waits for persistence and is resumable`, async () => {
    const f = await fixture();
    try {
      const prefs = new Preferences(f.store, {
        encrypt: (s) => s,
        decrypt: (s) => s,
      });
      const settings = defaultSettings();
      settings.tools.codex = process.execPath;
      settings.local.code = {
        baseUrl: "http://127.0.0.1:9/v1",
        model: "test-fixture",
        route: "",
      };
      prefs.save(settings, {});
      const rpc = new FakeRpc();
      f.task.design = {
        genres: ["narrative", "management"],
        theme: "cyberpunk",
        style: "pixel",
        scope: "prototype",
      };
      const statuses: string[] = [];
      const executor = new LocalExecutor(
        f.store.root,
        path.resolve("resources"),
        path.resolve("dist/mcp.cjs"),
        prefs,
        () => {},
        (t) => f.store.put("task", t.id, t),
        async (_t, status) => {
          await new Promise((r) => setTimeout(r, 20));
          statuses.push(status);
        },
        () => rpc,
      );
      const start = executor.start(f.task);
      if (!duringStartup) {
        await start;
        assert.match(JSON.stringify(rpc.turnInput), /叙事.*经营管理/);
        assert.match(JSON.stringify(rpc.turnInput), /不是已实现功能/);
        await executor.steer(f.task.id, "keep my change");
        assert.match(
          f.store.get<Task>("task", f.task.id)!.prompt,
          /keep my change/,
        );
      }
      await executor.shutdown();
      await start;
      assert.deepEqual(statuses, ["interrupted"]);
      assert.equal(rpc.connected, !duringStartup);
      await executor.shutdown();
      assert.equal(statuses.length, 1);
    } finally {
      await f.cleanup();
    }
  });

test("export target follows preset rather than host operating system", () => {
  const config =
    '[preset.0]\nname="Windows"\nplatform="Windows Desktop"\n[preset.0.options]\nfoo=true\n[preset.1]\nname="Penguin"\nplatform="Linux"\n';
  assert.equal(exportTarget(config, "Windows").entry, "Game.exe");
  assert.equal(exportTarget(config, "Penguin").entry, "Game.x86_64");
  assert.throws(() => exportTarget(config, "missing"));
});
test("command timeout terminates its owned child before returning", async () => {
  await assert.rejects(
    runCommand(
      process.execPath,
      ["-e", "setInterval(() => {}, 1000)"],
      undefined,
      150,
    ),
    /超时/,
  );
});

test("feature acceptance cannot regress a newer adoption and full rollback restores prior baseline", async () => {
  const f = await fixture();
  const prefs = new Preferences(f.store, {
    encrypt: (s) => s,
    decrypt: (s) => s,
  });
  const tasks = new Tasks(
    f.store,
    f.files,
    new ProjectLocks(),
    prefs,
    path.resolve("resources"),
    path.resolve("dist/mcp.cjs"),
    () => {},
  );
  try {
    await tasks.ready;
    f.task.status = "completed";
    f.task.feature = { id: "save", version: "1", snapshot: {} };
    f.store.put("task", f.task.id, f.task);
    tasks.accept(f.task.id);
    const previous = {
      id: "save",
      version: "1",
      snapshot: {},
      taskId: f.task.id,
    };
    const updated: Task = {
      ...f.task,
      id: "task-2",
      accepted: false,
      feature: { id: "save", version: "2", snapshot: {}, previous },
    };
    f.store.put("task", updated.id, updated);
    tasks.accept(updated.id);
    tasks.accept(f.task.id);
    assert.equal(
      f.store.get<{ version: string }>("feature", "project:save")?.version,
      "2",
    );
    const stale: Task = { ...f.task, id: "stale", accepted: false };
    f.store.put("task", stale.id, stale);
    assert.throws(() => tasks.accept(stale.id), /另一任务/);
    await tasks.rollback(updated.id, []);
    assert.deepEqual(f.store.get("feature", "project:save"), previous);
  } finally {
    await tasks.shutdown();
    await f.cleanup();
  }
});

test("unavailable project recovery retains journal and does not prevent other projects from opening", async () => {
  const f = await fixture();
  try {
    await fs.rmdir(f.project.path);
    f.store.put("operation", "op", {
      id: "op",
      taskId: f.task.id,
      projectId: f.project.id,
      kind: "merge",
      state: "applying",
      changes: [{ path: "a", after: "0".repeat(64) }],
      taskAfter: { ...f.task, status: "completed" },
    });
    await new Journal(f.store, f.files).recover();
    assert.equal(
      f.store.get<FileOperation>("operation", "op")?.state,
      "applying",
    );
    assert.equal(f.store.get<Task>("task", f.task.id)?.status, "conflict");
    assert.match(f.store.get<Task>("task", f.task.id)!.error!, /恢复尚未完成/);
  } finally {
    await f.cleanup();
  }
});
