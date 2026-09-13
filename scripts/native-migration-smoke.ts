import assert from "node:assert/strict";
import fs from "node:fs/promises";
import path from "node:path";
import { spawnSync, type SpawnSyncReturns } from "node:child_process";
import { Store } from "../src/core/store";
import { inventory } from "./native-release";

const projectIds = [
  "12345678-1234-4234-8234-123456789001",
  "12345678-1234-4234-8234-123456789002",
];
async function main() {
  if (process.argv[2] === "--fixture") {
    const root = path.resolve(process.argv[3]!);
    assert.equal(
      await fs.readFile(path.join(root, "fixture.txt"), "utf8"),
      "isolated migration fixture",
    );
    const store = new Store(path.join(root, "source"));
    projectIds.forEach((id, index) =>
      store.put("project", id, {
        id,
        path: path.join(root, `game-${index}`),
        unknown: { keep: "unchanged" },
      }),
    );
    store.put("secret", "code", "opaque-original-cipher");
    store.put("task", "fixture", {
      id: "fixture",
      projectId: projectIds[0],
      status: "running",
      workspace: path.join(root, "source/workspaces/fixture"),
      threadId: "old-thread",
      turnId: "old-turn",
    });
    store.event("fixture", "report", "committed WAL event");
    process.exit(0);
  }
  const executable = path.resolve(process.argv[2] ?? "target/debug/Beaver.exe");
  const godot = process.argv[3] ? path.resolve(process.argv[3]) : undefined;
  const root = path.resolve(
    "output/validation",
    `native-migration-${Date.now()}`,
  );
  await fs.mkdir(root, { recursive: true });
  const source = path.join(root, "source");
  await fs.mkdir(
    path.join(source, "workspaces/fixture/.beaver-context/empty"),
    { recursive: true },
  );
  await fs.mkdir(path.join(source, "codex/fixture/sessions"), {
    recursive: true,
  });
  await fs.writeFile(
    path.join(source, "codex/fixture/sessions/old.jsonl"),
    "opaque session with old paths\n",
  );
  await fs.writeFile(path.join(source, "Local State"), '{"fixture":true}');
  await fs.writeFile(
    path.join(root, "fixture.txt"),
    "isolated migration fixture",
  );
  for (let index = 0; index < 2; index++) {
    const game = path.join(root, `game-${index}`);
    await fs.mkdir(path.join(game, ".git/empty"), { recursive: true });
    await fs.writeFile(
      path.join(game, "project.godot"),
      'config_version=5\n[application]\nconfig/name="Migration fixture"\n',
    );
    await fs.writeFile(
      path.join(game, ".git/config"),
      "original repository config",
    );
    await fs.writeFile(
      path.join(game, "中文.bin"),
      Buffer.from([0, 255, index]),
    );
    await fs.writeFile(
      path.join(game, "verify.gd"),
      'extends SceneTree\nfunc _initialize():\n\tprint("RESTORED_PROJECT_OK")\n\tquit()\n',
    );
  }
  const fixture = spawnSync(
    process.execPath,
    ["--import", "tsx", __filename, "--fixture", root],
    { encoding: "utf8", timeout: 30_000 },
  );
  assert.equal(fixture.status, 0, fixture.stderr);
  assert.ok((await fs.stat(path.join(source, "beaver.sqlite-wal"))).size > 0);
  const originalData = await inventory(source);
  const originalProjects = await Promise.all(
    [0, 1].map((index) => inventory(path.join(root, `game-${index}`))),
  );
  const env = {
    ...process.env,
    BEAVER_DATA_DIR: path.join(root, "unexpected-app-data"),
    WEBVIEW2_USER_DATA_FOLDER: path.join(root, "unexpected-webview"),
  };
  function run(args: string[], expected = 0) {
    const result = spawnSync(executable, ["--migration-bundle", ...args], {
      encoding: "utf8",
      env,
      windowsHide: true,
      timeout: 60_000,
    });
    assert.equal(result.status, expected, result.stderr);
    return result;
  }
  const backup = path.join(root, "bundle");
  run(["create", source, backup]);
  run(["verify", backup]);
  assert.deepEqual(await inventory(source), originalData);
  for (let index = 0; index < 2; index++)
    assert.deepEqual(
      await inventory(path.join(root, `game-${index}`)),
      originalProjects[index],
    );
  const archiveBefore = await inventory(backup);
  // Make only this test's original directories unavailable before recovery.
  for (const name of ["source", "game-0", "game-1"]) {
    const from = path.resolve(root, name),
      to = path.resolve(root, `${name}-unavailable`);
    assert.ok(
      from.startsWith(root + path.sep) && to.startsWith(root + path.sep),
    );
    await fs.rename(from, to);
  }
  run(["verify", backup]);
  const restored = path.join(root, "restored");
  const receipt = JSON.parse(run(["restore", backup, restored]).stdout) as {
    ready_to_activate: boolean;
    paths_rewritten: boolean;
    credentials_converted: boolean;
    projects: { id: string; original_path: string; restored_path: string }[];
  };
  assert.equal(receipt.ready_to_activate, false);
  assert.equal(receipt.paths_rewritten, false);
  assert.equal(receipt.credentials_converted, false);
  assert.deepEqual(await inventory(path.join(restored, "data")), originalData);
  for (let index = 0; index < 2; index++)
    assert.deepEqual(
      await inventory(path.join(restored, "projects", projectIds[index]!)),
      originalProjects[index],
    );
  assert.deepEqual(await inventory(backup), archiveBefore);
  await assert.rejects(fs.access(path.join(root, "unexpected-webview")));
  const blocked = spawnSync(executable, [], {
    encoding: "utf8",
    env: { ...env, BEAVER_DATA_DIR: path.join(restored, "data") },
    windowsHide: true,
    timeout: 30_000,
  });
  assert.notEqual(blocked.status, null, blocked.error?.message);
  assert.notEqual(blocked.status, 0);
  assert.ok(blocked.stderr.includes("recovery copy is not activated"));
  await assert.rejects(
    fs.access(path.join(restored, "data/.beaver-native.lock")),
  );
  assert.deepEqual(await inventory(path.join(restored, "data")), originalData);
  const old = new Store(path.join(restored, "data"));
  assert.equal(old.get<string>("secret", "code"), "opaque-original-cipher");
  assert.equal(
    old.get<{ workspace: string }>("task", "fixture")?.workspace,
    path.join(source, "workspaces/fixture"),
  );
  assert.equal(
    old.get<{ threadId: string }>("task", "fixture")?.threadId,
    "old-thread",
  );
  assert.ok(
    old.events("fixture").some((event) => event.text === "committed WAL event"),
  );
  old.close();
  const gameChecks: { id: string; exitCode: number | null }[] = [];
  if (godot) {
    for (const project of receipt.projects) {
      const result: SpawnSyncReturns<string> = spawnSync(
        godot,
        [
          "--headless",
          "--path",
          project.restored_path,
          "--script",
          "res://verify.gd",
        ],
        { encoding: "utf8", windowsHide: true, timeout: 60_000 },
      );
      assert.equal(result.status, 0, result.stderr);
      assert.ok(result.stdout.includes("RESTORED_PROJECT_OK"));
      assert.ok(
        !/SCRIPT ERROR|Parse Error/.test(result.stdout + result.stderr),
      );
      gameChecks.push({ id: project.id, exitCode: result.status });
    }
  }
  run(["restore", backup, restored], 1);
  await fs.writeFile(
    path.join(backup, "projects", projectIds[0]!, "project.godot"),
    "tampered",
  );
  run(["verify", backup], 1);
  run(["restore", backup, path.join(root, "rejected")], 1);
  await assert.rejects(fs.access(path.join(root, "rejected")));
  await assert.rejects(fs.access(path.join(root, "unexpected-app-data")));
  assert.deepEqual(
    await inventory(path.join(root, "source-unavailable")),
    originalData,
  );
  for (let index = 0; index < 2; index++)
    assert.deepEqual(
      await inventory(path.join(root, `game-${index}-unavailable`)),
      originalProjects[index],
    );
  const proof = {
    executable,
    checks: [
      "headless production command does not initialize desktop or user database",
      "actual legacy Store committed WAL included",
      "both registered external projects included with repository files and Unicode assets",
      "backup and recovery never modify source bytes",
      "archive verifies and restores while original directories are unavailable",
      "restored application and projects match exact archived bytes",
      "legacy Store reads restored WAL credentials and thread IDs without path rewriting",
      "desktop rejects pending recovery before instance lock or database writes",
      "existing destination and tampered archive rejected",
    ],
    gameChecks,
    readyToActivate: false,
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
