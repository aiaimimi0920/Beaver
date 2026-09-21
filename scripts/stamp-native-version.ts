import assert from "node:assert/strict";
import fs from "node:fs/promises";
import { readProductVersion } from "./product-version";
import { stampWindowsVersion, verifyWindowsVersion } from "./windows-version";

async function main() {
  assert.equal(process.platform, "win32", "Windows native executable only");
  const version = await readProductVersion();
  assert.deepEqual(
    JSON.parse(await fs.readFile("dist-native/build.json", "utf8")),
    version,
    "Rebuild the native frontend with the same version and channel first",
  );
  const file = "target/release/Beaver.exe";
  const original = await fs.readFile(file);
  try {
    verifyWindowsVersion(original, version);
  } catch {
    await fs.writeFile(file, stampWindowsVersion(original, version));
    verifyWindowsVersion(await fs.readFile(file), version);
  }
  console.log(
    `Native executable version: ${version.version} (${version.channel})`,
  );
}

main().catch((error: unknown) => {
  console.error(error);
  process.exitCode = 1;
});
