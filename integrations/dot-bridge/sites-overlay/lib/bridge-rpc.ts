import {
  listTasks,
  getTask,
  completeTask,
  removeSubscription,
  subscriptions,
} from "./bridge-db";
import { eventName, subscribe } from "./bridge-events";
const requestSchema = {
  type: "object",
  properties: { request_id: { type: "string" } },
  required: ["request_id"],
  additionalProperties: false,
};
export async function rpc(owner: string, method: string, p: any = {}) {
  if (method === "server/discover")
    return {
      resultType: "complete",
      supportedVersions: ["2026-07-28", "2025-06-18"],
      capabilities: { tools: {}, events: {} },
    };
  if (method === "initialize")
    return {
      protocolVersion: [
        "2026-07-28",
        "2025-06-18",
        "2025-03-26",
        "2024-11-05",
      ].includes(p.protocolVersion)
        ? p.protocolVersion
        : "2026-07-28",
      serverInfo: { name: "beaver-dot-bridge", version: "0.1.0" },
      capabilities: { tools: {}, events: {} },
    };
  if (method === "ping") return {};
  if (method === "tools/list")
    return {
      tools: [
        {
          name: "bridge_status",
          description:
            "Read this private smoke-test bridge status. No model or Codex call is made.",
          inputSchema: {
            type: "object",
            properties: {},
            additionalProperties: false,
          },
          annotations: { readOnlyHint: true },
        },
        {
          name: "get_demo_task",
          description: "Read one Beaver smoke-test request.",
          inputSchema: requestSchema,
          annotations: { readOnlyHint: true },
        },
        {
          name: "submit_demo_result",
          description:
            "Return only the approved smoke-test marker to Beaver. This tool does not execute scripts.",
          inputSchema: {
            ...requestSchema,
            properties: {
              request_id: { type: "string" },
              result: { type: "string", const: "BEAVER_DOT_SMOKE_OK" },
            },
            required: ["request_id", "result"],
          },
          annotations: { destructiveHint: false, idempotentHint: true },
        },
      ],
    };
  if (method === "events/list")
    return {
      events: [
        {
          name: eventName,
          description: "A harmless Beaver smoke-test request is ready.",
          delivery: ["webhook"],
          inputSchema: {
            type: "object",
            properties: { queue_id: { type: "string", const: "smoke-test" } },
            required: ["queue_id"],
            additionalProperties: false,
          },
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
  if (!owner) throw Error("Authenticated user required");
  if (method === "events/subscribe") return subscribe(owner, p);
  if (method === "events/unsubscribe") {
    if (p.name !== eventName || p.arguments?.queue_id !== "smoke-test")
      throw Error("Unsupported subscription");
    await removeSubscription(owner, p.delivery?.url ?? "");
    return {};
  }
  if (method === "tools/call") {
    let result;
    if (p.name === "bridge_status")
      result = {
        mode: "smoke-test-only",
        model_calls: 0,
        active_subscriptions: (await subscriptions()).length,
        tasks: await listTasks(),
      };
    else if (p.name === "get_demo_task")
      result = await getTask(p.arguments?.request_id);
    else if (p.name === "submit_demo_result")
      result = await completeTask(p.arguments?.request_id, p.arguments?.result);
    else throw Error("Unknown tool");
    return {
      content: [{ type: "text", text: JSON.stringify(result) }],
      structuredContent: result,
    };
  }
  throw Error("Unknown method");
}
