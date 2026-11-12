import { requireValue, sha256 } from "./contract.mjs";
import { client, unique, scope, secret, now, form } from "./oauth-common.mjs";

const escape = (value) =>
  String(value).replace(
    /[&<>"']/g,
    (c) =>
      ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[
        c
      ],
  );
const cookieName = "__Host-beaver_oauth";
export async function authorize(request, env, cfg) {
  const input = unique(new URL(request.url).searchParams);
  const record = client(cfg, input.client_id, input.redirect_uri);
  requireValue(
    input.redirect_uri && input.response_type === "code",
    "invalid_request",
  );
  requireValue(input.resource === cfg.resource, "invalid_target");
  requireValue(input.scope === scope, "invalid_scope");
  requireValue(
    input.code_challenge_method === "S256" &&
      /^[A-Za-z0-9_-]{43}$/.test(input.code_challenge ?? ""),
    "invalid_request",
  );
  requireValue(
    typeof input.state === "string" &&
      input.state.length >= 1 &&
      input.state.length <= 1024,
    "invalid_request",
  );
  const id = secret(),
    browser = secret();
  await env.DB.prepare(
    "INSERT INTO oauth_flows (id, browser_hash, payload, expires) VALUES (?, ?, ?, ?)",
  )
    .bind(id, await sha256(browser), JSON.stringify(input), now() + 300)
    .run();
  const html = `<!doctype html><html lang="zh-CN"><meta charset="utf-8"><meta name="viewport" content="width=device-width"><title>Beaver 连接授权</title>
<body><main><h1>连接 Beaver 测试中转服务</h1><p>客户端：<strong>${escape(record.name ?? input.client_id)}</strong></p>
<p>回调地址：${escape(input.redirect_uri)}</p><p>权限：读取此测试通道中的所有任务及回执，并提交命令供 Beaver 执行。仅批准你信任的客户端。</p>
<p>请输入本服务所有者密钥，不是 OpenAI 密码。连接有效期为 1 小时。</p>
<form method="post" action="/oauth/consent"><input type="hidden" name="flow" value="${id}">
<label>服务所有者密钥 <input name="owner_key" type="password" autocomplete="off" maxlength="128"></label>
<button name="decision" value="allow">登录并授权</button><button name="decision" value="deny">拒绝</button></form></main></body></html>`;
  return new Response(html, {
    headers: {
      "Content-Type": "text/html; charset=utf-8",
      "Cache-Control": "no-store",
      "Referrer-Policy": "no-referrer",
      "Content-Security-Policy":
        "default-src 'none'; form-action 'self'; frame-ancestors 'none'; base-uri 'none'",
      "X-Frame-Options": "DENY",
      "X-Content-Type-Options": "nosniff",
      "Set-Cookie": `${cookieName}=${browser}; Secure; HttpOnly; SameSite=Lax; Path=/; Max-Age=300`,
    },
  });
}
export async function consent(request, env, cfg) {
  requireValue(
    request.headers.get("origin") === cfg.issuer,
    "invalid_request",
    403,
  );
  const input = await form(request);
  const cookie = request.headers
    .get("cookie")
    ?.split(";")
    .map((x) => x.trim())
    .find((x) => x.startsWith(cookieName + "="))
    ?.slice(cookieName.length + 1);
  requireValue(cookie && /^[a-f0-9]{64}$/.test(cookie), "invalid_request", 403);
  const row = await env.DB.prepare(
    "SELECT * FROM oauth_flows WHERE id = ? AND used = 0 AND expires > ?",
  )
    .bind(input.flow ?? "", now())
    .first();
  requireValue(
    row && row.browser_hash === (await sha256(cookie)),
    "invalid_request",
    403,
  );
  requireValue(["allow", "deny"].includes(input.decision), "invalid_request");
  const data = JSON.parse(row.payload);
  client(cfg, data.client_id, data.redirect_uri);
  // Consume before checking the key: a failed login requires a new consent flow.
  const consumed = await env.DB.prepare(
    "UPDATE oauth_flows SET used = 1 WHERE id = ? AND used = 0 RETURNING id",
  )
    .bind(input.flow)
    .first();
  requireValue(consumed, "invalid_request");
  const redirect = new URL(data.redirect_uri);
  redirect.searchParams.set("state", data.state);
  redirect.searchParams.set("iss", cfg.issuer);
  if (input.decision === "deny")
    redirect.searchParams.set("error", "access_denied");
  else {
    requireValue(
      (await sha256(input.owner_key ?? "")) === env.OAUTH_OWNER_KEY_SHA256,
      "access_denied",
      403,
    );
    const code = secret();
    await env.DB.prepare(
      "INSERT INTO oauth_codes (hash, payload, expires) VALUES (?, ?, ?)",
    )
      .bind(await sha256(code), row.payload, now() + 60)
      .run();
    redirect.searchParams.set("code", code);
  }
  return new Response(null, {
    status: 303,
    headers: {
      Location: redirect.href,
      "Cache-Control": "no-store",
      "Referrer-Policy": "no-referrer",
      "Set-Cookie": `${cookieName}=; Secure; HttpOnly; SameSite=Lax; Path=/; Max-Age=0`,
    },
  });
}
