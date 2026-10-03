import assert from "node:assert/strict";
import fs from "node:fs/promises";
import path from "node:path";
import { createHash } from "node:crypto";
import test from "node:test";
import {
  includedNprFile,
  NPR_COMMIT,
  NPR_SOURCE_PATH,
  vendorNpr,
} from "../scripts/vendor-npr";

const packageRoot = path.resolve("resources/packages/npr-characters");

test("NPR distribution pins exact plugin bytes and distinct contract/plugin versions", async () => {
  const provenance = JSON.parse(
    await fs.readFile(path.join(packageRoot, "provenance.json"), "utf8"),
  );
  assert.equal(provenance.sourceCommit, NPR_COMMIT);
  assert.equal(provenance.sourcePath, NPR_SOURCE_PATH);
  assert.equal(provenance.contractVersion, "1.1.0");
  assert.equal(provenance.pluginVersion, "1.3.0");
  assert.equal(provenance.contract, "npr-character-frame");
  const installedRoot = path.join(packageRoot, NPR_SOURCE_PATH);
  for (const [name, sha256] of Object.entries(provenance.files)) {
    assert.ok(includedNprFile(name));
    const bytes = await fs.readFile(path.join(installedRoot, name));
    assert.equal(
      createHash("sha256").update(bytes).digest("hex"),
      sha256,
      name,
    );
    assert.equal(
      createHash("sha1")
        .update(`blob ${bytes.length}\0`)
        .update(bytes)
        .digest("hex"),
      provenance.gitBlobs[name],
      name,
    );
  }
  await assert.rejects(
    fs.access(path.join(packageRoot, "addons/npr_characters")),
  );
  await assert.rejects(fs.access(path.join(installedRoot, "samples")));
  assert.ok(provenance.files[".ci_script/model/check_model.gd"]);
  for (const name of ["geometry", "textures", "feature_data"]) {
    assert.ok(provenance.files[`docs/model_authoring/prompts/${name}.md`]);
  }
});

test("excluded sample launchers cannot leave dangling static Godot resource dependencies", async () => {
  const provenance = JSON.parse(
    await fs.readFile(path.join(packageRoot, "provenance.json"), "utf8"),
  );
  for (const name of Object.keys(provenance.files)) {
    if (!/\.(gd|tscn|tres|gdshader|gdshaderinc)$/.test(name)) continue;
    const content = await fs.readFile(
      path.join(packageRoot, NPR_SOURCE_PATH, name),
      "utf8",
    );
    const refs = content.matchAll(
      /(?:preload\(|load\(|#include\s+|\bpath=)\s*"(res:\/\/[^"\n]+)"/g,
    );
    for (const match of refs) {
      const resource = match[1]!;
      await assert.doesNotReject(
        fs.access(path.join(packageRoot, resource.slice("res://".length))),
        `${name}: ${resource}`,
      );
    }
  }
  for (const sample of [
    "samples/silver_wolf/character_definition.tres",
    "showcase/npr_character_preview.gd",
    "showcase/npr_lab.tscn",
    "showcase/wardrobe.tscn",
  ]) {
    assert.equal(includedNprFile(sample), false);
  }
  assert.equal(includedNprFile("showcase/wardrobe.gd"), true);
  assert.equal(includedNprFile("runtime/face/npr_face_rig.gd"), true);
});

test("unverified archive bytes cannot replace the currently bundled package", async () => {
  const os = await import("node:os");
  const temp = await fs.mkdtemp(path.join(os.tmpdir(), "beaver-vendor-test-"));
  try {
    const source = path.join(temp, "source");
    const target = path.join(temp, "target");
    await fs.mkdir(path.join(source, NPR_SOURCE_PATH), { recursive: true });
    await fs.mkdir(target);
    await fs.writeFile(path.join(target, "keep"), "unchanged");
    await fs.writeFile(
      path.join(source, NPR_SOURCE_PATH, "plugin.cfg"),
      "modified",
    );
    const manifest = path.join(temp, "manifest.json");
    await fs.writeFile(
      manifest,
      JSON.stringify({
        repositories: [
          {
            repository_url: "https://github.com/aiaimimi0920/NPRCharacterFrame",
            commit: NPR_COMMIT,
            files: [
              {
                path: `${NPR_SOURCE_PATH}/plugin.cfg`,
                git_mode: "100644",
                git_blob_oid: "0".repeat(40),
              },
            ],
          },
        ],
      }),
    );
    await assert.rejects(
      vendorNpr(source, target, manifest),
      /differs from pinned blob/,
    );
    assert.equal(
      await fs.readFile(path.join(target, "keep"), "utf8"),
      "unchanged",
    );
  } finally {
    await fs.rm(temp, { recursive: true, force: true });
  }
});
