import assert from "node:assert/strict";
import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { codeStructureInstructions, prepareHome } from "../src/core/codex-home";
import { Preferences } from "../src/core/settings";
import { Store } from "../src/core/store";
import { defaultSettings } from "../src/shared/types";

test("bundled structure rules reach fresh and reused task homes without replacing project rules", async () => {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), "beaver-structure-"));
  const store = new Store(path.join(root, "data"));
  const prefs = new Preferences(store, {
    encrypt: (value) => Buffer.from(value).toString("base64"),
    decrypt: (value) => Buffer.from(value, "base64").toString(),
  });
  try {
    const settings = defaultSettings();
    settings.local.code = {
      baseUrl: "http://127.0.0.1:9231/v1",
      model: "fixture",
      route: "",
    };
    settings.tools = {
      codex: process.execPath,
      node: process.execPath,
      godot: process.execPath,
      blender: process.execPath,
    };
    settings.mcp = { godot: false, blender: false };
    prefs.save(settings, { code: "fixture-code-only" });
    const workspace = path.join(root, "workspace");
    await fs.mkdir(workspace);
    await fs.writeFile(
      path.join(workspace, "AGENTS.md"),
      "# User project instructions\n",
    );
    const task = {
      id: "structure-test",
      workspace,
      capability: "code" as const,
    };
    const resources = path.resolve("resources");
    const rule = await codeStructureInstructions(resources);
    assert.match(rule, /100-250/);
    assert.match(rule, /above 700/);
    const env = await prepareHome(
      root,
      resources,
      "fixture-mcp.cjs",
      task,
      prefs,
    );
    const home = env.CODEX_HOME!;
    assert.notEqual(home, process.env.CODEX_HOME);
    assert.ok(
      (await fs.readFile(path.join(home, "AGENTS.md"), "utf8")).endsWith(rule),
    );
    await fs.mkdir(path.join(home, "sessions"));
    await fs.writeFile(
      path.join(home, "sessions", "retained.jsonl"),
      "retained",
    );
    await fs.writeFile(
      path.join(home, "AGENTS.md"),
      "# Old application rules\n",
    );
    await prepareHome(root, resources, "fixture-mcp.cjs", task, prefs);
    assert.ok(
      (await fs.readFile(path.join(home, "AGENTS.md"), "utf8")).endsWith(rule),
    );
    assert.equal(
      await fs.readFile(path.join(home, "sessions", "retained.jsonl"), "utf8"),
      "retained",
    );
    assert.equal(
      await fs.readFile(path.join(workspace, "AGENTS.md"), "utf8"),
      "# User project instructions\n",
    );
    assert.ok(
      !(await fs.readFile(path.join(home, "config.toml"), "utf8")).includes(
        "fixture-code-only",
      ),
    );
  } finally {
    store.close();
    await fs.rm(root, { recursive: true, force: true });
  }
});
