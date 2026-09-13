import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { ToolSetup } from "../src/core/tool-setup";
import { managedCodex } from "../src/core/managed-codex";
import type { ToolName, ToolSetupState } from "../src/shared/tool-setup";
import { matchesToolVersion } from "../src/shared/tool-setup";

test("readiness verifies tool identity rather than accepting any successful executable", () => {
  assert.equal(matchesToolVersion("node", "v22.22.2"), true);
  assert.equal(matchesToolVersion("codex", "codex-cli 0.144.5"), true);
  assert.equal(
    matchesToolVersion("godot", "4.6.stable.custom_build.a8cacb6f2"),
    true,
  );
  assert.equal(matchesToolVersion("blender", "Blender 4.5.3"), true);
  assert.equal(matchesToolVersion("godot", "v22.22.2"), false);
  assert.equal(matchesToolVersion("godot", "3.6.stable"), false);
  assert.equal(matchesToolVersion("codex", ""), false);
});

function fixture(available = new Set<ToolName>()) {
  const installed: ToolName[] = [];
  const accepted: ToolName[] = [];
  let saved: ToolSetupState | undefined;
  const operations = {
    detect: async (name: ToolName, _signal: AbortSignal) => ({
      name,
      path: available.has(name) ? `/tools/${name}` : "",
      available: available.has(name),
      version: "fixture",
    }),
    install: async (name: ToolName, _signal: AbortSignal) => {
      installed.push(name);
      available.add(name);
    },
    accept: (tool: { name: ToolName }) => {
      accepted.push(tool.name);
    },
    save: (state: ToolSetupState) => {
      saved = state;
    },
    redact: (error: string) => error.replaceAll("SECRET", "[REDACTED]"),
  };
  return { operations, installed, accepted, available, saved: () => saved };
}

test("tool setup prepares Node before Codex, skips existing programs and persists verified paths", async () => {
  const f = fixture(new Set(["godot"]));
  const setup = new ToolSetup(f.operations);
  assert.equal((await setup.prepare()).status, "completed");
  assert.deepEqual(f.installed, ["node", "codex", "blender"]);
  assert.deepEqual(f.accepted, ["node", "codex", "godot", "blender"]);
  assert.equal(
    f.saved()?.steps.filter((step) => step.status === "ready").length,
    4,
  );
  await setup.prepare();
  assert.equal(f.installed.length, 3, "retry must not reinstall ready tools");
});

test("successful installer exit alone is not readiness; failed stages stop dependent work", async () => {
  const f = fixture();
  f.operations.install = async () => {};
  const setup = new ToolSetup(f.operations);
  const result = await setup.prepare();
  assert.equal(result.status, "failed");
  assert.match(result.error!, /安装后仍未通过/);
  assert.deepEqual(f.accepted, []);
  assert.equal(result.steps[1]?.status, "waiting");
});

test("cancel stops preparation, blocks duplicates and restart never auto-installs", async () => {
  const f = fixture(new Set(["node"]));
  let reached: () => void = () => {};
  const started = new Promise<void>((resolve) => {
    reached = resolve;
  });
  f.operations.install = async (_name, signal) => {
    reached();
    await new Promise<void>((_resolve, reject) =>
      signal.addEventListener("abort", () => reject(new Error("cancelled")), {
        once: true,
      }),
    );
  };
  const setup = new ToolSetup(f.operations);
  const running = setup.prepare(["codex"]);
  await started;
  await assert.rejects(setup.prepare(), /正在进行/);
  const interrupted = setup.read();
  setup.cancel();
  assert.equal((await running).status, "cancelled");
  assert.deepEqual(f.accepted, ["node"]);
  const restored = new ToolSetup(f.operations, interrupted);
  assert.equal(restored.read().status, "cancelled");
  assert.equal(restored.read().active, undefined);
});

test("setup errors are redacted before persistence", async () => {
  const f = fixture();
  f.operations.install = async () => {
    throw new Error("SECRET provider detail");
  };
  const setup = new ToolSetup(f.operations);
  await setup.prepare();
  assert.ok(!JSON.stringify(f.saved()).includes("SECRET"));
});

test("managed Codex discovery supports hoisted, nested and legacy layouts without global fallback", async () => {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), "beaver-codex-layout-"));
  try {
    assert.equal(await managedCodex(root, "x64"), undefined);
    for (const relative of [
      "node_modules/@openai/codex-win32-x64/vendor/x86_64-pc-windows-msvc/bin/codex.exe",
      "node_modules/@openai/codex/node_modules/@openai/codex-win32-x64/vendor/x86_64-pc-windows-msvc/codex/codex.exe",
      "node_modules/@openai/codex/vendor/x86_64-pc-windows-msvc/bin/codex.exe",
    ]) {
      const file = path.join(root, relative);
      await fs.mkdir(path.dirname(file), { recursive: true });
      await fs.writeFile(file, "fixture, not executable");
      assert.equal(await managedCodex(root, "x64"), file);
      await fs.unlink(file);
    }
  } finally {
    await fs.rm(root, { recursive: true, force: true });
  }
});
