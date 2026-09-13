import assert from "node:assert/strict";
import fs from "node:fs/promises";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { Store } from "../src/core/store";
import { inventory } from "./native-release";

async function main() {
  if (process.argv[2] === "--fixture") {
    const source = path.resolve(process.argv[3]!);
    assert.equal(
      await fs.readFile(path.join(source, "backup-fixture.txt"), "utf8"),
      "isolated test only",
    );
    const store = new Store(source);
    store.put("project", "fixture", {
      id: "fixture",
      path: "external-project-not-copied",
      unknown: "preserved",
    });
    store.put("secret", "code", "opaque-legacy-cipher");
    store.put("task", "fixture", {
      id: "fixture",
      status: "running",
      workspace: path.join(source, "workspaces/fixture"),
      threadId: "old-thread",
      unknown: [1, 2, 3],
    });
    store.event("fixture", "report", "persisted in WAL");
    // Immediate process exit deliberately leaves committed WAL without Store.close().
    process.exit(0);
  }
  const output = path.resolve(
    "output/validation",
    `native-backup-${Date.now()}`,
  );
  const source = path.join(output, "source");
  const backup = path.join(output, "backup");
  const restored = path.join(output, "restored");
  await fs.mkdir(
    path.join(source, "workspaces/fixture/.beaver-context/empty"),
    { recursive: true },
  );
  await fs.mkdir(path.join(source, "blobs"));
  await fs.writeFile(
    path.join(source, "backup-fixture.txt"),
    "isolated test only",
  );
  await fs.writeFile(
    path.join(source, "Local State"),
    '{"fixture":true,"opaque":"preserved"}',
  );
  await fs.writeFile(
    path.join(source, "blobs/blob-fixture"),
    Buffer.from([0, 255, 127]),
  );
  await fs.writeFile(
    path.join(source, "workspaces/fixture/中文.txt"),
    "original workspace",
  );
  const child = spawnSync(
    process.execPath,
    ["--import", "tsx", __filename, "--fixture", source],
    { encoding: "utf8", timeout: 30_000 },
  );
  assert.equal(child.status, 0, child.stderr);
  assert.ok((await fs.stat(path.join(source, "beaver.sqlite-wal"))).size > 0);
  const before = await inventory(source);
  const run = (args: string[], expected = 0) => {
    const result = spawnSync(
      "rtk",
      [
        "proxy",
        "cargo",
        "run",
        "--locked",
        "-p",
        "beaver-core",
        "--example",
        "data_backup",
        "--",
        ...args,
      ],
      { encoding: "utf8", timeout: 120_000 },
    );
    assert.equal(result.status, expected, result.stderr);
  };
  run(["create", source, backup]);
  run(["verify", backup]);
  run(["restore", backup, restored]);
  assert.deepEqual(await inventory(restored), before);
  assert.deepEqual(await inventory(source), before);
  const old = new Store(restored);
  assert.equal(
    old.get<{ unknown: string }>("project", "fixture")?.unknown,
    "preserved",
  );
  assert.equal(old.get<string>("secret", "code"), "opaque-legacy-cipher");
  assert.equal(
    old.get<{ status: string; threadId: string }>("task", "fixture")?.status,
    "running",
  );
  assert.equal(
    old.get<{ threadId: string }>("task", "fixture")?.threadId,
    "old-thread",
  );
  assert.ok(
    old.events("fixture").some((event) => event.text === "persisted in WAL"),
  );
  old.close();
  run(["restore", backup, restored], 1);
  await fs.writeFile(path.join(backup, "data/Local State"), "tampered");
  run(["verify", backup], 1);
  run(["restore", backup, path.join(output, "rejected")], 1);
  assert.deepEqual(await inventory(source), before);
  await assert.rejects(fs.access(path.join(output, "rejected")));
  const proof = {
    checks: [
      "actual TypeScript Store leaves committed WAL",
      "Rust offline backup preserves all source bytes",
      "new-directory restore matches database WAL workspace blobs and profile",
      "old TypeScript Store reads restored committed WAL and unknown fields",
      "opaque credentials and task state preserved without conversion or recovery",
      "existing restore target rejected",
      "tampered backup rejected before destination creation",
    ],
    sourceUnchanged: true,
    externalProjectsIncluded: false,
    pathMigrationImplemented: false,
    credentialConversionExecuted: false,
  };
  await fs.writeFile(
    path.join(output, "proof.json"),
    JSON.stringify(proof, null, 2),
  );
  console.log(JSON.stringify({ output, ...proof }, null, 2));
}
main().catch((error: unknown) => {
  console.error(error);
  process.exitCode = 1;
});
