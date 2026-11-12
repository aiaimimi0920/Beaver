import { requireValue, sha256 } from "./contract.mjs";

export const scope = "beaver:agent";
export const now = () => Math.floor(Date.now() / 1000);
export function secret() {
  return Array.from(crypto.getRandomValues(new Uint8Array(32)), (b) =>
    b.toString(16).padStart(2, "0"),
  ).join("");
}
export function config(env) {
  const origin = new URL(env.OAUTH_ISSUER);
  requireValue(
    origin.protocol === "https:" && origin.href === origin.origin + "/",
    "Invalid OAuth issuer",
    503,
  );
  requireValue(
    /^[a-f0-9]{64}$/.test(env.OAUTH_OWNER_KEY_SHA256 ?? ""),
    "OAuth owner not configured",
    503,
  );
  const clients = JSON.parse(env.OAUTH_CLIENTS_JSON ?? "{}");
  return { issuer: origin.origin, resource: origin.origin + "/mcp", clients };
}
export function json(body, status = 200, extra = {}) {
  return Response.json(body, {
    status,
    headers: { "Cache-Control": "no-store", Pragma: "no-cache", ...extra },
  });
}
export async function form(request) {
  requireValue(
    request.headers.get("content-type")?.split(";")[0] ===
      "application/x-www-form-urlencoded",
    "invalid_request",
  );
  const reader = request.body?.getReader();
  requireValue(reader, "invalid_request");
  let text = "",
    size = 0;
  const decoder = new TextDecoder("utf-8", { fatal: true });
  while (true) {
    const { value, done } = await reader.read();
    if (done) break;
    size += value.length;
    if (size > 8192) {
      await reader.cancel();
      requireValue(false, "invalid_request", 413);
    }
    text += decoder.decode(value, { stream: true });
  }
  text += decoder.decode();
  return unique(new URLSearchParams(text));
}
export function unique(params) {
  for (const key of params.keys())
    requireValue(params.getAll(key).length === 1, "invalid_request");
  return Object.fromEntries(params);
}
export function client(config, id, redirect) {
  const record = config.clients[id];
  requireValue(
    Object.hasOwn(config.clients, id ?? "") &&
      record &&
      Array.isArray(record.redirect_uris),
    "invalid_client",
  );
  if (redirect !== undefined) {
    requireValue(
      record.redirect_uris.includes(redirect),
      "invalid_redirect_uri",
    );
    const url = new URL(redirect);
    requireValue(
      url.protocol === "https:" && !url.username && !url.password && !url.hash,
      "invalid_redirect_uri",
    );
  }
  return record;
}
export async function challenge(verifier) {
  const hex = await sha256(verifier);
  return btoa(
    String.fromCharCode(...hex.match(/../g).map((x) => parseInt(x, 16))),
  )
    .replaceAll("+", "-")
    .replaceAll("/", "_")
    .replace(/=+$/, "");
}
export async function oauthRole(request, env, digest) {
  if (!env.OAUTH_ISSUER) return null;
  const cfg = config(env);
  if (new URL(request.url).pathname !== "/mcp") return null;
  const row = await env.DB.prepare(
    "SELECT * FROM oauth_tokens WHERE hash = ? AND revoked = 0 AND expires > ?",
  )
    .bind(digest, now())
    .first();
  return row &&
    row.resource === cfg.resource &&
    row.scope === scope &&
    Object.hasOwn(cfg.clients, row.client_id)
    ? "dot"
    : null;
}
