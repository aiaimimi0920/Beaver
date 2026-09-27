import assert from "node:assert/strict";
import { spawn, spawnSync } from "node:child_process";
import fs from "node:fs/promises";
import path from "node:path";
import { createInterface } from "node:readline";
import { z } from "zod";
import { ObjectGenerationSession } from "../src/ui/object-preview/object-generation-session";

const proofSchema = z.object({
  root: z.string(),
  executable: z.string(),
  pid: z.number().int().positive(),
  port: z.number().int().positive(),
  projectId: z.string(),
  priorProjects: z.literal(0),
  priorTasks: z.literal(0),
  template: z.literal("blank"),
  exeSha256: z.string(),
});
const envelopeSchema = z.object({
  apiVersion: z.literal("1"),
  ok: z.boolean(),
  result: z.unknown().optional(),
  error: z.object({ code: z.string(), message: z.string() }).optional(),
});

async function main() {
  const proofPath = process.env.BEAVER_START_PROOF;
  assert(
    proofPath,
    "Use start-fresh-native-test.ps1 and set BEAVER_START_PROOF",
  );
  const proof = proofSchema.parse(
    JSON.parse(await fs.readFile(proofPath, "utf8")),
  );
  const token = process.env.BEAVER_API_TOKEN;
  assert(
    token,
    "BEAVER_API_TOKEN must be inherited, not passed as an argument",
  );
  const base = `http://127.0.0.1:${proof.port}`;
  const headers = {
    Authorization: `Bearer ${token}`,
    "Content-Type": "application/json",
  };
  const checks: string[] = [];
  const calls: string[] = [];
  async function http(method: string, input: unknown = {}) {
    calls.push(method);
    const response = await fetch(`${base}/v1/call`, {
      method: "POST",
      headers,
      body: JSON.stringify({ method, input }),
      signal: AbortSignal.timeout(30000),
    });
    return {
      status: response.status,
      body: envelopeSchema.parse(await response.json()),
    };
  }
  async function api(method: string, input: unknown) {
    const result = await http(method, input);
    assert.equal(result.body.ok, true, JSON.stringify(result.body));
    return result.body.result;
  }
  const mcp = spawn(proof.executable, ["--business-mcp"], {
    env: { ...process.env, BEAVER_API_PORT: String(proof.port) },
    windowsHide: true,
    stdio: ["pipe", "pipe", "ignore"],
  });
  const lines = createInterface({ input: mcp.stdout });
  const pending = new Map<
    number,
    {
      resolve: (value: unknown) => void;
      reject: (error: Error) => void;
      timer: NodeJS.Timeout;
    }
  >();
  let sequence = 0;
  lines.on("line", (line) => {
    const value = z
      .object({
        id: z.number(),
        result: z.unknown().optional(),
        error: z.unknown().optional(),
      })
      .parse(JSON.parse(line));
    const item = pending.get(value.id);
    if (!item) return;
    clearTimeout(item.timer);
    pending.delete(value.id);
    if (value.error) item.reject(new Error(JSON.stringify(value.error)));
    else item.resolve(value.result);
  });
  function failPending() {
    for (const item of pending.values()) {
      clearTimeout(item.timer);
      item.reject(new Error("MCP process closed"));
    }
    pending.clear();
  }
  mcp.on("error", failPending);
  mcp.on("exit", failPending);
  function rpc(method: string, params: unknown = {}): Promise<unknown> {
    const id = ++sequence;
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        pending.delete(id);
        reject(new Error("MCP request timed out"));
      }, 30000);
      pending.set(id, { resolve, reject, timer });
      mcp.stdin.write(
        JSON.stringify({ jsonrpc: "2.0", id, method, params }) + "\n",
      );
    });
  }
  async function tool(name: string, args: unknown) {
    const result = z
      .object({ isError: z.boolean(), structuredContent: envelopeSchema })
      .parse(await rpc("tools/call", { name, arguments: args }));
    assert.equal(result.isError, !result.structuredContent.ok);
    return result.structuredContent;
  }
  try {
    assert.equal((await fetch(`${base}/v1/capabilities`)).status, 401);
    assert.equal(
      (
        await fetch(`${base}/v1/capabilities`, {
          headers: { ...headers, Origin: "http://localhost" },
        })
      ).status,
      401,
    );
    checks.push(
      "real native HTTP rejects missing credentials and browser origins",
    );
    const initialized = z
      .object({ protocolVersion: z.string() })
      .parse(await rpc("initialize", { protocolVersion: "2025-11-25" }));
    assert.equal(initialized.protocolVersion, "2025-11-25");
    const listed = z
      .object({ tools: z.array(z.object({ name: z.string() })) })
      .parse(await rpc("tools/list"));
    for (const name of [
      "objectTask.commit",
      "objectTask.publicationPreview",
      "objectTask.deferCandidateFeedback",
      "objectTask.publishCandidate",
    ]) {
      assert(
        listed.tools.some((entry) => entry.name === name),
        name,
      );
    }
    checks.push(
      "native stdio MCP negotiates version and discovers object workflow commands",
    );
    const values = new Map<string, string>();
    const storage = {
      getItem: (key: string) => values.get(key) ?? null,
      setItem: (key: string, value: string) => {
        values.set(key, value);
      },
    };
    const session = new ObjectGenerationSession(
      proof.projectId,
      api,
      storage,
      "generation-transport",
    );
    session.edit({
      name: "Transport object",
      category: "模型",
      prompt: "Create a small tree",
      acceptance: "A readable tree silhouette",
    });
    await session.submit();
    const saved = session.getSnapshot();
    assert(!saved.error, saved.error);
    assert(saved.saved.receipt, "Generation did not commit");
    const receipt = saved.saved.receipt;
    const snapshotInput = { projectId: proof.projectId };
    const snapshot = await api("objectTask.snapshot", snapshotInput);
    const mcpSnapshot = await tool("objectTask.snapshot", snapshotInput);
    assert.equal(mcpSnapshot.ok, true);
    assert.deepEqual(mcpSnapshot.result, snapshot);
    const replay = await tool("objectTask.commit", {
      projectId: proof.projectId,
      requestId: receipt.requestId,
      draftId: receipt.draftId,
      expectedDraftRevision: receipt.draftRevision,
      expectedPlanRevision: receipt.previousPlanRevision,
    });
    assert.equal(replay.ok, true, JSON.stringify(replay));
    assert.deepEqual(replay.result, receipt);
    assert.deepEqual(await api("objectTask.snapshot", snapshotInput), snapshot);
    const reopened = new ObjectGenerationSession(proof.projectId, api, storage);
    assert.equal(reopened.getSnapshot().error, "");
    assert.deepEqual(reopened.getSnapshot().saved.receipt, receipt);
    checks.push(
      "production generation session commits through HTTP; MCP exact retry returns same receipt without duplicate tasks",
    );
    const tasks = z
      .object({
        tasks: z.array(z.object({ status: z.string() })),
        runs: z.array(z.object({ status: z.string() })),
      })
      .parse(snapshot);
    assert.equal(tasks.tasks.length, 2);
    assert(tasks.tasks.every((task) => task.status === "planned"));
    assert.equal(tasks.runs.length, 1);
    assert.equal(tasks.runs[0]?.status, "planned");
    const catalog = z
      .array(z.object({ id: z.string() }))
      .parse(await api("object.list", snapshotInput));
    assert.deepEqual(
      catalog.map((entry) => entry.id),
      receipt.objectIds,
    );
    checks.push(
      "native project storage exposes exactly one object and two planned tasks without execution",
    );
    const foreign = { projectId: "missing-transport-project" };
    const rejectedHttp = await http("objectTask.snapshot", foreign);
    const rejectedMcp = await tool("objectTask.snapshot", foreign);
    assert.equal(rejectedHttp.status, 409);
    assert.equal(rejectedHttp.body.ok, false);
    assert.deepEqual(rejectedMcp, rejectedHttp.body);
    checks.push(
      "HTTP and MCP reject the same unregistered project at the shared core boundary",
    );
    await fs.writeFile(
      path.join(proof.root, "object-transport-proof.json"),
      JSON.stringify(
        {
          startProof: proofPath,
          executable: proof.executable,
          exeSha256: proof.exeSha256,
          checks,
          calls,
          receipt,
          snapshot,
          scope:
            "Real HTTP and stdio MCP; no Tauri UI invocation, engine, model, or publication acceptance",
        },
        null,
        2,
      ),
      "utf8",
    );
    console.log(
      JSON.stringify(
        { proof: path.join(proof.root, "object-transport-proof.json"), checks },
        null,
        2,
      ),
    );
  } finally {
    lines.close();
    mcp.stdin.end();
    if (mcp.exitCode === null) mcp.kill();
    failPending();
    // Only close the host identified by the fresh-round proof.
    const shutdown = spawnSync(
      "powershell.exe",
      [
        "-NoProfile",
        "-Command",
        `$p = Get-Process -Id ${proof.pid} -ErrorAction SilentlyContinue; if ($p -and $p.Path -eq $env:BEAVER_PROOF_EXE) { if ($p.CloseMainWindow()) { [void]$p.WaitForExit(10000) }; if (!$p.HasExited) { Stop-Process -Id $p.Id } }`,
      ],
      {
        env: { ...process.env, BEAVER_PROOF_EXE: proof.executable },
        windowsHide: true,
      },
    );
    assert.equal(shutdown.status, 0, "Owned host cleanup failed");
  }
}
main().catch((error: unknown) => {
  console.error(error);
  process.exitCode = 1;
});
