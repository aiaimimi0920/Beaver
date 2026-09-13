import test from "node:test";
import assert from "node:assert/strict";
import http from "node:http";
import fs from "node:fs/promises";
import path from "node:path";
import os from "node:os";
import readline from "node:readline";
import { spawn } from "node:child_process";
import { terminate } from "../src/core/process";
import { defaultSettings, capabilities } from "../src/shared/types";
import { Preferences } from "../src/core/settings";
import { Store } from "../src/core/store";

test("every AI capability resolves both local and platform credentials without mixing them", async () => {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), "beaver-provider-"));
  const store = new Store(root);
  try {
    const prefs = new Preferences(store, {
      encrypt: (s) => s,
      decrypt: (s) => s,
    });
    const settings = defaultSettings();
    settings.cloud.baseUrl = "https://platform.example/v1";
    for (const c of capabilities) {
      settings.local[c].baseUrl = `https://${c}.example/v1`;
      settings.local[c].model = `local-${c}`;
      settings.cloudModels[c] = `cloud-${c}`;
    }
    prefs.save(
      settings,
      Object.fromEntries([
        ...capabilities.map((c) => [c, `fixture-${c}`]),
        ["cloud", "fixture-cloud"],
      ]),
    );
    for (const c of capabilities) {
      assert.equal(prefs.resolve(c).model, `local-${c}`);
      assert.equal(prefs.resolve(c).key, `fixture-${c}`);
    }
    settings.mode = "cloud";
    prefs.save(settings, {});
    for (const c of capabilities) {
      assert.equal(prefs.resolve(c).baseUrl, "https://platform.example/v1");
      assert.equal(prefs.resolve(c).model, `cloud-${c}`);
      assert.equal(prefs.resolve(c).key, "fixture-cloud");
    }
  } finally {
    store.close();
    await fs.rm(root, { recursive: true, force: true });
  }
});

test("media MCP stdio routes references, audio and translation through explicit fixture providers", async () => {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), "beaver-media-"));
  const png = Buffer.from(
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+jWZkAAAAASUVORK5CYII=",
    "base64",
  );
  const routes: string[] = [];
  const server = http.createServer(async (req, res) => {
    for await (const _chunk of req) {
      /* Drain fixture input. */
    }
    routes.push(req.url ?? "");
    assert.equal(req.headers.authorization, "Bearer fixture-key");
    if (req.url?.endsWith("responses"))
      res
        .setHeader("Content-Type", "application/json")
        .end(JSON.stringify({ output_text: "fixture translation" }));
    else if (req.url?.includes("speech"))
      res
        .setHeader("Content-Type", "audio/wav")
        .end(Buffer.from("RIFF-fixture-audio"));
    else
      res
        .setHeader("Content-Type", "application/json")
        .end(JSON.stringify({ data: [{ b64_json: png.toString("base64") }] }));
  });
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
  const address = server.address();
  if (!address || typeof address === "string")
    throw new Error("Fixture server did not start");
  const providers = Object.fromEntries(
    ["image", "speech", "music", "translation"].map((c) => [
      c,
      {
        baseUrl: `http://127.0.0.1:${address.port}/v1`,
        model: "fixture",
        key: "fixture-key",
        route: c === "translation" ? "/responses" : `/${c}-custom`,
      },
    ]),
  );
  await fs.writeFile(path.join(root, "reference.png"), png);
  const child = spawn(
    process.execPath,
    ["--import", "tsx", "src/mcp/server.ts"],
    {
      cwd: process.cwd(),
      windowsHide: true,
      detached: process.platform !== "win32",
      stdio: "pipe",
      env: {
        ...process.env,
        BEAVER_PROJECT_ROOT: root,
        BEAVER_MEDIA_PROVIDERS: JSON.stringify(providers),
      },
    },
  );
  child.stderr.resume();
  let sequence = 0;
  const pending = new Map<number, (r: unknown) => void>();
  const lines = readline.createInterface({ input: child.stdout });
  lines.on("line", (line) => {
    const r = JSON.parse(line) as { id: number; result: unknown };
    pending.get(r.id)?.(r.result);
    pending.delete(r.id);
  });
  const request = async (
    method: string,
    params = {},
  ): Promise<{
    isError?: boolean;
    content?: { text: string }[];
    tools?: unknown[];
  }> => {
    const id = ++sequence;
    let timer: NodeJS.Timeout | undefined;
    try {
      return await new Promise((resolve, reject) => {
        timer = setTimeout(
          () => reject(new Error("MCP fixture timeout")),
          10000,
        );
        pending.set(id, (r) =>
          resolve(r as Awaited<ReturnType<typeof request>>),
        );
        child.stdin.write(
          JSON.stringify({ jsonrpc: "2.0", id, method, params }) + "\n",
        );
      });
    } finally {
      clearTimeout(timer);
    }
  };
  try {
    await request("initialize", { protocolVersion: "2024-11-05" });
    assert.equal((await request("tools/list")).tools?.length, 4);
    for (const [name, output, references] of [
      ["generate_image", "made.png", ["reference.png"]],
      ["generate_speech", "made.wav", undefined],
      ["generate_music", "music.bin", undefined],
    ] as const) {
      const result = await request("tools/call", {
        name,
        arguments: { prompt: "fixture", output, references },
      });
      assert.ok(!result.isError, JSON.stringify(result));
      assert.ok((await fs.stat(path.join(root, output))).size > 0);
    }
    const translated = await request("tools/call", {
      name: "translate_text",
      arguments: { prompt: "fixture" },
    });
    assert.match(translated.content?.[0]?.text ?? "", /fixture translation/);
    assert.deepEqual(routes, [
      "/v1/image-custom",
      "/v1/speech-custom",
      "/v1/music-custom",
      "/v1/responses",
    ]);
    const bad = await request("tools/call", {
      name: "generate_image",
      arguments: { prompt: "fixture", output: "../escape.png" },
    });
    assert.equal(bad.isError, true);
    assert.equal(
      routes.length,
      4,
      "Invalid output must be rejected before a billable request",
    );
  } finally {
    lines.close();
    await terminate(child);
    server.closeAllConnections();
    await new Promise<void>((resolve) => server.close(() => resolve()));
    await fs.rm(root, { recursive: true, force: true });
  }
});
