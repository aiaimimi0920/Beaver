import fs from "node:fs/promises";
import path from "node:path";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { Store } from "../src/core/store";
import { Files } from "../src/core/files";

async function main() {
  const output = path.resolve(
    "output/validation",
    `native-store-${Date.now()}`,
  );
  await fs.mkdir(output, { recursive: true });
  await fs.writeFile(
    path.join(output, "migration-fixture.json"),
    '{"generated":true}\n',
  );
  const project = { name: "迁移验证", future: { preserve: true } };
  const legacy = new Store(output);
  try {
    legacy.put("project", "fixture", project);
    legacy.put("task", "fixture", {
      status: "running",
      threadId: "preserve-thread",
      references: [{ path: "世界观.md" }],
    });
    legacy.put("task", "waiting", { status: "awaitingInput" });
    legacy.put("secret", "fixture", "NOT-A-REAL-SECRET");
    legacy.event("fixture", "legacy", "TypeScript 事件");
  } finally {
    legacy.close();
  }
  const result = execFileSync(
    "rtk",
    [
      "cargo",
      "run",
      "--locked",
      "-p",
      "beaver-core",
      "--example",
      "store_contract",
      "--",
      output,
    ],
    {
      encoding: "utf8",
      timeout: 120000,
      windowsHide: true,
      stdio: ["ignore", "pipe", "pipe"],
    },
  );
  assert.match(result, /NATIVE_STORE_CONTRACT_OK/);
  const reopened = new Store(output);
  try {
    assert.deepEqual(reopened.get("project", "fixture"), project);
    assert.equal(reopened.get("secret", "fixture"), "NOT-A-REAL-SECRET");
    assert.deepEqual(reopened.get("nativeProof", "fixture"), {
      from: "rust",
      unicode: "原样保留",
    });
    const task = reopened.get<{ status: string; threadId: string }>(
      "task",
      "fixture",
    )!;
    assert.equal(task.status, "interrupted");
    assert.equal(task.threadId, "preserve-thread");
    assert.equal(
      reopened.get<{ status: string }>("task", "waiting")!.status,
      "awaitingInput",
    );
    assert.deepEqual(
      reopened.events("fixture").map((event) => event.kind),
      ["legacy", "native"],
    );
  } finally {
    reopened.close();
  }
  const projectRoot = path.join(output, "project");
  await fs.mkdir(projectRoot);
  await fs.writeFile(path.join(projectRoot, "a.txt"), "before");
  await fs.writeFile(path.join(projectRoot, "世界观.md"), "旧世界观");
  const files = new Files(output);
  const before = await files.capture(projectRoot);
  await fs.writeFile(path.join(projectRoot, "a.txt"), "after");
  await fs.writeFile(path.join(projectRoot, "世界观.md"), "新世界观");
  const after = await files.capture(projectRoot);
  await files.restoreCopy(before, projectRoot);
  const changes = files.changes(before, after);
  const legacyFiles = new Store(output);
  try {
    legacyFiles.put("project", "file-project", {
      id: "file-project",
      path: projectRoot,
    });
    legacyFiles.put("task", "file-task", {
      id: "file-task",
      status: "running",
      direction: "story",
    });
    legacyFiles.put("operation", "file-operation", {
      id: "file-operation",
      projectId: "file-project",
      taskId: "file-task",
      kind: "merge",
      state: "applying",
      changes,
      taskAfter: { id: "file-task", status: "completed", direction: "code" },
    });
  } finally {
    legacyFiles.close();
  }
  await files.apply(projectRoot, changes.slice(0, 1));
  const fileResult = execFileSync(
    "rtk",
    [
      "cargo",
      "run",
      "--locked",
      "-p",
      "beaver-core",
      "--example",
      "files_contract",
      "--",
      output,
    ],
    {
      encoding: "utf8",
      timeout: 120000,
      windowsHide: true,
      stdio: ["ignore", "pipe", "pipe"],
    },
  );
  assert.match(fileResult, /NATIVE_FILES_CONTRACT_OK/);
  const fileProof = new Store(output);
  try {
    assert.deepEqual(
      fileProof.get<{ snapshot: unknown }>("nativeProof", "files")!.snapshot,
      after,
    );
    assert.equal(
      fileProof.get<{ state: string }>("operation", "file-operation")!.state,
      "complete",
    );
    assert.equal(
      fileProof.get<{ direction: string }>("task", "file-task")!.direction,
      "story",
    );
  } finally {
    fileProof.close();
  }
  assert.deepEqual(await files.capture(projectRoot), after);
  const proof = {
    output,
    checks: [
      "TypeScript writes / Rust reads",
      "Rust writes / TypeScript reads",
      "unknown fields and opaque secrets preserved",
      "interrupted status retains thread",
      "pending user question preserved",
      "event order preserved",
      "legacy content-addressed blobs read by Rust",
      "legacy partial file journal recovered by Rust",
      "Rust and TypeScript snapshot hashes match",
    ],
    personalDataTouched: false,
  };
  await fs.writeFile(
    path.join(output, "proof.json"),
    JSON.stringify(proof, null, 2),
  );
  console.log(JSON.stringify(proof));
}
void main();
