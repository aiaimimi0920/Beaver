import assert from "node:assert/strict";
import { NtExecutable, NtExecutableResource } from "pe-library";
import { Resource } from "resedit";
import { type ProductVersion } from "./product-version";

export function setWindowsVersion(
  info: Resource.VersionInfo,
  version: ProductVersion,
) {
  const languages = info.getAllLanguagesForStringValues();
  assert.ok(languages.length, "Executable version has no string table");
  for (const language of languages) {
    info.setFileVersion(version.windowsVersion, language.lang);
    info.setProductVersion(version.windowsVersion, language.lang);
    // PE fixed fields always have four components; public strings have three.
    info.setStringValues(language, {
      FileVersion: version.version,
      ProductVersion: version.version,
    });
  }
}

export function verifyWindowsVersion(
  data: Uint8Array,
  version: ProductVersion,
) {
  const executable = NtExecutable.from(data, { ignoreCert: true });
  const resources = NtExecutableResource.from(executable);
  const infos = Resource.VersionInfo.fromEntries(resources.entries);
  assert.ok(infos.length, "Executable has no version resource");
  const [major, minor, patch, revision] = version.windowsVersion
    .split(".")
    .map(Number) as [number, number, number, number];
  for (const info of infos) {
    assert.equal(info.fixedInfo.fileVersionMS, major * 65536 + minor);
    assert.equal(info.fixedInfo.fileVersionLS, patch * 65536 + revision);
    assert.equal(info.fixedInfo.productVersionMS, major * 65536 + minor);
    assert.equal(info.fixedInfo.productVersionLS, patch * 65536 + revision);
    const languages = info.getAllLanguagesForStringValues();
    assert.ok(languages.length, "Executable version has no string table");
    for (const language of languages) {
      const strings = info.getStringValues(language);
      assert.equal(
        strings.FileVersion,
        version.version,
        "FileVersion mismatch",
      );
      assert.equal(
        strings.ProductVersion,
        version.version,
        "ProductVersion mismatch",
      );
    }
  }
}

export function stampWindowsVersion(data: Uint8Array, version: ProductVersion) {
  // Signed binaries must be stamped before signing, never silently stripped.
  const executable = NtExecutable.from(data);
  const resources = NtExecutableResource.from(executable);
  const infos = Resource.VersionInfo.fromEntries(resources.entries);
  assert.ok(infos.length, "Executable has no version resource");
  for (const info of infos) {
    setWindowsVersion(info, version);
    info.outputToResourceEntries(resources.entries);
  }
  resources.outputResource(executable);
  const result = Buffer.from(executable.generate());
  verifyWindowsVersion(result, version);
  return result;
}
