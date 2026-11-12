import { readJSON, requireValue, sha256 } from "./contract.mjs";
import { ExchangeStore } from "./store.mjs";
import { rpc } from "./mcp.mjs";
import { oauth } from "./oauth.mjs";
import { oauthRole } from "./oauth-common.mjs";

function json(value, status = 200) {
  return Response.json(value, {
    status,
    headers: {
      "Cache-Control": "no-store",
      "X-Content-Type-Options": "nosniff",
    },
  });
}
async function authenticate(request, env) {
  requireValue(
    /^[a-f0-9]{64}$/.test(env.BEAVER_TOKEN_SHA256 ?? "") &&
      /^[a-f0-9]{64}$/.test(env.DOT_TOKEN_SHA256 ?? "") &&
      env.BEAVER_TOKEN_SHA256 !== env.DOT_TOKEN_SHA256,
    "Authentication not configured",
    503,
  );
  const auth = request.headers.get("authorization") ?? "";
  requireValue(/^Bearer [!-~]{32,512}$/.test(auth), "Unauthorized", 401);
  const digest = await sha256(auth.slice(7));
  if (digest === env.BEAVER_TOKEN_SHA256) return "beaver";
  if (digest === env.DOT_TOKEN_SHA256) return "dot";
  const role = await oauthRole(request, env, digest);
  if (role) return role;
  requireValue(false, "Unauthorized", 401);
}
export default {
  async fetch(request, env) {
    try {
      const url = new URL(request.url);
      const authorization = await oauth(request, env);
      if (authorization) return authorization;
      requireValue(
        !request.headers.has("origin"),
        "Browser-origin requests are disabled",
        403,
      );
      const role = await authenticate(request, env);
      const store = new ExchangeStore(env.DB);
      if (url.pathname === "/mcp" && request.method === "POST") {
        requireValue(role === "dot", "Forbidden", 403);
        const result = await rpc(store, await readJSON(request));
        return result === null
          ? new Response(null, { status: 202 })
          : json(result);
      }
      if (url.pathname === "/v1/exchanges") {
        if (request.method === "POST") {
          requireValue(role === "beaver", "Forbidden", 403);
          return json(await store.publish(await readJSON(request)));
        }
        if (request.method === "GET") {
          requireValue(role === "dot", "Forbidden", 403);
          return json(await store.pending());
        }
      }
      const match =
        /^\/v1\/exchanges\/([A-Za-z0-9_.:-]{1,160})(?:\/(reply|receipt))?$/.exec(
          url.pathname,
        );
      if (match) {
        const [, id, action] = match;
        if (!action && request.method === "GET")
          return json(await store.get(id));
        if (action && request.method === "POST") {
          requireValue(
            role === (action === "reply" ? "dot" : "beaver"),
            "Forbidden",
            403,
          );
          return json(await store[action](id, await readJSON(request)));
        }
      }
      return json({ error: "Not found" }, 404);
    } catch (error) {
      if (error.status === 401 && env.OAUTH_ISSUER) {
        return Response.json(
          { error: "Unauthorized" },
          {
            status: 401,
            headers: {
              "Cache-Control": "no-store",
              "WWW-Authenticate": `Bearer resource_metadata="${env.OAUTH_ISSUER}/.well-known/oauth-protected-resource/mcp"`,
            },
          },
        );
      }
      return json(
        { error: error.status ? error.message : "Relay unavailable" },
        error.status ?? 500,
      );
    }
  },
};
