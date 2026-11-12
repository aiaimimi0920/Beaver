import { requireValue, sha256 } from "./contract.mjs";
import {
  config,
  client,
  scope,
  now,
  secret,
  json,
  form,
  challenge,
} from "./oauth-common.mjs";
import { authorize, consent } from "./oauth-consent.mjs";

async function token(request, env, cfg) {
  const input = await form(request);
  requireValue(
    !request.headers.has("authorization") && !input.client_secret,
    "invalid_client",
  );
  client(cfg, input.client_id, input.redirect_uri);
  requireValue(
    input.grant_type === "authorization_code",
    "unsupported_grant_type",
  );
  requireValue(input.resource === cfg.resource, "invalid_target");
  requireValue(
    /^[A-Za-z0-9._~-]{43,128}$/.test(input.code_verifier ?? ""),
    "invalid_grant",
  );
  const hash = await sha256(input.code ?? "");
  const row = await env.DB.prepare(
    "SELECT * FROM oauth_codes WHERE hash = ? AND used = 0 AND expires > ?",
  )
    .bind(hash, now())
    .first();
  requireValue(row, "invalid_grant");
  const data = JSON.parse(row.payload);
  requireValue(
    data.client_id === input.client_id &&
      data.redirect_uri === input.redirect_uri &&
      data.resource === input.resource &&
      data.code_challenge === (await challenge(input.code_verifier)),
    "invalid_grant",
  );
  const consumed = await env.DB.prepare(
    "UPDATE oauth_codes SET used = 1 WHERE hash = ? AND used = 0 AND expires > ? RETURNING hash",
  )
    .bind(hash, now())
    .first();
  requireValue(consumed, "invalid_grant");
  const access = secret();
  await env.DB.prepare(
    "INSERT INTO oauth_tokens (hash, client_id, resource, scope, expires) VALUES (?, ?, ?, ?, ?)",
  )
    .bind(
      await sha256(access),
      input.client_id,
      cfg.resource,
      scope,
      now() + 3600,
    )
    .run();
  return json({
    access_token: access,
    token_type: "Bearer",
    expires_in: 3600,
    scope,
  });
}
async function revoke(request, env, cfg) {
  const input = await form(request);
  client(cfg, input.client_id);
  await env.DB.prepare(
    "UPDATE oauth_tokens SET revoked = 1 WHERE hash = ? AND client_id = ?",
  )
    .bind(await sha256(input.token ?? ""), input.client_id)
    .run();
  return json({});
}
export async function oauth(request, env) {
  const url = new URL(request.url);
  if (
    !url.pathname.startsWith("/oauth/") &&
    !url.pathname.startsWith("/.well-known/")
  )
    return null;
  try {
    const cfg = config(env);
    requireValue(url.origin === cfg.issuer, "invalid_request");
    if (
      request.method === "GET" &&
      [
        "/.well-known/oauth-protected-resource",
        "/.well-known/oauth-protected-resource/mcp",
      ].includes(url.pathname)
    )
      return json({
        resource: cfg.resource,
        authorization_servers: [cfg.issuer],
        scopes_supported: [scope],
        bearer_methods_supported: ["header"],
      });
    if (
      request.method === "GET" &&
      url.pathname === "/.well-known/oauth-authorization-server"
    )
      return json({
        issuer: cfg.issuer,
        authorization_endpoint: cfg.issuer + "/oauth/authorize",
        token_endpoint: cfg.issuer + "/oauth/token",
        revocation_endpoint: cfg.issuer + "/oauth/revoke",
        response_types_supported: ["code"],
        grant_types_supported: ["authorization_code"],
        token_endpoint_auth_methods_supported: ["none"],
        code_challenge_methods_supported: ["S256"],
        scopes_supported: [scope],
        authorization_response_iss_parameter_supported: true,
      });
    if (request.method === "GET" && url.pathname === "/oauth/authorize")
      return await authorize(request, env, cfg);
    if (request.method === "POST" && url.pathname === "/oauth/consent")
      return await consent(request, env, cfg);
    requireValue(!request.headers.has("origin"), "invalid_request", 403);
    if (request.method === "POST" && url.pathname === "/oauth/token")
      return await token(request, env, cfg);
    if (request.method === "POST" && url.pathname === "/oauth/revoke")
      return await revoke(request, env, cfg);
    return json({ error: "invalid_request" }, 404);
  } catch (error) {
    return json(
      { error: error.status ? error.message : "server_error" },
      error.status ?? 500,
    );
  }
}
