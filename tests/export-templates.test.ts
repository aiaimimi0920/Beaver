import assert from "node:assert/strict";
import test from "node:test";
import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { createHash } from "node:crypto";
import {
  templateVersion,
  templateRelease,
  templateChecksum,
  templateDirectory,
  templatesReady,
  installTemplateArchive,
  downloadTemplates,
} from "../src/core/export-templates";
import { runCommand } from "../src/core/process";

test("template downloads stream to disk and reject checksum mismatches", async (t) => {
  const root = await fs.mkdtemp(
    path.join(os.tmpdir(), "beaver-template-download-"),
  );
  const content = Buffer.from("test archive bytes");
  const digest = createHash("sha512").update(content).digest("hex");
  let tamper = false;
  t.mock.method(globalThis, "fetch", async (url: string) => {
    assert.ok(
      url.startsWith(
        "https://github.com/godotengine/godot-builds/releases/download/4.6-stable/",
      ),
    );
    return url.endsWith("SHA512-SUMS.txt")
      ? new Response(`${digest}  Godot_v4.6-stable_export_templates.tpz\n`)
      : new Response(tamper ? "tampered bytes" : content);
  });
  try {
    const file = path.join(root, "valid.tpz");
    await downloadTemplates(
      "4.6.stable.official.abc",
      file,
      new AbortController().signal,
    );
    assert.deepEqual(await fs.readFile(file), content);
    tamper = true;
    await assert.rejects(
      downloadTemplates(
        "4.6.stable.official.abc",
        path.join(root, "invalid.tpz"),
        new AbortController().signal,
      ),
      /SHA-512/,
    );
  } finally {
    await fs.rm(root, { recursive: true, force: true });
  }
});

test("template version comes from executable output, not directory names", () => {
  assert.equal(
    templateVersion("4.6.stable.custom_build.abcdef\n"),
    "4.6.stable",
  );
  assert.equal(templateVersion("4.5.2.rc1.official.abcdef"), "4.5.2.rc1");
  assert.throws(() => templateVersion("../4.6"));
  assert.throws(() => templateVersion("3.6.stable.official"));
  assert.equal(
    templateRelease("4.6.stable.official.abc").filename,
    "Godot_v4.6-stable_export_templates.tpz",
  );
  assert.throws(() => templateRelease("4.6.stable.custom_build.abc"));
  assert.throws(() => templateRelease("4.6.stable.mono.official.abc"));
  assert.throws(() => templateRelease("4.6.rc1.official.abc"));
});

test("official checksum requires exactly one exact filename", () => {
  const digest = "a".repeat(128);
  assert.equal(
    templateChecksum(`${digest} *templates.tpz\n`, "templates.tpz"),
    digest,
  );
  assert.throws(() =>
    templateChecksum(`${digest} templates.tpz.bad`, "templates.tpz"),
  );
  assert.throws(() =>
    templateChecksum(
      `${digest} templates.tpz\n${digest} templates.tpz`,
      "templates.tpz",
    ),
  );
  assert.throws(() => templateChecksum("bad templates.tpz", "templates.tpz"));
});

test("template discovery honors self-contained engines and rejects arbitrary directory versions", async () => {
  const root = await fs.mkdtemp(
    path.join(os.tmpdir(), "beaver-template-path-"),
  );
  try {
    const executable = path.join(root, "godot.exe");
    assert.equal(
      await templateDirectory(executable, "4.6.stable", root),
      path.join(root, "Godot/export_templates/4.6.stable"),
    );
    await fs.writeFile(path.join(root, "_sc_"), "");
    assert.equal(
      await templateDirectory(executable, "4.6.stable", root),
      path.join(root, "editor_data/export_templates/4.6.stable"),
    );
    await assert.rejects(
      templateDirectory(executable, "4.6.stable/../../elsewhere", root),
    );
    assert.equal(await templatesReady(root), false);
    await fs.writeFile(
      path.join(root, "windows_release_x86_64.exe"),
      Buffer.alloc(2048),
    );
    await fs.writeFile(
      path.join(root, "windows_debug_x86_64.exe"),
      Buffer.alloc(2048),
    );
    assert.equal(await templatesReady(root), false);
  } finally {
    await fs.rm(root, { recursive: true, force: true });
  }
});

test(
  "Windows TPZ import validates version, extracts only approved files and never replaces existing templates",
  { skip: process.platform !== "win32" },
  async () => {
    const root = await fs.mkdtemp(path.join(os.tmpdir(), "beaver-tpz-"));
    const archive = path.join(root, "test.tpz");
    const literal = (s: string) => `'${s.replace(/'/g, "''")}'`;
    try {
      const script = `$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem
$zip = [IO.Compression.ZipFile]::Open(${literal(archive)}, [IO.Compression.ZipArchiveMode]::Create)
try {
foreach ($name in @('templates/version.txt','templates/windows_release_x86_64.exe','templates/windows_debug_x86_64.exe','../../outside.txt')) {
 $entry = $zip.CreateEntry($name); $stream = $entry.Open()
 try {
  if ($name.EndsWith('version.txt')) { $bytes = [Text.Encoding]::UTF8.GetBytes('4.6.stable') }
  else { $bytes = New-Object byte[] 2048; $bytes[0] = 77; $bytes[1] = 90 }
  $stream.Write($bytes, 0, $bytes.Length)
 } finally { $stream.Dispose() }
}
} finally { $zip.Dispose() }`;
      const result = await runCommand("powershell.exe", [
        "-NoProfile",
        "-EncodedCommand",
        Buffer.from(script, "utf16le").toString("base64"),
      ]);
      assert.equal(result.code, 0, result.output);
      const mismatch = path.join(root, "4.5.stable");
      await assert.rejects(
        installTemplateArchive(
          archive,
          mismatch,
          "4.5.stable",
          new AbortController().signal,
        ),
        /version does not match/,
      );
      assert.equal(
        await fs.stat(mismatch).then(
          () => true,
          () => false,
        ),
        false,
      );
      const destination = path.join(root, "4.6.stable");
      await installTemplateArchive(
        archive,
        destination,
        "4.6.stable",
        new AbortController().signal,
      );
      assert.equal(await templatesReady(destination), true);
      assert.deepEqual((await fs.readdir(destination)).sort(), [
        "version.txt",
        "windows_debug_x86_64.exe",
        "windows_release_x86_64.exe",
      ]);
      await assert.rejects(
        installTemplateArchive(
          archive,
          destination,
          "4.6.stable",
          new AbortController().signal,
        ),
        /已经存在/,
      );
      const aborted = new AbortController();
      aborted.abort();
      await assert.rejects(
        installTemplateArchive(
          archive,
          path.join(root, "cancelled"),
          "4.6.stable",
          aborted.signal,
        ),
      );
      assert.equal(
        (await fs.readdir(root)).some((name) =>
          name.startsWith(".beaver-templates-"),
        ),
        false,
      );
    } finally {
      await fs.rm(root, { recursive: true, force: true });
    }
  },
);
