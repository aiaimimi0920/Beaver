import { test } from "node:test";
import assert from "node:assert/strict";
import { DatabaseSync } from "node:sqlite";
import { readFileSync } from "node:fs";
import worker from "./worker.mjs";
import { sha256 } from "./contract.mjs";
import { RelayClient, applyExchange } from "./adapter.mjs";

const beaverToken = "b".repeat(40),
  dotToken = "d".repeat(40);
const request = {
  id: "exchange-1",
  taskId: "task-1",
  runId: "run-1",
  revision: 1,
  prompt: "创建测试场景",
  context: { status: "active" },
};
const reply = {
  method: "external.tool",
  arguments: {
    tool: "file.write",
    arguments: { path: "test.txt", content: "你好" },
  },
};
async function fixture(t) {
  const db = new DatabaseSync(":memory:");
  t.after(() => db.close());
  db.exec(readFileSync(new URL("schema.sql", import.meta.url), "utf8"));
  const env = {
    BEAVER_TOKEN_SHA256: await sha256(beaverToken),
    DOT_TOKEN_SHA256: await sha256(dotToken),
    DB: {
      prepare(sql) {
        let args = [];
        return {
          bind(...values) {
            args = values;
            return this;
          },
          async first() {
            return db.prepare(sql).get(...args) ?? null;
          },
          async run() {
            return db.prepare(sql).run(...args);
          },
          async all() {
            return { results: db.prepare(sql).all(...args) };
          },
        };
      },
    },
  };
  const transport = (url, options) =>
    worker.fetch(new Request(url, options), env);
  const client = new RelayClient(
    "https://relay.example",
    beaverToken,
    transport,
  );
  const call = (role, path, body) =>
    transport("https://relay.example" + path, {
      method: body === undefined ? "GET" : "POST",
      headers: {
        authorization: `Bearer ${role === "dot" ? dotToken : beaverToken}`,
        "content-type": "application/json",
      },
      body: body === undefined ? undefined : JSON.stringify(body),
    });
  return { client, call, transport, env };
}
test("real SQL message roundtrip, role boundaries, immutable replay", async (t) => {
  const { client, call, transport } = await fixture(t);
  assert.equal(
    (await transport("https://relay.example/v1/exchanges", {})).status,
    401,
  );
  assert.equal((await call("dot", "/v1/exchanges", request)).status, 403);
  await client.publish(request);
  await client.publish({ ...request, context: { status: "active" } });
  assert.equal(
    (await call("beaver", "/v1/exchanges", { ...request, prompt: "changed" }))
      .status,
    409,
  );
  assert.equal(
    (await call("beaver", "/v1/exchanges/exchange-1/reply", reply)).status,
    403,
  );
  assert.equal(
    (await call("dot", "/v1/exchanges/exchange-1/receipt", {})).status,
    403,
  );
  assert.equal(
    (await call("dot", "/v1/exchanges/exchange-1/reply", reply)).status,
    200,
  );
  assert.equal(
    (await call("dot", "/v1/exchanges/exchange-1/reply", reply)).status,
    200,
  );
  assert.equal(
    (
      await call("dot", "/v1/exchanges/exchange-1/reply", {
        ...reply,
        arguments: {},
      })
    ).status,
    409,
  );
  const calls = [];
  const core = {
    async call(method, input) {
      calls.push({ method, input });
      return { status: "completed", revision: 2 };
    },
  };
  assert.equal(
    (await applyExchange(core, client, request)).status,
    "acknowledged",
  );
  await applyExchange(core, client, request);
  assert.equal(calls.length, 1);
  assert.deepEqual(calls[0], {
    method: reply.method,
    input: {
      taskId: request.taskId,
      runId: request.runId,
      revision: 1,
      requestId: request.id,
      arguments: reply.arguments,
    },
  });
  assert.deepEqual((await client.get(request.id)).receipt, {
    status: "completed",
    revision: 2,
  });
});
test("MCP exposes structured replies and pending tasks, not arbitrary methods", async (t) => {
  const { client, call } = await fixture(t);
  await client.publish(request);
  const rpc = async (method, params) =>
    (
      await call("dot", "/mcp", { jsonrpc: "2.0", id: 1, method, params })
    ).json();
  assert.equal(
    (await rpc("initialize", { protocolVersion: "2025-06-18" })).result
      .serverInfo.name,
    "beaver-cloudflare-relay",
  );
  assert.equal((await rpc("tools/list")).result.tools.length, 3);
  assert.equal(
    (await rpc("tools/call", { name: "beaver_pending" })).result
      .structuredContent.exchanges[0].id,
    request.id,
  );
  assert.ok(
    (
      await rpc("tools/call", {
        name: "beaver_reply",
        arguments: {
          id: request.id,
          reply: { method: "settings.save", arguments: {} },
        },
      })
    ).error,
  );
  assert.ok(
    (
      await rpc("tools/call", {
        name: "beaver_reply",
        arguments: { id: request.id, reply },
      })
    ).result,
  );
  assert.equal(
    (await rpc("tools/call", { name: "beaver_pending" })).result
      .structuredContent.exchanges.length,
    0,
  );
});
test("lost acknowledgement replays exact Core identity; mismatched local grant never executes", async (t) => {
  const { client, call } = await fixture(t);
  await client.publish(request);
  await call("dot", "/v1/exchanges/exchange-1/reply", reply);
  const calls = [];
  const core = {
    async call(method, input) {
      calls.push(input);
      return { status: "completed" };
    },
  };
  const broken = {
    get: (id) => client.get(id),
    receipt: async () => {
      throw new Error("offline");
    },
  };
  await assert.rejects(applyExchange(core, broken, request), /offline/);
  await applyExchange(core, client, request);
  assert.deepEqual(calls[0], calls[1]);
  await assert.rejects(
    applyExchange(core, client, { ...request, taskId: "another-task" }),
    /local authorization/,
  );
  assert.equal(calls.length, 2);
});
test("reject oversized input, missing auth configuration, and browser origins", async (t) => {
  const { call, env } = await fixture(t);
  assert.equal(
    (
      await call("beaver", "/v1/exchanges", {
        ...request,
        prompt: "x".repeat(524288),
      })
    ).status,
    413,
  );
  assert.equal(
    (await call("beaver", "/v1/exchanges", { ...request, revision: -1 }))
      .status,
    400,
  );
  assert.equal(
    (
      await worker.fetch(
        new Request("https://relay.example", {
          headers: { origin: "https://evil.example" },
        }),
        env,
      )
    ).status,
    403,
  );
  assert.equal(
    (await worker.fetch(new Request("https://relay.example"), {})).status,
    503,
  );
});
