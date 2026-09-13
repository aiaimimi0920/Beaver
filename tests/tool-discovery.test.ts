import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs/promises";
import path from "node:path";
import os from "node:os";
import { findTool } from "../src/core/process";
import {
  executableHint,
  isToolExecutable,
  launcherTarget,
  resolveToolHints,
} from "../src/core/tool-discovery";

test("Windows application hints preserve spaces and remove icon indexes or arguments", () => {
  assert.equal(
    executableHint('"D:\\Steam Library\\Blender\\blender.exe",0'),
    "D:\\Steam Library\\Blender\\blender.exe",
  );
  assert.equal(
    executableHint('D:\\Apps\\Godot_v4.7_win64.exe "%1"'),
    "D:\\Apps\\Godot_v4.7_win64.exe",
  );
  assert.equal(
    executableHint("D:\\Steam Library\\Blender"),
    "D:\\Steam Library\\Blender",
  );
  assert.equal(
    executableHint("C:\\Apps\\blender.exe,0"),
    "C:\\Apps\\blender.exe",
  );
});

test("Godot discovery rejects exported game templates and unrelated executables", () => {
  for (const file of [
    "godot.windows.template_release.x86_64.exe",
    "godot.windows.template_debug.dev.x86_64.exe",
    "godot_server.exe",
    "Game.exe",
    "godot-mcp.exe.cmd",
  ])
    assert.equal(isToolExecutable("godot", file), false, file);
  assert.ok(isToolExecutable("godot", "godot.windows.editor.x86_64.exe"));
  assert.ok(isToolExecutable("godot", "Godot_v4.7-stable_win64_console.exe"));
});

test("simple PATH launchers resolve targets without executing their shell contents", () => {
  assert.equal(
    launcherTarget(
      "godot",
      '@echo off\r\n"Z:\\project\\godot\\godot.exe" %*',
      "C:\\bin\\godot.cmd",
      {},
    ),
    "Z:\\project\\godot\\godot.exe",
  );
  assert.equal(
    launcherTarget(
      "godot",
      '@"%~dp0tools\\godot.exe" %*',
      "C:\\bin\\godot.cmd",
      {},
    ),
    "C:\\bin\\tools\\godot.exe",
  );
  assert.equal(
    launcherTarget(
      "blender",
      'call "%BLENDER_PATH%" %*',
      "C:\\bin\\blender.cmd",
      { blender_path: "D:\\Blender\\blender.exe" },
    ),
    "D:\\Blender\\blender.exe",
  );
  assert.equal(
    launcherTarget(
      "godot",
      'powershell -Command "anything"',
      "C:\\bin\\godot.cmd",
      {},
    ),
    undefined,
  );
});

test("registered Steam install roots and portable editor hints skip stale entries and templates", async () => {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), "beaver-discovery-"));
  try {
    const steam = path.join(
      root,
      "Steam Library",
      "steamapps",
      "common",
      "Blender",
    );
    const portable = path.join(root, "Godot 4", "bin");
    await fs.mkdir(steam, { recursive: true });
    await fs.mkdir(portable, { recursive: true });
    await fs.writeFile(path.join(steam, "blender.exe"), "fixture");
    const gui = path.join(portable, "godot.windows.editor.x86_64.exe");
    const console = path.join(
      portable,
      "godot.windows.editor.x86_64.console.exe",
    );
    const template = path.join(
      portable,
      "godot.windows.template_release.x86_64.exe",
    );
    for (const file of [gui, console, template])
      await fs.writeFile(file, "fixture");
    assert.equal(
      await resolveToolHints("blender", [
        path.join(root, "missing.exe"),
        steam,
      ]),
      path.join(steam, "blender.exe"),
    );
    assert.equal(await resolveToolHints("godot", [template, console]), gui);
    assert.equal(
      await resolveToolHints("godot", [path.dirname(portable)]),
      gui,
    );
    assert.equal(await resolveToolHints("godot", [template]), undefined);
    if (process.platform === "win32") {
      await assert.rejects(findTool("godot", template), /编辑器程序/);
      assert.equal(await findTool("godot", gui), gui);
      const renamed = path.join(root, "my-editor.exe");
      await fs.writeFile(renamed, "fixture");
      assert.equal(await findTool("godot", renamed), renamed);
    }
  } finally {
    await fs.rm(root, { recursive: true, force: true });
  }
});
