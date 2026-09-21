import fs from "node:fs/promises";
import path from "node:path";
import { createHash } from "node:crypto";
import assert from "node:assert/strict";
import { readProductVersion, verifyProductVersion } from "./product-version.ts";
import { verifyWindowsVersion } from "./windows-version.ts";

const { version } = await readProductVersion();
const root = path.resolve(
  process.argv[2] || `release/Beaver-${version}-win32-x64`,
);
const allowed = path.resolve("release") + path.sep;
assert.ok(
  root.startsWith(allowed),
  "Release must remain in this workspace release directory",
);
const manifestPath = path.join(root, "RELEASE.json");
const manifest = JSON.parse(await fs.readFile(manifestPath, "utf8"));
if (manifest.channel !== undefined) {
  verifyWindowsVersion(
    await fs.readFile(path.join(root, "Beaver.exe")),
    verifyProductVersion(manifest),
  );
}
let bytes = 0;
for (const file of manifest.files) {
  const resolved = path.resolve(root, file.path);
  assert.ok(resolved.startsWith(root + path.sep));
  const data = await fs.readFile(resolved);
  assert.equal(data.length, file.bytes, file.path);
  assert.equal(
    createHash("sha256").update(data).digest("hex"),
    file.sha256,
    file.path,
  );
  bytes += data.length;
}
if (process.argv[3]) {
  const proof = JSON.parse(
    await fs.readFile(path.resolve(process.argv[3]), "utf8"),
  );
  assert.equal(
    path.resolve(proof.actualExecutable).toLowerCase(),
    path.join(root, manifest.entry).toLowerCase(),
  );
  assert.ok(proof.checks.includes("full exit and restart retains tasks"));
  assert.deepEqual(proof.errors, []);
  manifest.runtimeVerified = true;
  manifest.verifiedAt = new Date().toISOString();
  manifest.runtimeChecks = proof.checks;
  await fs.writeFile(manifestPath, JSON.stringify(manifest, null, 2));
}
console.log(
  JSON.stringify({
    root,
    files: manifest.files.length,
    bytes,
    runtimeVerified: manifest.runtimeVerified,
  }),
);
