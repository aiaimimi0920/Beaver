import assert from "node:assert/strict";
import { spawn, spawnSync } from "node:child_process";
import { randomBytes } from "node:crypto";
import fs from "node:fs/promises";
import path from "node:path";
import net from "node:net";
import { createInterface } from "node:readline";

async function main() {
  const executable = path.resolve(process.argv[2] ?? "target/debug/Beaver.exe");
  const root = path.resolve(`output/validation/native-business-${Date.now()}`);
  await fs.mkdir(root, { recursive: true });
  const port = await new Promise<number>((resolve, reject) => {
    const server = net.createServer();
    server.on("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      assert(address && typeof address === "object");
      server.close(() => resolve(address.port));
    });
  });
  const token = randomBytes(32).toString("hex");
  const env = {
    ...process.env,
    BEAVER_DATA_DIR: path.join(root, "data"),
    BEAVER_API_PORT: String(port),
    BEAVER_API_TOKEN: token,
    WEBVIEW2_USER_DATA_FOLDER: path.join(root, "webview"),
  };
  const app = spawn("rtk", ["proxy", executable], {
    env,
    windowsHide: true,
    stdio: "ignore",
  });
  let mcp: ReturnType<typeof spawn> | undefined;
  const checks: string[] = [];
  const base = `http://127.0.0.1:${port}`;
  const headers = {
    Authorization: `Bearer ${token}`,
    "Content-Type": "application/json",
  };
  async function call(method: string, input: unknown = {}) {
    const response = await fetch(`${base}/v1/call`, {
      method: "POST",
      headers,
      body: JSON.stringify({ method, input }),
      signal: AbortSignal.timeout(30000),
    });
    return {
      status: response.status,
      body: (await response.json()) as Record<string, unknown>,
    };
  }
  function object(value: unknown): Record<string, unknown> {
    assert(value && typeof value === "object" && !Array.isArray(value));
    return value as Record<string, unknown>;
  }
  function stop(child: ReturnType<typeof spawn> | undefined) {
    if (child?.pid && child.exitCode === null)
      spawnSync(
        "rtk",
        ["proxy", "taskkill", "/PID", String(child.pid), "/T", "/F"],
        { windowsHide: true, stdio: "ignore" },
      );
  }
  try {
    let ready = false;
    for (let i = 0; i < 120; i++) {
      try {
        const response = await fetch(`${base}/v1/capabilities`, {
          headers,
          signal: AbortSignal.timeout(1000),
        });
        if (response.ok) {
          ready = true;
          break;
        }
      } catch {
        /* Wait for the isolated native host. */
      }
      if (app.exitCode !== null)
        throw new Error("Native host exited before API startup");
      await new Promise((resolve) => setTimeout(resolve, 500));
    }
    assert(ready, "API did not become ready");
    assert.equal((await fetch(`${base}/v1/capabilities`)).status, 401);
    assert.equal(
      (
        await fetch(`${base}/v1/capabilities`, {
          headers: { ...headers, Origin: "http://localhost" },
        })
      ).status,
      401,
    );
    checks.push("unauthenticated and browser-origin access rejected");
    const capabilities = object(
      await (await fetch(`${base}/v1/capabilities`, { headers })).json(),
    );
    assert(Array.isArray(capabilities.tools));
    assert.equal(capabilities.tools.length, 52);
    for (const method of [
      "project.npr.install",
      "workflow.list",
      "workflow.run",
      "logs.query",
    ]) {
      assert(
        capabilities.tools.some(
          (tool: unknown) => object(tool).name === method,
        ),
      );
    }
    checks.push(
      "52 discoverable business methods including NPR workflows and logs",
    );
    assert.equal((await call("chooseDirectory")).status, 400);
    assert.equal((await call("asset.import", { id: "x" })).status, 400);
    assert.equal((await call("game.importTemplates")).status, 400);
    checks.push(
      "dialog-only and missing-path requests fail without human interaction",
    );
    const created = await call("project.create", {
      parent: root,
      name: "API Smoke",
      template: "blank",
    });
    assert.equal(created.body.ok, true, JSON.stringify(created.body));
    const project = object(created.body.result);
    const id = project.id;
    assert.equal(typeof id, "string");
    assert.equal(typeof project.path, "string");
    await fs.access(path.join(project.path as string, "project.godot"));
    checks.push("project creation writes a real Godot project");
    const state = object((await call("state")).body.result);
    assert(Array.isArray(state.projects));
    assert(state.projects.some((p: unknown) => object(p).id === id));
    checks.push("created project visible through shared persistent state");
    const workflows = object((await call("workflow.list", { id })).body.result);
    assert(Array.isArray(workflows.workflows));
    assert.equal(object(workflows.workflows[0]).enabled, false);
    assert.equal(
      (
        await call("workflow.run", {
          id,
          workflow: "npr-character",
          action: "inspect",
        })
      ).status,
      409,
    );
    checks.push(
      "blank projects advertise disabled NPR and reject uninstalled workflow execution",
    );
    assert.equal(
      (
        await call("document.save", {
          id,
          path: "api-proof.md",
          text: "# Created through API\n",
          revision: null,
        })
      ).body.ok,
      true,
    );
    const document = object(
      (await call("document.read", { id, path: "api-proof.md" })).body.result,
    );
    assert.equal(typeof document.revision, "string");
    assert.equal(
      (
        await call("document.save", {
          id,
          path: "api-proof.md",
          text: "# Updated\n",
          revision: document.revision,
        })
      ).body.ok,
      true,
    );
    assert.equal(
      (
        await call("document.save", {
          id,
          path: "api-proof.md",
          text: "stale",
          revision: document.revision,
        })
      ).status,
      409,
    );
    assert.equal(
      await fs.readFile(
        path.join(project.path as string, "api-proof.md"),
        "utf8",
      ),
      "# Updated\n",
    );
    checks.push(
      "document edits persist and stale revisions preserve newer content",
    );
    const source = path.join(root, "import-proof.md");
    await fs.writeFile(source, "# Imported without a dialog\n", "utf8");
    assert.equal(
      (await call("asset.import", { id, paths: [source] })).body.ok,
      true,
    );
    checks.push("explicit-path asset import executes without a file picker");
    assert.equal(
      (await call("document.read", { id, path: "../import-proof.md" })).status,
      409,
    );
    checks.push("relative-path traversal rejected by shared core");

    mcp = spawn("rtk", ["proxy", executable, "--business-mcp"], {
      env,
      windowsHide: true,
      stdio: "pipe",
    });
    assert(mcp.stdin && mcp.stdout);
    const pending = new Map<
      number,
      {
        resolve: (v: Record<string, unknown>) => void;
        reject: (e: Error) => void;
        timer: NodeJS.Timeout;
      }
    >();
    let sequence = 0;
    const lines = createInterface({ input: mcp.stdout });
    lines.on("line", (line) => {
      const response = object(JSON.parse(line));
      const entry = pending.get(response.id as number);
      if (entry) {
        clearTimeout(entry.timer);
        pending.delete(response.id as number);
        entry.resolve(response);
      }
    });
    async function rpc(method: string, params: unknown = {}) {
      const id = ++sequence;
      return new Promise<Record<string, unknown>>((resolve, reject) => {
        const timer = setTimeout(() => {
          pending.delete(id);
          reject(new Error(`MCP timeout: ${method}`));
        }, 30000);
        pending.set(id, { resolve, reject, timer });
        mcp!.stdin!.write(
          `${JSON.stringify({ jsonrpc: "2.0", id, method, params })}\n`,
        );
      });
    }
    assert(
      object(
        (
          await rpc("initialize", {
            protocolVersion: "2025-11-25",
            capabilities: {},
            clientInfo: { name: "beaver-smoke", version: "1" },
          })
        ).result,
      ).capabilities,
    );
    mcp.stdin.write(
      `${JSON.stringify({ jsonrpc: "2.0", method: "notifications/initialized" })}\n`,
    );
    const listed = object((await rpc("tools/list")).result);
    assert(Array.isArray(listed.tools));
    assert.equal(listed.tools.length, 52);
    const mcpState = object(
      (await rpc("tools/call", { name: "state", arguments: {} })).result,
    );
    assert.equal(mcpState.isError, false);
    assert.deepEqual(
      object(object(mcpState.structuredContent).result).projects,
      state.projects,
    );
    checks.push(
      "real stdio MCP initialization, discovery and shared project state",
    );
    const edited = object(
      (
        await rpc("tools/call", {
          name: "document.save",
          arguments: {
            id,
            path: "mcp-proof.md",
            text: "# MCP\n",
            revision: null,
          },
        })
      ).result,
    );
    assert.equal(edited.isError, false);
    assert.equal(
      await fs.readFile(
        path.join(project.path as string, "mcp-proof.md"),
        "utf8",
      ),
      "# MCP\n",
    );
    checks.push("MCP mutation writes through the same native business service");
    const invalid = object(
      (await rpc("tools/call", { name: "asset.import", arguments: { id } }))
        .result,
    );
    assert.equal(invalid.isError, true);
    checks.push("MCP business validation failures are structured tool errors");
    const logResult = object(
      (
        await rpc("tools/call", {
          name: "logs.query",
          arguments: { projectId: id, limit: 100 },
        })
      ).result,
    );
    assert.equal(logResult.isError, false);
    const logPage = object(object(logResult.structuredContent).result);
    assert(Array.isArray(logPage.records));
    const records = logPage.records.map(object);
    assert(
      records.some(
        (row) =>
          row.method === "project.create" &&
          row.projectId === id &&
          row.status === "succeeded",
      ),
    );
    assert(
      records.some(
        (row) =>
          row.source === "api" &&
          row.method === "document.save" &&
          row.status === "failed",
      ),
    );
    assert(
      records.some(
        (row) =>
          row.source === "mcp" &&
          row.method === "document.save" &&
          row.status === "succeeded",
      ),
    );
    assert(
      records.some(
        (row) => row.method === "workflow.run" && row.status === "failed",
      ),
    );
    assert(!JSON.stringify(logPage).includes("# MCP"));
    assert(!JSON.stringify(logPage).includes(token));
    assert.equal((await call("logs.query", { limit: 0 })).status, 409);
    checks.push(
      "API and MCP logs correlate project calls, preserve failures, and omit raw content and token",
    );
    mcp.stdin.end();
    lines.close();
    const proof = {
      executable,
      root,
      checks,
      liveModelUsed: false,
      credentialsRecorded: false,
    };
    await fs.writeFile(
      path.join(root, "proof.json"),
      `${JSON.stringify(proof, null, 2)}\n`,
      "utf8",
    );
    console.log(JSON.stringify(proof, null, 2));
  } finally {
    stop(mcp);
    stop(app);
  }
}

void main().catch((error: unknown) => {
  console.error(error);
  process.exitCode = 1;
});
