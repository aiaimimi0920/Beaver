import fs from "node:fs/promises";
import path from "node:path";
import { createHash } from "node:crypto";
import assert from "node:assert/strict";
import { verifyProductVersion } from "./product-version";
import { verifyWindowsVersion } from "./windows-version";

export const MAX_PAYLOAD_BYTES = 50_000_000;
export type PayloadFile = { path: string; bytes: number; sha256: string };

export async function inventory(root: string): Promise<PayloadFile[]> {
  const files: PayloadFile[] = [];
  async function visit(directory: string) {
    for (const name of (await fs.readdir(directory)).sort()) {
      const file = path.join(directory, name);
      const stat = await fs.lstat(file);
      assert.ok(!stat.isSymbolicLink(), `Linked payload entry: ${name}`);
      if (stat.isDirectory()) await visit(file);
      else {
        assert.ok(stat.isFile(), `Unsupported payload entry: ${name}`);
        const relative = path.relative(root, file).split(path.sep).join("/");
        if (relative === "RELEASE.json") continue;
        const data = await fs.readFile(file);
        files.push({
          path: relative,
          bytes: data.length,
          sha256: createHash("sha256").update(data).digest("hex"),
        });
      }
    }
  }
  await visit(root);
  return files;
}

export async function verifyNativeRelease(root: string) {
  const manifestBytes = await fs.readFile(path.join(root, "RELEASE.json"));
  const manifest = JSON.parse(manifestBytes.toString("utf8"));
  assert.equal(manifest.format, "beaver-native-release-v1");
  assert.equal(manifest.entry, "Beaver.exe");
  assert.equal(manifest.maxPayloadBytes, MAX_PAYLOAD_BYTES);
  // Historical v1 releases are still used as verified license caches.
  if (
    manifest.channel !== "migration-preview" ||
    manifest.version !== undefined
  ) {
    const product = verifyProductVersion(manifest);
    verifyWindowsVersion(
      await fs.readFile(path.join(root, "Beaver.exe")),
      product,
    );
  }
  const files = await inventory(root);
  assert.deepEqual(files, manifest.files, "Payload differs from manifest");
  assert.deepEqual(
    files.filter((f) => /\.(exe|dll|node)$/i.test(f.path)).map((f) => f.path),
    ["Beaver.exe"],
    "Unexpected runtime binary",
  );
  for (const name of ["README.txt", "Start-Beaver.ps1", "THIRD-PARTY.json"]) {
    assert.ok(
      files.some((file) => file.path === name),
      `Missing ${name}`,
    );
  }
  const totalBytes = files.reduce(
    (sum, file) => sum + file.bytes,
    manifestBytes.length,
  );
  assert.ok(
    totalBytes <= MAX_PAYLOAD_BYTES,
    `Payload exceeds ${MAX_PAYLOAD_BYTES} bytes: ${totalBytes}`,
  );
  return {
    root,
    version: manifest.version,
    files: files.length + 1,
    totalBytes,
    runtimeVerified: manifest.runtimeVerified === true,
  };
}
