import assert from "node:assert/strict";
import fs from "node:fs/promises";
import path from "node:path";
import http from "node:http";
import readline from "node:readline";
import { spawn } from "node:child_process";

async function main() {
  const sourceExecutable = path.resolve(
    process.argv[2] ?? "target/debug/Beaver.exe",
  );
  const directory = path.resolve(
    "output/validation",
    `native-media-${Date.now()}`,
  );
  const workspace = path.join(directory, "project");
  const application = path.join(directory, "application");
  const executable = path.join(application, path.basename(sourceExecutable));
  await fs.mkdir(application, { recursive: true });
  await fs.copyFile(sourceExecutable, executable, fs.constants.COPYFILE_EXCL);
  await fs.mkdir(workspace, { recursive: true });
  await fs.writeFile(
    path.join(workspace, "参考.png"),
    Buffer.from([137, 80, 78, 71]),
  );
  const requests: {
    url: string;
    authorization: string | undefined;
    body: string;
  }[] = [];
  const server = http.createServer(async (request, response) => {
    const chunks: Buffer[] = [];
    for await (const chunk of request) chunks.push(Buffer.from(chunk));
    const body = Buffer.concat(chunks).toString("utf8");
    requests.push({
      url: request.url ?? "",
      authorization: request.headers.authorization,
      body,
    });
    if (body.includes("oversized-response")) {
      response.writeHead(200, {
        "Content-Type": "application/octet-stream",
        "Content-Length": 200 * 1024 * 1024,
      });
      response.end();
      return;
    }
    if (body.includes("http-error")) {
      response.writeHead(401);
      response.end("dummy-api-secret");
      return;
    }
    if (body.includes("redirect")) {
      response.writeHead(302, { Location: "/credential-trap" });
      response.end();
      return;
    }
    if (request.url?.endsWith("/audio/speech")) {
      response.writeHead(200, { "Content-Type": "audio/wav" });
      response.end("RIFF-native-audio");
      return;
    }
    response.setHeader("Content-Type", "application/json");
    if (request.url?.endsWith("/responses")) {
      response.end(JSON.stringify({ output_text: "翻译完成" }));
      return;
    }
    if (body.includes("insecure-download")) {
      response.end(
        JSON.stringify({ data: [{ url: "http://127.0.0.1/media" }] }),
      );
      return;
    }
    response.end(
      JSON.stringify({
        data: [
          {
            b64_json: Buffer.from("native-generated-media").toString("base64"),
          },
        ],
      }),
    );
  });
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
  const address = server.address();
  assert(address && typeof address !== "string");
  const baseUrl = `http://127.0.0.1:${address.port}/v1`;
  const provider = (route: string) => ({
    baseUrl,
    model: "fixture-model",
    route,
    key: "dummy-api-secret",
  });
  const child = spawn(executable, ["--media-mcp"], {
    windowsHide: true,
    env: {
      ...process.env,
      BEAVER_DATA_DIR: path.join(directory, "unexpected-desktop-data"),
      WEBVIEW2_USER_DATA_FOLDER: path.join(
        directory,
        "unexpected-webview-data",
      ),
      BEAVER_PROJECT_ROOT: workspace,
      BEAVER_MEDIA_PROVIDERS: JSON.stringify({
        image: provider("/images/generations"),
        speech: provider("/audio/speech"),
        music: provider("/music"),
        translation: provider(""),
      }),
    },
    stdio: ["pipe", "pipe", "pipe"],
  });
  const pending = new Map<
    number,
    {
      resolve: (v: Record<string, unknown>) => void;
      reject: (e: Error) => void;
    }
  >();
  let sequence = 0;
  let errors = "";
  child.stderr.on("data", (chunk: Buffer) => {
    errors += chunk.toString();
  });
  const exited = new Promise<void>((resolve) =>
    child.once("exit", () => resolve()),
  );
  child.on("error", (error) => {
    for (const entry of pending.values()) entry.reject(error);
  });
  child.on("exit", () => {
    for (const entry of pending.values())
      entry.reject(new Error("MCP exited early"));
  });
  const lines = readline.createInterface({ input: child.stdout });
  lines.on("line", (line) => {
    const value: Record<string, unknown> = JSON.parse(line);
    const entry = pending.get(Number(value.id));
    if (entry) {
      pending.delete(Number(value.id));
      entry.resolve(value);
    }
  });
  async function rpc(method: string, params: unknown = {}) {
    const id = ++sequence;
    let timer: ReturnType<typeof setTimeout> | undefined;
    try {
      return await new Promise<Record<string, unknown>>((resolve, reject) => {
        pending.set(id, { resolve, reject });
        timer = setTimeout(() => reject(new Error("MCP timeout")), 20000);
        child.stdin.write(
          JSON.stringify({ jsonrpc: "2.0", id, method, params }) + "\n",
        );
      });
    } finally {
      clearTimeout(timer);
      pending.delete(id);
    }
  }
  const checks: string[] = [];
  async function call(name: string, args: unknown, failure = false) {
    const response = await rpc("tools/call", { name, arguments: args });
    const result = response.result as {
      isError?: boolean;
      content: { text: string }[];
    };
    assert.equal(!!result.isError, failure, JSON.stringify(result));
    return result.content[0]!.text;
  }
  try {
    assert((await rpc("initialize", { protocolVersion: "2024-11-05" })).result);
    const list = (await rpc("tools/list")).result as {
      tools: { name: string }[];
    };
    assert.equal(list.tools.length, 6);
    assert(list.tools.some((tool) => tool.name === "beaver_workflow_run"));
    const workflows = JSON.parse(await call("beaver_workflow_list", {})) as {
      workflows: { enabled: boolean }[];
    };
    assert.equal(workflows.workflows[0]?.enabled, false);
    await call(
      "beaver_workflow_run",
      { workflow: "npr-character", action: "inspect" },
      true,
    );
    checks.push(
      "native stdio initialize, media/workflow discovery and disabled-package rejection",
    );
    await call("generate_image", {
      prompt: "original image",
      output: "assets/图像.bin",
    });
    assert.equal(
      await fs.readFile(path.join(workspace, "assets/图像.bin"), "utf8"),
      "native-generated-media",
    );
    checks.push("image base64 output");
    await call("generate_image", {
      prompt: "edit reference",
      output: "assets/edited.bin",
      references: ["参考.png"],
    });
    assert.equal(requests.at(-1)?.url, "/v1/images/edits");
    assert(requests.at(-1)?.body.includes('name="image[]"'));
    checks.push("multipart reference edit route");
    await call("generate_speech", {
      prompt: "你好",
      output: "assets/voice.wav",
    });
    assert.equal(
      await fs.readFile(path.join(workspace, "assets/voice.wav"), "utf8"),
      "RIFF-native-audio",
    );
    assert.equal(JSON.parse(requests.at(-1)!.body).voice, "alloy");
    checks.push("binary speech and default voice");
    await call("generate_music", {
      prompt: "music",
      output: "assets/music.bin",
    });
    checks.push("custom music route");
    assert.equal(
      JSON.parse(await call("translate_text", { prompt: "translate" }))
        .output_text,
      "翻译完成",
    );
    checks.push("translation Responses route");
    const before = requests.length;
    await call(
      "generate_image",
      { prompt: "overwrite", output: "assets/图像.bin" },
      true,
    );
    await call(
      "generate_image",
      { prompt: "escape", output: "../escape.bin" },
      true,
    );
    await call(
      "generate_image",
      {
        prompt: "reference escape",
        output: "assets/escape.bin",
        references: ["../outside.png"],
      },
      true,
    );
    assert.equal(requests.length, before);
    checks.push("overwrite and traversal rejected before HTTP");
    const failed = await call(
      "generate_image",
      { prompt: "http-error", output: "assets/failure.bin" },
      true,
    );
    assert(failed.includes("401"));
    assert(!failed.includes("dummy-api-secret"));
    checks.push("HTTP error excludes provider body and credential");
    await call(
      "generate_image",
      { prompt: "redirect", output: "assets/redirect.bin" },
      true,
    );
    assert(!requests.some((r) => r.url === "/credential-trap"));
    checks.push("credential-bearing request redirects refused");
    await call(
      "generate_image",
      { prompt: "insecure-download", output: "assets/download.bin" },
      true,
    );
    checks.push("insecure media URL refused");
    await call(
      "generate_image",
      { prompt: "oversized-response", output: "assets/too-large.bin" },
      true,
    );
    checks.push("oversized response refused");
    const races = await Promise.all(
      [1, 2].map(() =>
        rpc("tools/call", {
          name: "generate_image",
          arguments: { prompt: "race", output: "assets/race.bin" },
        }),
      ),
    );
    assert.equal(
      races.filter((value) => !(value.result as { isError?: boolean }).isError)
        .length,
      1,
    );
    assert.equal(
      await fs.readFile(path.join(workspace, "assets/race.bin"), "utf8"),
      "native-generated-media",
    );
    checks.push("concurrent output collision preserves one complete asset");
    await call("unknown", { prompt: "invalid" }, true);
    checks.push("unknown tool rejected");
    assert(
      requests.every((r) => r.authorization === "Bearer dummy-api-secret"),
    );
    assert.equal(errors, "");
    checks.push("configured key used without stderr leakage");
    child.stdin.end();
    await Promise.race([
      exited,
      new Promise<never>((_, reject) => {
        const t = setTimeout(
          () => reject(new Error("MCP failed to exit on EOF")),
          5000,
        );
        t.unref();
      }),
    ]);
    assert.equal(child.exitCode, 0);
    checks.push("EOF clean exit");
    assert.deepEqual(await fs.readdir(application), [
      path.basename(executable),
    ]);
    for (const name of ["unexpected-desktop-data", "unexpected-webview-data"]) {
      await assert.rejects(fs.access(path.join(directory, name)), {
        code: "ENOENT",
      });
    }
    checks.push(
      "a lone Beaver executable serves MCP without a sidecar or desktop data initialization",
    );
    await fs.writeFile(
      path.join(directory, "proof.json"),
      JSON.stringify(
        {
          sourceExecutable,
          executable,
          args: ["--media-mcp"],
          checks,
          passed: true,
          exitCode: child.exitCode,
        },
        null,
        2,
      ),
    );
    console.log(directory, `${checks.length} checks passed`);
  } finally {
    if (child.exitCode === null) child.kill();
    lines.close();
    server.closeAllConnections();
    await new Promise<void>((resolve) => server.close(() => resolve()));
  }
}
void main();
