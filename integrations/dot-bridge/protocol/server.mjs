import http from "node:http";
import { timingSafeEqual } from "node:crypto";
import { Bridge } from "./bridge.mjs";
import { openStore } from "./store.mjs";

// Operator-supplied credentials only; this prototype never creates access tokens.
const token = process.env.BEAVER_BRIDGE_TOKEN;
if (!token || token.length < 32)
  throw Error(
    "Supply an approved BEAVER_BRIDGE_TOKEN of at least32 characters",
  );
const store = await openStore(
  process.env.BEAVER_BRIDGE_STATE ?? "./.local/state.json",
);
const bridge = new Bridge({ store }); // Live webhook transport deliberately disabled.
function authorized(header) {
  const a = Buffer.from(header ?? ""),
    b = Buffer.from("Bearer " + token);
  return a.length === b.length && timingSafeEqual(a, b);
}
const server = http.createServer(async (req, res) => {
  const reply = (status, value) => {
    res.writeHead(status, {
      "Content-Type": "application/json",
      "Cache-Control": "no-store",
    });
    res.end(JSON.stringify(value));
  };
  if (!authorized(req.headers.authorization))
    return reply(401, { error: "Authentication required" });
  if (req.method !== "POST") return reply(405, { error: "POST required" });
  const chunks = [];
  let size = 0;
  try {
    for await (const chunk of req) {
      size += chunk.length;
      if (size > 262144) return reply(413, { error: "Request too large" });
      chunks.push(chunk);
    }
    const data = JSON.parse(Buffer.concat(chunks));
    // Fixed owner is valid only for this isolated single-owner prototype.
    const owner = "local-owner";
    if (req.url === "/demo/request")
      return reply(200, await bridge.createDemo(owner, data));
    if (req.url === "/demo/deliver")
      return reply(200, await bridge.deliver(owner, data.request_id));
    if (req.url !== "/mcp") return reply(404, { error: "Not found" });
    if (data.jsonrpc !== "2.0")
      return reply(400, { error: "JSON-RPC2.0 required" });
    if (data.method?.startsWith("notifications/")) {
      res.writeHead(202);
      return res.end();
    }
    const result = await bridge.rpc(owner, data.method, data.params);
    reply(200, { jsonrpc: "2.0", id: data.id, result });
  } catch (error) {
    reply(400, { error: error.message });
  }
});
server.listen(Number(process.env.PORT ?? 8765), "127.0.0.1", () =>
  console.log("Local-only bridge listening; live dot callbacks are DISABLED."),
);
