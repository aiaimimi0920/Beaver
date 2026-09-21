import {
  cp,
  mkdir,
  unlink,
  writeFile,
  access,
  readFile,
  readdir,
} from "node:fs/promises";
import { createHash } from "node:crypto";
import path from "node:path";
import assert from "node:assert/strict";
import { NtExecutable, NtExecutableResource } from "pe-library";
import { Data, Resource } from "resedit";
import { readProductVersion, verifyProductVersion } from "./product-version.ts";
import { setWindowsVersion, verifyWindowsVersion } from "./windows-version.ts";
if (process.platform !== "win32" || process.arch !== "x64")
  throw new Error("This preview packager is verified for Windows x64 only");
const releaseRoot = path.resolve("release");
const product = await readProductVersion();
const { version } = product;
assert.deepEqual(
  verifyProductVersion(JSON.parse(await readFile("dist/build.json", "utf8"))),
  product,
  "Rebuild Electron for the selected version and channel",
);
const target = path.resolve(
  process.env.BEAVER_RELEASE_DIR ||
    path.join(
      releaseRoot,
      `Beaver-${version}-${process.platform}-${process.arch}`,
    ),
);
if (!target.startsWith(releaseRoot + path.sep))
  throw new Error("Release output must stay inside the release directory");
try {
  await access(target);
  throw new Error(
    "Release already exists; choose a new output version before replacing it.",
  );
} catch (e) {
  if (e.code !== "ENOENT") throw e;
}
await mkdir(target, { recursive: true });
await cp("node_modules/electron/dist", target, { recursive: true });
const app = path.join(
  target,
  process.platform === "darwin"
    ? "Electron.app/Contents/Resources/app"
    : "resources/app",
);
await mkdir(app, { recursive: true });
await cp("dist", path.join(app, "dist"), { recursive: true });
await cp("resources", path.join(app, "resources"), { recursive: true });
await cp("README.md", path.join(target, "START-HERE.md"));
await cp("README.md", path.join(target, "README.md"));
await cp("docs", path.join(target, "docs"), { recursive: true });
const licenses = path.join(target, "third-party-licenses");
await mkdir(licenses, { recursive: true });
for (const name of [
  "electron",
  "react",
  "react-dom",
  "scheduler",
  "smol-toml",
  "three",
  "zod",
])
  await cp(`node_modules/${name}/LICENSE`, path.join(licenses, `${name}.txt`));
await writeFile(
  path.join(app, "package.json"),
  JSON.stringify({
    name: "beaver-studio",
    productName: "Beaver",
    version: product.releaseVersion,
    main: "dist/main.cjs",
  }),
);
// Only edit the new release copy; never the installed Electron or an old release.
const electronPath = path.join(target, "electron.exe");
const executable = NtExecutable.from(await readFile(electronPath), {
  ignoreCert: true,
});
const resources = NtExecutableResource.from(executable);
const icons = Data.IconFile.from(
  await readFile("resources/branding/beaver.ico"),
).icons.map((item) => item.data);
const groups = Resource.IconGroupEntry.fromEntries(resources.entries);
assert.ok(groups.length > 0, "Electron must have an icon group to replace");
for (const group of groups)
  Resource.IconGroupEntry.replaceIconsForResource(
    resources.entries,
    group.id,
    group.lang,
    icons,
  );
for (const info of Resource.VersionInfo.fromEntries(resources.entries)) {
  setWindowsVersion(info, product);
  for (const language of info.getAllLanguagesForStringValues()) {
    info.setStringValues(language, {
      FileDescription: "Beaver",
      ProductName: "Beaver",
      InternalName: "Beaver.exe",
      OriginalFilename: "Beaver.exe",
    });
    info.removeStringValue(language, "CompanyName");
  }
  info.outputToResourceEntries(resources.entries);
}
resources.outputResource(executable);
// Resource editing produces an unsigned preview binary, as recorded below.
const branded = Buffer.from(executable.generate());
verifyWindowsVersion(branded, product);
const verifiedResources = NtExecutableResource.from(NtExecutable.from(branded));
const verifiedGroups = Resource.IconGroupEntry.fromEntries(
  verifiedResources.entries,
);
assert.equal(verifiedGroups.length, groups.length);
for (const group of verifiedGroups) {
  const embedded = group.getIconItemsFromEntries(verifiedResources.entries);
  assert.equal(embedded.length, icons.length);
  for (let i = 0; i < icons.length; i++) {
    assert.ok(embedded[i].isRaw() && icons[i].isRaw());
    assert.deepEqual(Buffer.from(embedded[i].bin), Buffer.from(icons[i].bin));
  }
}
await writeFile(path.join(target, "Beaver.exe"), branded, { flag: "wx" });
await unlink(electronPath);
const files = [];
async function inventory(dir, relative = "") {
  for (const entry of await readdir(dir, { withFileTypes: true })) {
    const name = relative ? `${relative}/${entry.name}` : entry.name;
    if (entry.isDirectory()) await inventory(path.join(dir, entry.name), name);
    else {
      const bytes = await readFile(path.join(dir, entry.name));
      files.push({
        path: name,
        bytes: bytes.length,
        sha256: createHash("sha256").update(bytes).digest("hex"),
      });
    }
  }
}
await inventory(target);
await writeFile(
  path.join(target, "RELEASE.json"),
  JSON.stringify(
    {
      name: "Beaver",
      ...product,
      platform: process.platform,
      arch: process.arch,
      builtAt: new Date().toISOString(),
      signed: false,
      runtimeVerified: false,
      entry: "Beaver.exe",
      files,
    },
    null,
    2,
  ),
);
console.log(target);
