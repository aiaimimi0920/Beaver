import fs from "node:fs/promises";
import path from "node:path";
import assert from "node:assert/strict";
import { Store } from "../src/core/store";
import { Projects } from "../src/core/projects";
import { Preferences } from "../src/core/settings";
import { defaultSettings } from "../src/shared/types";
import { Games } from "../src/core/game";
import { findTool, runCommand } from "../src/core/process";
import { verifyExportBundle } from "../src/core/export-bundle";

async function main() {
  const output = path.resolve("output/validation", `godot-${Date.now()}`);
  await fs.mkdir(output, { recursive: true });
  const archive = process.env.BEAVER_TEMPLATE_ARCHIVE;
  if (archive) process.env.APPDATA = path.join(output, "godot-data");
  const store = new Store(path.join(output, "data"));
  const prefs = new Preferences(store, {
    encrypt: (s) => s,
    decrypt: (s) => s,
  });
  const settings = defaultSettings();
  settings.tools.godot = await findTool("godot", process.env.GODOT_PATH);
  prefs.save(settings, {});
  const games = new Games(prefs);
  try {
    if (archive) {
      const prepared = await games.prepareTemplates(path.resolve(archive));
      assert.equal(prepared.reused, false);
      const reused = await games.prepareTemplates(path.resolve(archive));
      assert.equal(reused.reused, true);
    }
    const project = await new Projects(store, path.resolve("resources")).create(
      output,
      "nightbar",
      "nightbar",
    );
    const check = async (name: string, command: string, args: string[]) => {
      const result = await runCommand(command, args, project.path, 180000);
      await fs.writeFile(path.join(output, name + ".log"), result.output);
      assert.equal(result.code, 0, result.output);
      assert.ok(
        !/SCRIPT ERROR|Parse Error|ERROR:/.test(result.output),
        result.output,
      );
      return result.output;
    };
    await check("import", settings.tools.godot, [
      "--headless",
      "--path",
      project.path,
      "--editor",
      "--import",
      "--quit",
    ]);
    await fs.copyFile(
      "tests/godot/nightbar-smoke.gd",
      path.join(project.path, "smoke.gd"),
    );
    const gameplay = await check("gameplay", settings.tools.godot, [
      "--headless",
      "--path",
      project.path,
      "--script",
      "res://smoke.gd",
    ]);
    assert.match(gameplay, /BEAVER_GAMEPLAY_OK/);
    await fs.rm(path.join(project.path, "smoke.gd"));
    const exported = await games.export(project, output, "Windows Desktop");
    await verifyExportBundle(exported.path);
    const executable = path.join(exported.path, "Game.exe");
    await check("standalone", executable, ["--headless", "--quit-after", "12"]);
    const manifestPath = path.join(exported.path, "export-manifest.json");
    const manifest = JSON.parse(
      await fs.readFile(manifestPath, "utf8"),
    ) as Record<string, unknown>;
    manifest.runtimeVerified = true;
    manifest.validation =
      "Headless standalone launch, 12 frames; gameplay tested in source scene separately. Visual quality requires user acceptance.";
    await fs.writeFile(manifestPath, JSON.stringify(manifest, null, 2));
    await fs.writeFile(
      path.join(output, "godot-proof.json"),
      JSON.stringify(
        {
          engine: settings.tools.godot,
          exported: exported.path,
          project: project.path,
          checks: [
            "real Godot import",
            "three recipes",
            "duplicate serve",
            "narrative progression",
            "save/load",
            "real Windows export",
            "complete exported payload integrity",
            "standalone executable launch without editor",
          ],
          templateVerification: archive
            ? "Imported trusted local TPZ into isolated APPDATA using production installer; version and Windows executables checked; repeated prepare reused files. Archive authenticity not rehashed."
            : "Installed templates used; source archive was not rehashed in this run.",
        },
        null,
        2,
      ),
    );
    console.log(JSON.stringify({ success: true, output, executable }));
  } finally {
    await games.shutdown();
    store.close();
  }
}
main().catch((e: unknown) => {
  console.error(e);
  process.exitCode = 1;
});
