import assert from "node:assert/strict";
import fs from "node:fs/promises";
import path from "node:path";
import { createHash } from "node:crypto";
import { spawnSync } from "node:child_process";
import { Store } from "../src/core/store";
import { inventory } from "./native-release";

const project = "12345678-1234-4234-8234-123456789011";
const tasks = [
  "12345678-1234-4234-8234-123456789012",
  "12345678-1234-4234-8234-123456789013",
];
const hash = (text: string) => createHash("sha256").update(text).digest("hex");

async function main() {
  if (process.argv[2] === "--fixture") {
    const root = path.resolve(process.argv[3]!);
    const fixture: unknown = JSON.parse(
      await fs.readFile(path.join(root, "fixture.json"), "utf8"),
    );
    assert.ok(
      typeof fixture === "object" &&
        fixture !== null &&
        "fixture" in fixture &&
        fixture.fixture === true,
    );
    assert.ok("cipher" in fixture && typeof fixture.cipher === "string");
    const store = new Store(path.join(root, "source"));
    store.put("project", project, {
      id: project,
      path: path.join(root, "game"),
      unknown: "kept",
    });
    for (const id of tasks) {
      store.put("task", id, {
        id,
        projectId: project,
        workspace: path.join(root, "source/workspaces", id),
        status: id === tasks[0] ? "running" : "queued",
        threadId: `old-${id}`,
        turnId: "old-turn",
        unknown: "kept",
      });
    }
    store.put("operation", "fixture-operation", {
      id: "fixture-operation",
      projectId: project,
      taskId: tasks[0],
      kind: "merge",
      state: "applying",
      changes: [
        { path: "a.txt", before: hash("before"), after: hash("after") },
      ],
      taskAfter: {
        ...store.get<Record<string, unknown>>("task", tasks[0]!),
        status: "completed",
      },
    });
    store.put("secret", "code", fixture.cipher);
    store.event(tasks[0]!, "report", "legacy WAL event");
    process.exit(0); // Preserve a committed legacy WAL for the production importer.
  }
  const executable = path.resolve(process.argv[2] ?? "target/debug/Beaver.exe");
  assert.ok(
    process.argv[3],
    "explicit marked Electron credential fixture directory required",
  );
  const legacyRoot = path.resolve(process.argv[3]!);
  const legacy: unknown = JSON.parse(
    await fs.readFile(path.join(legacyRoot, "legacy.json"), "utf8"),
  );
  assert.ok(
    typeof legacy === "object" &&
      legacy !== null &&
      "fixture" in legacy &&
      legacy.fixture === true,
  );
  assert.ok("cipher" in legacy && typeof legacy.cipher === "string");
  const legacyState = await fs.readFile(
    path.join(legacyRoot, "electron-profile/Local State"),
  );
  const root = path.resolve("output/validation", `native-import-${Date.now()}`);
  const source = path.join(root, "source"),
    game = path.join(root, "game");
  await fs.mkdir(path.join(source, "blobs"), { recursive: true });
  await fs.mkdir(game);
  await fs.writeFile(path.join(root, "fixture.json"), JSON.stringify(legacy));
  await fs.writeFile(path.join(source, "Local State"), legacyState);
  for (const text of ["before", "after"])
    await fs.writeFile(path.join(source, "blobs", hash(text)), text);
  await fs.writeFile(
    path.join(game, "project.godot"),
    'config_version=5\n[application]\nconfig/name="Prepared import fixture"\n',
  );
  await fs.writeFile(path.join(game, "a.txt"), "before");
  for (const id of tasks) {
    await fs.mkdir(path.join(source, "workspaces", id), { recursive: true });
    await fs.writeFile(
      path.join(source, "workspaces", id, "project.godot"),
      "config_version=5\n",
    );
    await fs.mkdir(path.join(source, "codex", id, "sessions"), {
      recursive: true,
    });
    await fs.writeFile(
      path.join(source, "codex", id, "sessions/old.jsonl"),
      `opaque session old cwd ${source}\n`,
    );
  }
  const fixture = spawnSync(
    "rtk",
    [
      "proxy",
      process.execPath,
      "--import",
      "tsx",
      __filename,
      "--fixture",
      root,
    ],
    { encoding: "utf8", windowsHide: true, timeout: 30_000 },
  );
  assert.equal(fixture.status, 0, fixture.stderr);
  assert.ok((await fs.stat(path.join(source, "beaver.sqlite-wal"))).size > 0);
  const before = await inventory(source),
    gameBefore = await inventory(game);
  const env = {
    ...process.env,
    BEAVER_DATA_DIR: path.join(root, "unexpected-data"),
    WEBVIEW2_USER_DATA_FOLDER: path.join(root, "unexpected-webview"),
  };
  function run(args: string[], expected = 0) {
    const result = spawnSync(
      "rtk",
      ["proxy", executable, "--migration-bundle", ...args],
      { encoding: "utf8", windowsHide: true, env, timeout: 60_000 },
    );
    assert.equal(result.status, expected, result.stderr);
    return result;
  }
  const backup = path.join(root, "backup"),
    target = path.join(root, "prepared");
  run(["create", source, backup]);
  const archiveBefore = await inventory(backup);
  for (const from of [source, game]) {
    const to = `${from}-unavailable`;
    assert.ok(
      path.resolve(from).startsWith(root + path.sep) &&
        path.resolve(to).startsWith(root + path.sep),
    );
    await fs.rename(from, to);
  }
  const report = JSON.parse(run(["prepare-import", backup, target]).stdout) as {
    restored_data: string;
    paths_rewritten: boolean;
    credentials_converted: number;
    credentials_verified: boolean;
    journals_recovered: number;
    tasks_interrupted: number;
    ready_to_activate: boolean;
  };
  assert.equal(report.paths_rewritten, true);
  assert.equal(report.credentials_verified, true);
  assert.equal(report.credentials_converted, 1);
  assert.equal(report.journals_recovered, 1);
  assert.equal(report.tasks_interrupted, 1);
  assert.equal(report.ready_to_activate, false);
  const store = new Store(report.restored_data);
  const migrated = store.get<{ path: string; unknown: string }>(
    "project",
    project,
  )!;
  assert.equal(migrated.unknown, "kept");
  assert.equal(
    await fs.readFile(path.join(migrated.path, "a.txt"), "utf8"),
    "after",
  );
  for (const [index, id] of tasks.entries()) {
    const task = store.get<{
      workspace: string;
      status: string;
      threadId: string;
      turnId: string;
      unknown: string;
    }>("task", id)!;
    assert.equal(
      task.workspace.toLowerCase(),
      path.join(report.restored_data, "workspaces", id).toLowerCase(),
    );
    assert.equal(task.status, index === 0 ? "completed" : "interrupted");
    assert.equal(task.threadId, `old-${id}`);
    assert.equal(task.turnId, "old-turn");
    assert.equal(task.unknown, "kept");
    assert.equal(
      await fs.readFile(
        path.join(report.restored_data, "codex", id, "sessions/old.jsonl"),
        "utf8",
      ),
      `opaque session old cwd ${source}\n`,
    );
  }
  assert.equal(
    store.get<{ taskAfter: { workspace: string } }>(
      "operation",
      "fixture-operation",
    )?.taskAfter.workspace,
    store.get<{ workspace: string }>("task", tasks[0]!)?.workspace,
  );
  assert.ok(
    store.get<string>("secret", "code")?.startsWith("beaver-dpapi-v1:"),
  );
  assert.equal(
    store.list<{ cipher: string }>("secret_backup")[0]?.cipher,
    legacy.cipher,
  );
  assert.ok(
    store.events(tasks[0]!).some((event) => event.text === "legacy WAL event"),
  );
  store.close();
  for (const name of ["unexpected-data", "unexpected-webview"])
    await assert.rejects(fs.access(path.join(root, name)));
  const importedBeforeLaunch = await inventory(report.restored_data);
  const blocked = spawnSync("rtk", ["proxy", executable], {
    encoding: "utf8",
    windowsHide: true,
    env: { ...env, BEAVER_DATA_DIR: report.restored_data },
    timeout: 30_000,
  });
  assert.notEqual(blocked.status, null, blocked.error?.message);
  assert.notEqual(blocked.status, 0);
  assert.ok(blocked.stderr.includes("recovery copy is not activated"));
  assert.deepEqual(await inventory(report.restored_data), importedBeforeLaunch);
  run(["prepare-import", backup, target], 1);
  run(["verify", backup]);
  assert.deepEqual(await inventory(backup), archiveBefore);
  assert.deepEqual(await inventory(`${source}-unavailable`), before);
  assert.deepEqual(await inventory(`${game}-unavailable`), gameBefore);
  assert.deepEqual(
    await fs.readFile(path.join(legacyRoot, "electron-profile/Local State")),
    legacyState,
  );
  // Tauri may initialize WebView runtime metadata before its setup guard runs.
  await assert.rejects(fs.access(path.join(root, "unexpected-data")));
  const proof = {
    executable,
    checks: [
      "actual single EXE prepares import without desktop initialization",
      "legacy Store WAL and real Electron safeStorage credential converted",
      "source data and project unavailable during import and unchanged afterward",
      "journal writes only relocated project and retains relocated taskAfter workspace",
      "journal commit precedes running/queued task interruption",
      "original ciphertext retained; threadId turnId opaque sessions and unknown task fields preserved",
      "prepared copy remains blocked at desktop entry without writes",
      "archive unchanged and existing target refused",
    ],
    readyToActivate: false,
    realCodexResumeVerified: false,
    userDataTouched: false,
  };
  await fs.writeFile(
    path.join(root, "proof.json"),
    JSON.stringify(proof, null, 2),
  );
  console.log(JSON.stringify({ root, ...proof }, null, 2));
}
main().catch((error: unknown) => {
  console.error(error);
  process.exitCode = 1;
});
