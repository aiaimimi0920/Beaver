import test from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { Bridge, EVENT, verify, sign } from "./bridge.mjs";
import { openStore } from "./store.mjs";

const secret = "whsec_" + Buffer.alloc(32, 7).toString("base64"); // Deliberately fake fixture, never a live credential.
const params = {
  name: EVENT,
  arguments: { queue_id: "smoke-test" },
  delivery: {
    mode: "webhook",
    url: "https://receiver.example.test/callback",
    secret,
  },
};
async function fixture(t) {
  const dir = await mkdtemp(join(tmpdir(), "beaver-bridge-test-"));
  t.after(() => rm(dir, { recursive: true, force: true }));
  const path = join(dir, "state.json"),
    deliveries = [];
  const store = await openStore(path);
  const transport = async (_url, request) => {
    const h = request.headers;
    assert.ok(
      verify(
        secret,
        h["webhook-id"],
        h["webhook-timestamp"],
        request.body,
        h["webhook-signature"],
      ),
    );
    const body = JSON.parse(request.body);
    if (body.type === "verification")
      return { status: 200, body: { challenge: body.challenge } };
    deliveries.push(body);
    return { status: 202, body: {} };
  };
  return { path, store, deliveries, bridge: new Bridge({ store, transport }) };
}
test("Simulated receiver accepts event, then simulated dot returns marker", async (t) => {
  const f = await fixture(t);
  await f.bridge.subscribe("alice", params);
  const task = await f.bridge.createDemo("alice", { request_id: "demo001" });
  const receipt = await f.bridge.deliver("alice", task.request_id);
  assert.equal(receipt.deliveries[0].received, true);
  assert.equal(f.bridge.get("alice", task.request_id).state, "pending");
  assert.equal(f.deliveries[0].name, EVENT);
  const output = await f.bridge.rpc("alice", "tools/call", {
    name: "submit_demo_result",
    arguments: { request_id: task.request_id, result: "BEAVER_DOT_SMOKE_OK" },
  });
  assert.equal(output.structuredContent.state, "completed");
});
test("Duplicate requests and results are idempotent", async (t) => {
  const { bridge } = await fixture(t);
  const a = await bridge.createDemo("alice", { request_id: "same" });
  assert.equal(
    (await bridge.createDemo("alice", { request_id: "same" })).eventId,
    a.eventId,
  );
  const one = await bridge.result("alice", {
    request_id: "same",
    result: "BEAVER_DOT_SMOKE_OK",
  });
  assert.equal(
    (
      await bridge.result("alice", {
        request_id: "same",
        result: "BEAVER_DOT_SMOKE_OK",
      })
    ).completed_at,
    one.completed_at,
  );
});
test("Cross-owner task/result access is rejected", async (t) => {
  const { bridge } = await fixture(t);
  await bridge.createDemo("alice", { request_id: "private" });
  assert.throws(() => bridge.get("bob", "private"), /not found/);
  await assert.rejects(
    bridge.result("bob", {
      request_id: "private",
      result: "BEAVER_DOT_SMOKE_OK",
    }),
    /not found/,
  );
});
test("Tampered and old webhook signatures fail", () => {
  const ts = String(Math.floor(Date.now() / 1000)),
    sig = sign(secret, "e1", ts, "{}");
  assert.equal(verify(secret, "e1", ts, '{"changed":true}', sig), false);
  assert.equal(verify(secret, "e1", "0", "{}", sig), false);
});
test("Restart retains task and subscription state", async (t) => {
  const f = await fixture(t);
  await f.bridge.subscribe("alice", params);
  await f.bridge.createDemo("alice", { request_id: "persist" });
  const store = await openStore(f.path);
  assert.equal(Object.keys(store.data.subscriptions).length, 1);
  assert.equal(new Bridge({ store }).get("alice", "persist").state, "pending");
});
test("No live transport means fail closed, not a claimed delivery", async (t) => {
  const { store } = await fixture(t);
  await assert.rejects(
    new Bridge({ store }).subscribe("alice", params),
    /not configured/,
  );
  assert.equal(Object.keys(store.data.subscriptions).length, 0);
});
test("Failed callback challenge never activates subscription", async (t) => {
  const { store } = await fixture(t);
  const bridge = new Bridge({
    store,
    transport: async () => ({ status: 200, body: { challenge: "wrong" } }),
  });
  await assert.rejects(
    bridge.subscribe("alice", params),
    /verification failed/,
  );
  assert.equal(Object.keys(store.data.subscriptions).length, 0);
});
test("Expiry and unsubscribe stop event delivery", async (t) => {
  const f = await fixture(t);
  await f.bridge.subscribe("alice", params);
  await f.bridge.createDemo("alice", { request_id: "x" });
  await f.bridge.rpc("alice", "events/unsubscribe", params);
  assert.equal((await f.bridge.deliver("alice", "x")).deliveries.length, 0);
  await f.bridge.subscribe("alice", { ...params, ttlMs: 1 });
  f.bridge.now = () => Date.now() + 5000;
  assert.equal((await f.bridge.deliver("alice", "x")).deliveries.length, 0);
});
