import { rpc } from "../../lib/bridge-rpc";
export async function POST(req: Request) {
  let id = null;
  let method = "";
  try {
    const raw = await req.text();
    if (new TextEncoder().encode(raw).length > 262144)
      return new Response("Too large", { status: 413 });
    const data = JSON.parse(raw);
    id = data.id ?? null;
    method = String(data.method ?? "");
    if (method === "events/subscribe") {
      let callbackHost = "invalid";
      try {
        callbackHost = new URL(data.params?.delivery?.url ?? "").hostname;
      } catch {}
      console.info(
        JSON.stringify({
          stage: "event-subscription-attempt",
          callbackHost,
          parameterKeys: Object.keys(data.params ?? {}),
          deliveryKeys: Object.keys(data.params?.delivery ?? {}),
        }),
      );
    }
    if (data.jsonrpc !== "2.0") throw Error("JSON-RPC2.0 required");
    if (data.method?.startsWith("notifications/"))
      return new Response(null, { status: 202 });
    const result = await rpc(
      req.headers.get("oai-authenticated-user-id") ?? "",
      data.method,
      data.params,
    );
    return Response.json({ jsonrpc: "2.0", id, result });
  } catch (error) {
    console.error(
      JSON.stringify({
        stage: "mcp-request-failed",
        method,
        reason: error instanceof Error ? error.message : "Request failed",
      }),
    );
    return Response.json({
      jsonrpc: "2.0",
      id,
      error: {
        code: -32000,
        message: error instanceof Error ? error.message : "Request failed",
      },
    });
  }
}
export async function GET() {
  return new Response("Stream transport not provided", { status: 405 });
}
