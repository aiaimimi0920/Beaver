import { test } from "node:test";
import assert from "node:assert/strict";
import { DatabaseSync } from "node:sqlite";
import { readFileSync } from "node:fs";
import worker from "./worker.mjs";
import { sha256 } from "./contract.mjs";
import { proveOAuth } from "./oauth-proof.mjs";
import { challenge } from "./oauth-common.mjs";

async function fixture(t) {
  const db = new DatabaseSync(":memory:");
  t.after(() => db.close());
  for (const file of ["schema.sql", "oauth-schema.sql"])
    db.exec(readFileSync(new URL(file, import.meta.url), "utf8"));
  const env = {
    OAUTH_ISSUER: "https://relay.example",
    OAUTH_OWNER_KEY_SHA256: await sha256("owner".repeat(16)),
    OAUTH_CLIENTS_JSON: JSON.stringify({
      "beaver-relay-proof": {
        name: "Proof",
        redirect_uris: ["https://example.com/beaver-oauth-proof"],
      },
    }),
    BEAVER_TOKEN_SHA256: await sha256("b".repeat(40)),
    DOT_TOKEN_SHA256: await sha256("d".repeat(40)),
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
  return {
    db,
    env,
    transport: (url, options) => worker.fetch(new Request(url, options), env),
  };
}
test("OAuth browser consent to MCP access, PKCE, denial, replay and revocation", async (t) => {
  const { transport } = await fixture(t);
  assert.equal(
    (await proveOAuth("https://relay.example", "owner".repeat(16), transport))
      .oauth,
    true,
  );
});
test("expired tokens, wrong audience and removed clients fail closed", async (t) => {
  const { db, transport, env } = await fixture(t);
  const access = "x".repeat(64),
    hash = await sha256(access);
  const request = () =>
    transport("https://relay.example/mcp", {
      method: "POST",
      headers: {
        authorization: "Bearer " + access,
        "content-type": "application/json",
      },
      body: JSON.stringify({ jsonrpc: "2.0", id: 1, method: "ping" }),
    });
  db.prepare("INSERT INTO oauth_tokens VALUES (?, ?, ?, ?, ?, 0)").run(
    hash,
    "beaver-relay-proof",
    "https://relay.example/mcp",
    "beaver:agent",
    1,
  );
  assert.equal((await request()).status, 401);
  db.prepare("UPDATE oauth_tokens SET expires = ?, resource = ?").run(
    Math.floor(Date.now() / 1000) + 60,
    "https://other.example/mcp",
  );
  assert.equal((await request()).status, 401);
  db.prepare("UPDATE oauth_tokens SET resource = ?").run(
    "https://relay.example/mcp",
  );
  assert.equal((await request()).status, 200);
  env.OAUTH_CLIENTS_JSON = "{}";
  assert.equal((await request()).status, 401);
});

test("wrong owner key cannot issue codes; expired code cannot be redeemed", async (t) => {
  const { db, transport } = await fixture(t);
  const payload = {
    client_id: "beaver-relay-proof",
    redirect_uri: "https://example.com/beaver-oauth-proof",
    resource: "https://relay.example/mcp",
    scope: "beaver:agent",
    state: "state",
    code_challenge: await challenge("v".repeat(64)),
  };
  db.prepare("INSERT INTO oauth_flows VALUES (?, ?, ?, ?, 0)").run(
    "flow",
    await sha256("c".repeat(64)),
    JSON.stringify(payload),
    Math.floor(Date.now() / 1000) + 60,
  );
  const response = await transport("https://relay.example/oauth/consent", {
    method: "POST",
    headers: {
      origin: "https://relay.example",
      cookie: "__Host-beaver_oauth=" + "c".repeat(64),
      "content-type": "application/x-www-form-urlencoded",
    },
    body: new URLSearchParams({
      flow: "flow",
      decision: "allow",
      owner_key: "wrong",
    }).toString(),
  });
  assert.equal(response.status, 403);
  assert.equal(db.prepare("SELECT count(*) AS n FROM oauth_codes").get().n, 0);
  assert.equal(db.prepare("SELECT used FROM oauth_flows").get().used, 1);
  db.prepare("INSERT INTO oauth_codes VALUES (?, ?, ?, 0)").run(
    await sha256("code"),
    JSON.stringify(payload),
    1,
  );
  const expired = await transport("https://relay.example/oauth/token", {
    method: "POST",
    headers: { "content-type": "application/x-www-form-urlencoded" },
    body: new URLSearchParams({
      ...payload,
      code: "code",
      code_verifier: "v".repeat(64),
      grant_type: "authorization_code",
    }).toString(),
  });
  assert.equal(expired.status, 400);
  assert.equal((await expired.json()).error, "invalid_grant");
});
