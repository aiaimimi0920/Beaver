import {
  createHmac,
  timingSafeEqual,
  createHash,
  randomUUID,
} from "node:crypto";

export const EVENT = "beaver.demo.ready";
const schema = {
  type: "object",
  properties: { queue_id: { type: "string", const: "smoke-test" } },
  required: ["queue_id"],
  additionalProperties: false,
};
const stable = (value) => JSON.stringify(value, Object.keys(value).sort());
function equal(a, b) {
  const x = Buffer.from(a),
    y = Buffer.from(b);
  return x.length === y.length && timingSafeEqual(x, y);
}
export function sign(secret, id, timestamp, body) {
  const bytes = Buffer.from(secret.slice(6), "base64");
  if (!secret.startsWith("whsec_") || bytes.length < 24 || bytes.length > 64)
    throw Error("Invalid signing secret");
  return (
    "v1," +
    createHmac("sha256", bytes)
      .update(`${id}.${timestamp}.${body}`)
      .digest("base64")
  );
}
export function verify(
  secret,
  id,
  timestamp,
  body,
  signature,
  now = Date.now(),
) {
  if (
    !Number.isInteger(Number(timestamp)) ||
    Math.abs(now / 1000 - Number(timestamp)) > 300
  )
    return false;
  return equal(sign(secret, id, timestamp, body), signature);
}

// A transport must pin a freshly validated public address, enforce TLS and reject redirects.
// No production outbound transport is provided by this offline prototype.
export class Bridge {
  constructor({ store, transport, now = () => Date.now() }) {
    this.store = store;
    this.transport = transport;
    this.now = now;
  }
  async save() {
    await this.store.save();
  }
  async send(sub, event) {
    if (!this.transport)
      throw Error("Live callback transport is not configured");
    const body = JSON.stringify(event),
      timestamp = String(Math.floor(this.now() / 1000));
    return this.transport(sub.url, {
      method: "POST",
      redirect: "error",
      body,
      headers: {
        "Content-Type": "application/json",
        "webhook-id": event.eventId,
        "webhook-timestamp": timestamp,
        "webhook-signature": sign(sub.secret, event.eventId, timestamp, body),
        "X-MCP-Subscription-Id": sub.id,
      },
    });
  }
  async subscribe(owner, params) {
    if (
      params.name !== EVENT ||
      params.arguments?.queue_id !== "smoke-test" ||
      params.delivery?.mode !== "webhook"
    )
      throw Error("Unsupported subscription");
    const url = new URL(params.delivery.url);
    if (url.protocol !== "https:" || url.username || url.password || url.hash)
      throw Error("HTTPS callback required");
    const id =
      "sub_" +
      createHash("sha256")
        .update(
          stable({ owner, url: url.href, event: EVENT, queue: "smoke-test" }),
        )
        .digest("hex")
        .slice(0, 32);
    const sub = {
      id,
      owner,
      url: url.href,
      secret: params.delivery.secret,
      expires: this.now() + Math.min(params.ttlMs ?? 3600000, 3600000),
    };
    if (!Number.isFinite(sub.expires) || sub.expires <= this.now())
      throw Error("Invalid lifetime");
    const challenge = randomUUID();
    const response = await this.send(sub, {
      type: "verification",
      challenge,
      eventId: "verify_" + randomUUID(),
    });
    if (
      response.status < 200 ||
      response.status >= 300 ||
      !equal(String(response.body?.challenge ?? ""), challenge)
    )
      throw Error("Callback verification failed");
    this.store.data.subscriptions[id] = sub;
    await this.save();
    return {
      id,
      refreshBefore: new Date(sub.expires).toISOString(),
      cursor: null,
      truncated: false,
    };
  }
  async createDemo(owner, { request_id }) {
    if (!/^[a-zA-Z0-9_-]{1,80}$/.test(request_id ?? ""))
      throw Error("Invalid request ID");
    const key = owner + ":" + request_id;
    if (this.store.data.tasks[key]) return this.store.data.tasks[key];
    const task = {
      request_id,
      queue_id: "smoke-test",
      state: "pending",
      expected: "BEAVER_DOT_SMOKE_OK",
      eventId: "evt_" + randomUUID(),
      created_at: new Date(this.now()).toISOString(),
    };
    this.store.data.tasks[key] = task;
    await this.save();
    return task;
  }
  async deliver(owner, request_id) {
    const task = this.get(owner, request_id);
    const event = {
      eventId: task.eventId,
      name: EVENT,
      timestamp: task.created_at,
      data: {
        queue_id: "smoke-test",
        request_id,
        summary: "A harmless bridge connectivity check is ready.",
      },
      cursor: null,
    };
    const result = [];
    for (const sub of Object.values(this.store.data.subscriptions)) {
      if (sub.owner !== owner || sub.expires <= this.now()) continue;
      const response = await this.send(sub, event);
      result.push({
        subscription_id: sub.id,
        http_status: response.status,
        received: response.status >= 200 && response.status < 300,
      });
    }
    return {
      deliveries: result,
      note: "Webhook receipt is not proof that dot processed the task.",
    };
  }
  get(owner, request_id) {
    const task = this.store.data.tasks[owner + ":" + request_id];
    if (!task) throw Error("Task not found");
    return task;
  }
  async result(owner, { request_id, result }) {
    const task = this.get(owner, request_id);
    if (result !== task.expected) throw Error("Unexpected demo result");
    if (task.state === "completed") return task;
    task.state = "completed";
    task.result = result;
    task.completed_at = new Date(this.now()).toISOString();
    await this.save();
    return task;
  }
  async rpc(owner, method, params = {}) {
    if (method === "server/discover")
      return {
        resultType: "complete",
        supportedVersions: ["2026-07-28"],
        capabilities: { tools: {}, events: {} },
      };
    if (method === "initialize")
      return {
        protocolVersion: "2026-07-28",
        capabilities: { tools: {}, events: {} },
        serverInfo: { name: "beaver-dot-bridge-prototype", version: "0.0.1" },
      };
    if (method === "tools/list")
      return {
        tools: [
          {
            name: "bridge_status",
            description:
              "Read bridge status. This prototype does not invoke any model.",
            inputSchema: {
              type: "object",
              properties: {},
              additionalProperties: false,
            },
            annotations: { readOnlyHint: true },
          },
          {
            name: "get_demo_task",
            description: "Read a harmless smoke-test task.",
            inputSchema: {
              type: "object",
              properties: { request_id: { type: "string" } },
              required: ["request_id"],
              additionalProperties: false,
            },
            annotations: { readOnlyHint: true },
          },
          {
            name: "submit_demo_result",
            description:
              "Return the approved smoke-test marker only. Does not execute scripts.",
            inputSchema: {
              type: "object",
              properties: {
                request_id: { type: "string" },
                result: { type: "string", const: "BEAVER_DOT_SMOKE_OK" },
              },
              required: ["request_id", "result"],
              additionalProperties: false,
            },
            annotations: { destructiveHint: false, idempotentHint: true },
          },
        ],
      };
    if (method === "events/list")
      return {
        events: [
          {
            name: EVENT,
            description: "A smoke-test task is ready.",
            delivery: ["webhook"],
            inputSchema: schema,
            payloadSchema: {
              type: "object",
              properties: {
                queue_id: { type: "string" },
                request_id: { type: "string" },
                summary: { type: "string" },
              },
              required: ["queue_id", "request_id", "summary"],
              additionalProperties: false,
            },
          },
        ],
      };
    if (method === "events/subscribe") return this.subscribe(owner, params);
    if (method === "events/unsubscribe") {
      for (const [id, sub] of Object.entries(this.store.data.subscriptions))
        if (
          sub.owner === owner &&
          sub.url === params.delivery?.url &&
          params.name === EVENT &&
          params.arguments?.queue_id === "smoke-test"
        )
          delete this.store.data.subscriptions[id];
      await this.save();
      return {};
    }
    if (method === "tools/call") {
      let value;
      if (params.name === "bridge_status")
        value = {
          prototype: true,
          live_callback_transport: Boolean(this.transport),
          model_calls: 0,
        };
      else if (params.name === "get_demo_task")
        value = this.get(owner, params.arguments?.request_id);
      else if (params.name === "submit_demo_result")
        value = await this.result(owner, params.arguments ?? {});
      else throw Error("Unknown tool");
      return {
        content: [{ type: "text", text: JSON.stringify(value) }],
        structuredContent: value,
      };
    }
    throw Error("Unknown method");
  }
}
