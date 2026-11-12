import assert from "node:assert/strict";
import { challenge } from "./oauth-common.mjs";

// HTTP-level OAuth client, not a real Dot login. Never follows callback redirects.
export async function proveOAuth(origin, ownerKey, transport = fetch) {
  const send = (path, options = {}) =>
    transport(origin + path, { redirect: "manual", ...options });
  const post = (path, values, headers = {}) =>
    send(path, {
      method: "POST",
      headers: {
        "content-type": "application/x-www-form-urlencoded",
        ...headers,
      },
      body: new URLSearchParams(values).toString(),
    });
  const client_id = "beaver-relay-proof";
  const redirect_uri = "https://example.com/beaver-oauth-proof";
  const resource = origin + "/mcp";
  const verifier = "v".repeat(64);
  const parameters = {
    client_id,
    redirect_uri,
    resource,
    scope: "beaver:agent",
    response_type: "code",
    code_challenge_method: "S256",
    code_challenge: await challenge(verifier),
    state: crypto.randomUUID(),
  };
  const discovery = await send("/.well-known/oauth-authorization-server");
  assert.equal((await discovery.json()).issuer, origin);
  assert.equal((await send("/mcp", { method: "POST" })).status, 401);
  const badRedirect = await send(
    "/oauth/authorize?" +
      new URLSearchParams({
        ...parameters,
        redirect_uri: "https://evil.example/callback",
      }),
  );
  assert.equal(badRedirect.status, 400);
  async function start() {
    const page = await send(
      "/oauth/authorize?" + new URLSearchParams(parameters),
    );
    assert.equal(page.status, 200);
    const cookie = page.headers.get("set-cookie").split(";")[0];
    const html = await page.text();
    const flow = html.match(/name="flow" value="([a-f0-9]+)"/)[1];
    return { cookie, flow };
  }
  const denied = await start();
  const denial = await post(
    "/oauth/consent",
    { flow: denied.flow, decision: "deny" },
    { origin, cookie: denied.cookie },
  );
  assert.equal(
    new URL(denial.headers.get("location")).searchParams.get("error"),
    "access_denied",
  );
  const grant = await start();
  assert.equal(
    (
      await post(
        "/oauth/consent",
        { flow: grant.flow, decision: "allow", owner_key: ownerKey },
        { origin: "https://evil.example", cookie: grant.cookie },
      )
    ).status,
    403,
  );
  const consent = await post(
    "/oauth/consent",
    { flow: grant.flow, decision: "allow", owner_key: ownerKey },
    { origin, cookie: grant.cookie },
  );
  assert.equal(consent.status, 303);
  const callback = new URL(consent.headers.get("location"));
  assert.equal(callback.searchParams.get("state"), parameters.state);
  assert.equal(callback.searchParams.get("iss"), origin);
  const code = callback.searchParams.get("code");
  const exchange = {
    client_id,
    redirect_uri,
    resource,
    code,
    code_verifier: verifier,
    grant_type: "authorization_code",
  };
  assert.equal(
    (await post("/oauth/token", { ...exchange, code_verifier: "w".repeat(64) }))
      .status,
    400,
  );
  const response = await post("/oauth/token", exchange);
  assert.equal(response.status, 200);
  const token = await response.json();
  assert.equal((await post("/oauth/token", exchange)).status, 400);
  const headers = {
    authorization: "Bearer " + token.access_token,
    "content-type": "application/json",
  };
  const rpc = await send("/mcp", {
    method: "POST",
    headers,
    body: JSON.stringify({ jsonrpc: "2.0", id: 1, method: "tools/list" }),
  });
  assert.equal(rpc.status, 200);
  assert.equal((await rpc.json()).result.tools.length, 3);
  assert.equal((await send("/v1/exchanges", { headers })).status, 401);
  assert.equal(
    (await post("/oauth/revoke", { client_id, token: token.access_token }))
      .status,
    200,
  );
  assert.equal((await send("/mcp", { method: "POST", headers })).status, 401);
  return {
    actualDot: false,
    oauth: true,
    discovery: true,
    consent: true,
    pkce: true,
    codeReplayRejected: true,
    crossOriginRejected: true,
    invalidRedirectRejected: true,
    denied: true,
    mcp: true,
    roleRestricted: true,
    revocation: true,
  };
}
