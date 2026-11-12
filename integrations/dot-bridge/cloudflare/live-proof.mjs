import assert from "node:assert/strict";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import { setTimeout as delay } from "node:timers/promises";
import { CoreTransport } from "../../../src/direct/transport.cjs";
import { RelayClient, prepareExchange, applyExchange } from "./adapter.mjs";

// Explicit development probe: simulates Dot's HTTP caller, never a model call.
const [origin, credentialsPath, binary, directory] = process.argv.slice(2);
if (!directory)
  throw new Error(
    "Usage: live-proof.mjs ORIGIN CREDENTIALS HEADLESS NEW_DIRECTORY",
  );
const credentials = JSON.parse(await readFile(credentialsPath, "utf8"));
await mkdir(directory, { recursive: false });
const data = resolve(directory, "data");
await mkdir(data);
const core = new CoreTransport(binary, data);
const relay = new RelayClient(origin, credentials.beaver);
async function dot(method, params) {
  const response = await fetch(origin + "/mcp", {
    method: "POST",
    redirect: "error",
    signal: AbortSignal.timeout(30000),
    headers: {
      authorization: `Bearer ${credentials.dot}`,
      "content-type": "application/json",
    },
    body: JSON.stringify({
      jsonrpc: "2.0",
      id: crypto.randomUUID(),
      method,
      params,
    }),
  });
  assert.equal(response.status, 200);
  const message = await response.json();
  assert.equal(message.error, undefined, JSON.stringify(message.error));
  return message.result;
}
const proof = { origin, actualDot: false, actualCore: true, rounds: [] };
try {
  assert.equal((await fetch(origin + "/v1/exchanges")).status, 401);
  await dot("initialize", { protocolVersion: "2025-06-18" });
  const project = await core.call("project.create", {
    parent: directory,
    name: "Relay Proof",
    template: "blank",
  });
  const task = await core.call("task.create", {
    projectId: project.id,
    prompt: "在隔离工作区创建 relay-proof.txt，内容为 Cloudflare 中转验证。",
    executionMode: "external-agent",
    capability: "code",
    decompose: false,
    autoAccept: false,
    askRatio: 0,
    maxMinutes: 10,
  });
  let ready = false;
  for (let i = 0; i < 60; i++) {
    const pending = await core.call("external.pending");
    if (pending.some((item) => item.taskId === task.id)) {
      ready = true;
      break;
    }
    await delay(500);
  }
  assert.ok(ready, "External task admission");
  for (const [tool, args] of [
    [
      "file.write",
      {
        path: "relay-proof.txt",
        content: "Cloudflare 中转验证",
        expectedSha256: null,
      },
    ],
    ["file.read", { path: "relay-proof.txt" }],
  ]) {
    const request = await prepareExchange(
      core,
      task.id,
      "仅用于消息中转开发验证，不调用模型。",
    );
    await relay.publish(request);
    const remote = (
      await dot("tools/call", {
        name: "beaver_get",
        arguments: { id: request.id },
      })
    ).structuredContent;
    assert.equal(remote.request.runId, request.runId);
    await dot("tools/call", {
      name: "beaver_reply",
      arguments: {
        id: request.id,
        reply: {
          method: "external.tool",
          arguments: { tool, arguments: args },
        },
      },
    });
    const result = await applyExchange(core, relay, request);
    assert.notEqual(
      result.receipt.status,
      "failed",
      JSON.stringify(result.receipt),
    );
    const readback = (
      await dot("tools/call", {
        name: "beaver_get",
        arguments: { id: request.id },
      })
    ).structuredContent;
    assert.deepEqual(readback.receipt, result.receipt);
    if (tool === "file.read")
      assert.match(JSON.stringify(result.receipt), /Cloudflare 中转验证/);
    proof.rounds.push({
      exchangeId: request.id,
      tool,
      receipt: result.receipt,
    });
  }
  proof.file = await readFile(
    resolve(
      (await core.call("external.context", { taskId: task.id })).workspace,
      "relay-proof.txt",
    ),
    "utf8",
  );
  assert.equal(proof.file, "Cloudflare 中转验证");
  await core.call("task.interrupt", { id: task.id });
  await writeFile(
    resolve(directory, "proof.json"),
    JSON.stringify(proof, null, 2),
    "utf8",
  );
  console.log(JSON.stringify(proof, null, 2));
} finally {
  if (!core.closed) await core.call("shutdown");
}
