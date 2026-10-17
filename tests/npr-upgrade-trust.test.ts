import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import fs from "node:fs/promises";
import { readFileSync } from "node:fs";
import path from "node:path";
import test from "node:test";

const root = "resources/packages/npr-characters";
const fixture = JSON.parse(
  readFileSync("tests/fixtures/npr-upgrade-legacy.json", "utf8"),
);
const current = JSON.parse(readFileSync(`${root}/provenance.json`, "utf8"));
const sha256 = (bytes: Uint8Array) =>
  createHash("sha256").update(bytes).digest("hex");

for (const [pin, manifestHash] of [
  [
    "eb5a0511",
    "5bc70f709f1e8e9d155bd889ace809abdd39baf0e0edb68a95918aa60d4dd5fe",
  ],
  [
    "05f3dcee",
    "e545ecc7e6737b144bd92cb0fe7020d1b71e2ecfa8cd8711112fe19f72616576",
  ],
  [
    "48839ce1",
    "523bfa632690a5eddfa9fb2ac8f35d10b94399033381591b09770e7a3fe81a0d",
  ],
]) {
  test(`trusted historical NPR ${pin} reconstructs every exact dependency`, async () => {
    const bytes = await fs.readFile(
      `resources/workflows/npr-upgrade-from-${pin}.json`,
    );
    const manifest = JSON.parse(bytes.toString("utf8"));
    assert.ok(manifest.sourceCommit.startsWith(pin));
    assert.equal(manifest.sourcePath, current.sourcePath);
    assert.equal(manifest.contractVersion, current.contractVersion);
    assert.equal(manifest.pluginVersion, current.pluginVersion);
    // Full hashes are pinned below from the source-controlled historical manifests.
    assert.equal(sha256(bytes), manifestHash);
    const replacements = fixture[manifest.sourceCommit];
    assert.ok(replacements);
    let changed = 0;
    for (const [name, hash] of Object.entries(manifest.files)) {
      const historical = replacements[name]
        ? Buffer.from(replacements[name], "base64")
        : await fs.readFile(path.join(root, manifest.sourcePath, name));
      assert.equal(sha256(historical), hash, name);
      assert.equal(
        createHash("sha1")
          .update(`blob ${historical.length}\0`)
          .update(historical)
          .digest("hex"),
        manifest.gitBlobs[name],
        name,
      );
      if (current.files[name] !== hash) changed++;
    }
    assert.equal(changed, Object.keys(replacements).length);
    assert.equal(Object.keys(manifest.files).length, 369);
  });
}
