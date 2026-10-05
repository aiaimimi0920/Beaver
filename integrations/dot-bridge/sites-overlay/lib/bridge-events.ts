import { createHash, createHmac, timingSafeEqual } from "node:crypto";
import {
  saveSubscription,
  subscriptions,
  recordDelivery,
  type Subscription,
  type Task,
} from "./bridge-db";
export const eventName = "beaver.demo.ready";
// Restricted pilot: only known public platform hosts, never arbitrary caller URLs.
const callbackHosts = new Set([
  "chatgpt.com",
  "api.chatgpt.com",
  "connectors.api.openai.com",
]);
function callbackURL(value: string) {
  const u = new URL(value);
  if (
    u.protocol !== "https:" ||
    u.port ||
    u.username ||
    u.password ||
    u.hash ||
    !callbackHosts.has(u.hostname)
  )
    throw Error("Callback host is not enabled for this restricted pilot");
  return u.href;
}
function equal(a: string, b: string) {
  const x = Buffer.from(a),
    y = Buffer.from(b);
  return x.length === y.length && timingSafeEqual(x, y);
}
async function send(s: Subscription, body: unknown, id: string) {
  const url = callbackURL(s.callback),
    raw = JSON.stringify(body),
    ts = String(Math.floor(Date.now() / 1000));
  const key = Buffer.from(s.secret.slice(6), "base64");
  if (!s.secret.startsWith("whsec_") || key.length < 24 || key.length > 64)
    throw Error("Invalid signing secret");
  const sig =
    "v1," +
    createHmac("sha256", key).update(`${id}.${ts}.${raw}`).digest("base64");
  const response = await fetch(url, {
    method: "POST",
    redirect: "manual",
    signal: AbortSignal.timeout(10000),
    headers: {
      "Content-Type": "application/json",
      "webhook-id": id,
      "webhook-timestamp": ts,
      "webhook-signature": sig,
      "X-MCP-Subscription-Id": s.id,
    },
    body: raw,
  });
  if (response.status >= 300 && response.status < 400)
    throw Error("Callback redirects are not allowed");
  return response;
}
export async function subscribe(
  owner: string,
  p: {
    name?: string;
    arguments?: { queue_id?: string };
    delivery?: { mode?: string; url?: string; secret?: string };
    ttlMs?: number | null;
  },
) {
  if (
    p.name !== eventName ||
    p.arguments?.queue_id !== "smoke-test" ||
    p.delivery?.mode !== "webhook"
  )
    throw Error("Unsupported event or queue");
  const callback = callbackURL(p.delivery.url ?? ""),
    secret = p.delivery.secret ?? "";
  const ttl = p.ttlMs ?? 3600000;
  if (!Number.isFinite(ttl) || ttl <= 0)
    throw Error("Invalid subscription lifetime");
  const id =
    "sub_" +
    createHash("sha256")
      .update(JSON.stringify([owner, callback, eventName, "smoke-test"]))
      .digest("hex")
      .slice(0, 32);
  const s = {
    id,
    owner,
    callback,
    secret,
    expires: Date.now() + Math.min(ttl, 3600000),
  };
  const challenge = crypto.randomUUID(),
    r = await send(
      s,
      { type: "verification", challenge },
      "verify_" + crypto.randomUUID(),
    );
  const answer = (await r.json()) as { challenge?: string };
  if (!r.ok || !equal(String(answer.challenge ?? ""), challenge))
    throw Error("Callback verification failed");
  await saveSubscription(s);
  return {
    id,
    refreshBefore: new Date(s.expires).toISOString(),
    cursor: null,
    truncated: false,
  };
}
export async function deliver(task: Task) {
  const all = await subscriptions();
  if (!all.length) {
    await recordDelivery(task.id, "no_active_subscription");
    return { delivered: false, reason: "no_active_subscription" };
  }
  const statuses = [];
  for (const s of all) {
    try {
      const r = await send(
        s,
        {
          eventId: task.event_id,
          name: eventName,
          timestamp: task.created_at,
          data: {
            queue_id: "smoke-test",
            request_id: task.id,
            summary: "A harmless Beaver bridge connectivity check is ready.",
          },
          cursor: null,
        },
        task.event_id,
      );
      statuses.push({
        subscription_id: s.id,
        status: r.status,
        received: r.ok,
      });
    } catch {
      statuses.push({ subscription_id: s.id, status: 0, received: false });
    }
  }
  await recordDelivery(task.id, JSON.stringify(statuses));
  return {
    deliveries: statuses,
    note: "Receipt is not proof of dot processing; verify the task result.",
  };
}
