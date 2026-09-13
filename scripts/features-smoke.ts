import fs from "node:fs/promises";
import path from "node:path";
import assert from "node:assert/strict";
import { findTool, runCommand } from "../src/core/process";

async function main() {
  const output = path.resolve("output/validation", `features-${Date.now()}`);
  const project = path.join(output, "project");
  await fs.mkdir(project, { recursive: true });
  await fs.writeFile(
    path.join(project, "project.godot"),
    'config_version=5\n[application]\nconfig/name="Beaver Feature Smoke"\n[rendering]\nrenderer/rendering_method="gl_compatibility"\n',
  );
  await fs.cp("resources/features", path.join(project, "features"), {
    recursive: true,
  });
  await fs.copyFile(
    "tests/godot/features-smoke.gd",
    path.join(project, "smoke.gd"),
  );
  const engine = await findTool("godot", process.env.GODOT_PATH);
  let result = "";
  for (const [name, args] of [
    [
      "parse-check",
      [
        "--headless",
        "--path",
        project,
        "--script",
        "res://smoke.gd",
        "--check-only",
      ],
    ],
    [
      "modules",
      ["--headless", "--path", project, "--script", "res://smoke.gd"],
    ],
  ] as const) {
    const run = await runCommand(engine, [...args], project, 120000);
    await fs.writeFile(path.join(output, `${name}.log`), run.output);
    assert.equal(run.code, 0, run.output);
    assert.doesNotMatch(run.output, /SCRIPT ERROR|Parse Error|ERROR:/);
    result = run.output;
  }
  assert.match(result, /BEAVER_FEATURES_OK checks=\d+ modules=8/);
  await fs.writeFile(
    path.join(output, "features-proof.json"),
    JSON.stringify(
      {
        engine,
        project,
        success: true,
        result: result.match(/BEAVER_FEATURES_OK[^\r\n]*/)?.[0],
      },
      null,
      2,
    ),
  );
  console.log(JSON.stringify({ success: true, output }));
}
void main().catch((error: unknown) => {
  console.error(error);
  process.exitCode = 1;
});
