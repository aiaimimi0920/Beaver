import assert from "node:assert/strict";
import test from "node:test";
import fs from "node:fs/promises";
import path from "node:path";
import os from "node:os";
import {
  bundlePreset,
  discoverGodotBundle,
  requireBundleVersion,
  type GodotBundle,
} from "../src/core/godot-bundle";
import { templateRelease, templateVersion } from "../src/core/export-templates";
import {
  copyBundleLibraries,
  prepareBundleWorkspace,
} from "../src/core/game-export-workspace";

test("local bundle requires the exact custom build and never downloads stable fallback", () => {
  const version = "4.8.dev.custom_build.38b6ddee7";
  assert.equal(templateVersion(version), "4.8.dev");
  assert.throws(() => templateRelease(version));
  requireBundleVersion(version, version);
  for (const other of [
    "4.8.dev.custom_build.other",
    "4.5.1.stable.official.abc",
    "4.8.dev",
  ])
    assert.throws(() => requireBundleVersion(version, other));
});

test("local editor missing siblings fails instead of consulting installed templates", async () => {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), "beaver-bundle-"));
  try {
    const signal = new AbortController().signal;
    assert.equal(
      await discoverGodotBundle(
        path.join(root, "Godot_v4.5.1-stable_win64.exe"),
        signal,
      ),
      undefined,
    );
    await assert.rejects(
      discoverGodotBundle(
        path.join(root, "godot.windows.editor.x86_64.exe"),
        signal,
      ),
      /缺失/,
    );
  } finally {
    await fs.rm(root, { recursive: true, force: true });
  }
});

test("bundle export uses an isolated copy, keeps other presets and preserves live project", async () => {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), "beaver-bundle-"));
  try {
    const project = path.join(root, "project");
    const workspace = path.join(root, "workspace");
    for (const dir of [project, workspace]) await fs.mkdir(dir);
    const bundle: GodotBundle = {
      directory: root,
      debug: path.join(root, "debug.exe"),
      release: path.join(root, "release.exe"),
      version: "4.8.dev.custom_build.38b6ddee7",
    };
    await fs.writeFile(bundle.debug, "debug");
    await fs.writeFile(bundle.release, "release");
    const config =
      '[preset.0]\nname="Other"\n[preset.0.options]\ncustom_template/release="other.exe"\n[preset.1]\nname="Desktop"\nplatform="Windows Desktop"\n[preset.1.options]\ncustom_template/release=""\nbinary_format/embed_pck=true\n';
    await fs.writeFile(path.join(project, "export_presets.cfg"), config);
    await fs.mkdir(path.join(project, ".beaver"));
    await fs.writeFile(path.join(project, ".beaver", "state"), "private");
    await prepareBundleWorkspace(project, workspace, bundle, "Desktop");
    const bound = await fs.readFile(
      path.join(workspace, "export_presets.cfg"),
      "utf8",
    );
    assert.ok(bound.includes('custom_template/release="other.exe"'));
    assert.ok(bound.includes("binary_format/embed_pck=true"));
    assert.ok(bound.includes(bundle.release.replace(/\\/g, "/")));
    assert.equal(
      await fs.readFile(path.join(project, "export_presets.cfg"), "utf8"),
      config,
    );
    await assert.rejects(fs.stat(path.join(workspace, ".beaver")), {
      code: "ENOENT",
    });
    assert.equal(
      await bundlePreset(bundle, bound, "Desktop", workspace),
      bound,
    );
    assert.ok(
      (
        await bundlePreset(
          bundle,
          '[preset.0]\nname="Desktop"\n',
          "Desktop",
          workspace,
        )
      ).includes("[preset.0.options]"),
    );
    await assert.rejects(
      bundlePreset(
        bundle,
        '[preset.0]\nname="Desktop"\n[preset.0.options]\nbinary_format/architecture="arm64"\n',
        "Desktop",
        workspace,
      ),
      /x86_64/,
    );
    await assert.rejects(
      bundlePreset(
        bundle,
        '[preset.0]\nname="Desktop"\n[preset.0.options]\ncustom_template/release="other.exe"\n',
        "Desktop",
        root,
      ),
    );
    await fs.writeFile(path.join(root, "SpoutLibrary.dll"), "library");
    await copyBundleLibraries(bundle, workspace);
    assert.equal(
      await fs.readFile(path.join(workspace, "SpoutLibrary.dll"), "utf8"),
      "library",
    );
    await assert.rejects(copyBundleLibraries(bundle, workspace), {
      code: "EEXIST",
    });
    assert.equal(
      await fs.readFile(path.join(workspace, "SpoutLibrary.dll"), "utf8"),
      "library",
    );
  } finally {
    await fs.rm(root, { recursive: true, force: true });
  }
});
