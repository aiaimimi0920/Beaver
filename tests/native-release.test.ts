import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import {
  inventory,
  MAX_PAYLOAD_BYTES,
  verifyNativeRelease,
} from "../scripts/native-release";

test("native payload gate rejects drift, injected runtimes and total size including manifest", async () => {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), "beaver-payload-"));
  try {
    for (const name of [
      "Beaver.exe",
      "README.txt",
      "Start-Beaver.ps1",
      "THIRD-PARTY.json",
    ])
      await fs.writeFile(path.join(root, name), name);
    const manifest = async () =>
      fs.writeFile(
        path.join(root, "RELEASE.json"),
        JSON.stringify({
          format: "beaver-native-release-v1",
          entry: "Beaver.exe",
          maxPayloadBytes: MAX_PAYLOAD_BYTES,
          runtimeVerified: false,
          files: await inventory(root),
        }),
      );
    await manifest();
    const result = await verifyNativeRelease(root);
    assert.equal(result.runtimeVerified, false);
    assert.equal(
      result.totalBytes,
      (
        await Promise.all(
          (await fs.readdir(root)).map(
            async (name) => (await fs.stat(path.join(root, name))).size,
          ),
        )
      ).reduce((a, b) => a + b, 0),
    );
    await fs.writeFile(path.join(root, "README.txt"), "changed");
    await assert.rejects(verifyNativeRelease(root), /Payload differs/);
    await manifest();
    await fs.writeFile(path.join(root, "beaver-media.exe"), "old helper");
    await assert.rejects(verifyNativeRelease(root), /Payload differs/);
    await manifest();
    await assert.rejects(verifyNativeRelease(root), /Unexpected runtime/);
    await fs.unlink(path.join(root, "beaver-media.exe"));
    const handle = await fs.open(path.join(root, "Beaver.exe"), "w");
    await handle.truncate(MAX_PAYLOAD_BYTES - 100);
    await handle.close();
    await manifest();
    await assert.rejects(verifyNativeRelease(root), /Payload exceeds/);
  } finally {
    await fs.rm(root, { recursive: true, force: true });
  }
});
