import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs/promises";
import path from "node:path";
import os from "node:os";
import { parse } from "smol-toml";
import { NtExecutable, NtExecutableResource } from "pe-library";
import { Resource } from "resedit";
import {
  productVersion,
  readProductVersion,
  verifyProductVersion,
} from "../scripts/product-version";
import {
  stampWindowsVersion,
  verifyWindowsVersion,
} from "../scripts/windows-version";
import {
  inventory,
  MAX_PAYLOAD_BYTES,
  verifyNativeRelease,
} from "../scripts/native-release";

test("internal iterations preserve the public base and builds never bump it", () => {
  const metadata = Object.freeze({ version: "1.2.3", beaverBuild: 27 });
  assert.equal(productVersion(metadata).version, "1.2.3.27");
  assert.equal(productVersion(metadata).version, "1.2.3.27");
  const next = productVersion({ ...metadata, beaverBuild: 28 });
  assert.equal(next.version, "1.2.3.28");
  assert.equal(next.releaseVersion, "1.2.3");
  const release = productVersion(metadata, "release");
  assert.equal(release.version, "1.2.3");
  assert.equal(release.windowsVersion, "1.2.3.0");
  assert.equal(release.buildNumber, 27);
});

test("invalid channels, noncanonical versions and overflowing PE components fail", () => {
  for (const version of ["1.2", "1.2.3.4", "1.2.3-beta", "01.2.3", "65536.0.0"])
    assert.throws(() => productVersion({ version, beaverBuild: 1 }));
  for (const beaverBuild of [undefined, "1", 0, -1, 1.5, 65536])
    assert.throws(() => productVersion({ version: "1.2.3", beaverBuild }));
  for (const channel of ["", "preview", "relase"])
    assert.throws(() =>
      productVersion({ version: "1.2.3", beaverBuild: 1 }, channel),
    );
  const product = productVersion({ version: "1.2.3", beaverBuild: 1 });
  assert.throws(() => verifyProductVersion({ ...product, version: "1.2.3" }));
  assert.throws(() => verifyProductVersion({ ...product, buildNumber: 2 }));
  assert.throws(() => verifyProductVersion({ ...product, channel: "release" }));
});

test("repository package managers share the public SemVer base", async () => {
  const product = await readProductVersion(process.cwd(), "development");
  const json = async (file: string) =>
    JSON.parse(await fs.readFile(file, "utf8"));
  const lock = await json("package-lock.json");
  assert.equal(lock.version, product.releaseVersion);
  assert.equal(lock.packages[""].version, product.releaseVersion);
  assert.equal(
    (await json("native/desktop/tauri.conf.json")).version,
    product.releaseVersion,
  );
  for (const file of ["native/core/Cargo.toml", "native/desktop/Cargo.toml"]) {
    const manifest = parse(await fs.readFile(file, "utf8"));
    const pkg = manifest.package;
    assert.ok(pkg && typeof pkg === "object" && "version" in pkg);
    assert.equal(pkg.version, product.releaseVersion);
  }
  const cargoLock = parse(await fs.readFile("Cargo.lock", "utf8"));
  assert.ok(Array.isArray(cargoLock.package));
  const packages = cargoLock.package.filter(
    (pkg) =>
      typeof pkg === "object" &&
      pkg !== null &&
      "name" in pkg &&
      (pkg.name === "beaver-core" || pkg.name === "beaver-desktop"),
  );
  assert.equal(packages.length, 2);
  for (const pkg of packages) {
    assert.ok(pkg && typeof pkg === "object" && "version" in pkg);
    assert.equal(pkg.version, product.releaseVersion);
  }
});

function fixtureExecutable() {
  const executable = NtExecutable.createEmpty(false, false);
  executable.setExtraData(Buffer.from("preserve-overlay"));
  const resources = NtExecutableResource.from(executable);
  Resource.VersionInfo.create({
    lang: 1033,
    fixedInfo: {},
    strings: [1033, 2052].map((lang) => ({
      lang,
      codepage: 1200,
      values: {
        ProductName: "Beaver",
        FileVersion: "0.0.0.0",
        ProductVersion: "0.0.0.0",
      },
    })),
  }).outputToResourceEntries(resources.entries);
  resources.entries.push({
    type: 10,
    id: 123,
    lang: 1033,
    codepage: 0,
    bin: new Uint8Array([4, 3, 2, 1]).buffer,
  });
  resources.outputResource(executable);
  return Buffer.from(executable.generate());
}

test("PE round-trip preserves other resources and uses three-part public strings", () => {
  const original = fixtureExecutable();
  for (const channel of ["development", "release"]) {
    const product = productVersion(
      { version: "1.2.3", beaverBuild: 27 },
      channel,
    );
    const stamped = stampWindowsVersion(original, product);
    verifyWindowsVersion(stamped, product);
    const executable = NtExecutable.from(stamped);
    const resources = NtExecutableResource.from(executable);
    const [info] = Resource.VersionInfo.fromEntries(resources.entries);
    assert.ok(info);
    assert.equal(
      info.fixedInfo.fileVersionLS,
      channel === "release" ? 196608 : 196635,
    );
    for (const language of info.getAllLanguagesForStringValues()) {
      assert.equal(info.getStringValues(language).ProductName, "Beaver");
      assert.equal(
        info.getStringValues(language).ProductVersion,
        channel === "release" ? "1.2.3" : "1.2.3.27",
      );
    }
    const resource = resources.entries.find((entry) => entry.id === 123);
    assert.ok(resource);
    assert.deepEqual(Buffer.from(resource.bin), Buffer.from([4, 3, 2, 1]));
    assert.deepEqual(
      Buffer.from(executable.getExtraData()!),
      Buffer.from("preserve-overlay"),
    );
  }
  assert.throws(() =>
    verifyWindowsVersion(
      original,
      productVersion({ version: "1.2.3", beaverBuild: 1 }),
    ),
  );
});

test("native package verification rejects relabeling an internal binary as public", async () => {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), "beaver-version-"));
  try {
    const development = productVersion({ version: "1.2.3", beaverBuild: 27 });
    const release = productVersion(
      { version: "1.2.3", beaverBuild: 27 },
      "release",
    );
    for (const name of ["README.txt", "Start-Beaver.ps1", "THIRD-PARTY.json"])
      await fs.writeFile(path.join(root, name), name);
    await fs.writeFile(
      path.join(root, "Beaver.exe"),
      stampWindowsVersion(fixtureExecutable(), development),
    );
    const manifest = async (product: Record<string, unknown>) =>
      fs.writeFile(
        path.join(root, "RELEASE.json"),
        JSON.stringify({
          format: "beaver-native-release-v1",
          entry: "Beaver.exe",
          maxPayloadBytes: MAX_PAYLOAD_BYTES,
          files: await inventory(root),
          ...product,
        }),
      );
    await manifest(development);
    assert.equal((await verifyNativeRelease(root)).version, "1.2.3.27");
    await manifest(release);
    await assert.rejects(verifyNativeRelease(root));
    await fs.writeFile(
      path.join(root, "Beaver.exe"),
      stampWindowsVersion(fixtureExecutable(), release),
    );
    await manifest(release);
    assert.equal((await verifyNativeRelease(root)).version, "1.2.3");
    await manifest({ channel: "release" });
    await assert.rejects(verifyNativeRelease(root));
  } finally {
    await fs.rm(root, { recursive: true, force: true });
  }
});
