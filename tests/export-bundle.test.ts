import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import {
  captureExportBundle,
  verifyExportBundle,
} from "../src/core/export-bundle";

async function fixture() {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), "beaver-export-"));
  const executable = Buffer.alloc(2048);
  executable.write("MZ");
  await fs.writeFile(path.join(root, "Game.exe"), executable);
  await fs.mkdir(path.join(root, "assets"));
  await fs.writeFile(path.join(root, "assets/game.pck"), "game content");
  const manifest = {
    version: 2,
    entry: "Game.exe",
    platform: "Windows Desktop",
    files: await captureExportBundle(root),
    runtimeVerified: false,
  };
  await fs.writeFile(
    path.join(root, "export-manifest.json"),
    JSON.stringify(manifest),
  );
  return {
    root,
    manifest,
    cleanup: () => fs.rm(root, { recursive: true, force: true }),
  };
}

test("full export manifest validates every payload file without executing the entry", async () => {
  const f = await fixture();
  try {
    await fs.writeFile(path.join(f.root, "export.log"), "diagnostic");
    const result = await verifyExportBundle(f.root);
    assert.equal(result.files, 2);
    assert.equal(result.bytes, 2060);
    await fs.writeFile(path.join(f.root, "assets/game.pck"), "altered pack");
    await assert.rejects(verifyExportBundle(f.root), /内容改变/);
  } finally {
    await f.cleanup();
  }
});

test("missing payload, extra executable, legacy manifest and path traversal cannot pass", async () => {
  const f = await fixture();
  try {
    await fs.writeFile(path.join(f.root, "unexpected.exe"), "extra");
    await assert.rejects(verifyExportBundle(f.root), /数量/);
    await fs.unlink(path.join(f.root, "unexpected.exe"));
    await fs.unlink(path.join(f.root, "assets/game.pck"));
    await assert.rejects(verifyExportBundle(f.root), /数量/);
    await fs.writeFile(
      path.join(f.root, "export-manifest.json"),
      JSON.stringify({
        ...f.manifest,
        files: [{ ...f.manifest.files[0], path: "../outside" }],
      }),
    );
    await assert.rejects(verifyExportBundle(f.root), /非法/);
    await fs.writeFile(
      path.join(f.root, "export-manifest.json"),
      JSON.stringify({ entry: "Game.exe", sha256: "0".repeat(64) }),
    );
    await assert.rejects(verifyExportBundle(f.root), /旧版/);
  } finally {
    await f.cleanup();
  }
});

test("duplicate records and incorrect platform headers are rejected", async () => {
  const f = await fixture();
  try {
    await fs.writeFile(
      path.join(f.root, "export-manifest.json"),
      JSON.stringify({
        ...f.manifest,
        files: [...f.manifest.files, f.manifest.files[0]],
      }),
    );
    await assert.rejects(verifyExportBundle(f.root), /重复/);
    await fs.writeFile(
      path.join(f.root, "export-manifest.json"),
      JSON.stringify({ ...f.manifest, platform: "Linux" }),
    );
    await assert.rejects(verifyExportBundle(f.root), /目标平台/);
  } finally {
    await f.cleanup();
  }
});
