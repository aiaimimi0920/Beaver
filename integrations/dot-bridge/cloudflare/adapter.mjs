import {
  canonical,
  identifier,
  requireValue,
  validateReply,
  validateRequest,
} from "./contract.mjs";

export class RelayClient {
  constructor(origin, token, transport = fetch) {
    const url = new URL(origin);
    requireValue(
      url.protocol === "https:" &&
        !url.username &&
        !url.password &&
        !url.search &&
        !url.hash &&
        url.pathname === "/",
      "Expected HTTPS relay origin",
    );
    requireValue(
      typeof token === "string" && /^[!-~]{32,512}$/.test(token),
      "Invalid relay token",
    );
    this.origin = url.origin;
    this.token = token;
    this.transport = transport;
  }
  async call(path, body) {
    const response = await this.transport(this.origin + path, {
      method: body === undefined ? "GET" : "POST",
      headers: {
        authorization: `Bearer ${this.token}`,
        ...(body === undefined ? {} : { "content-type": "application/json" }),
      },
      body: body === undefined ? undefined : JSON.stringify(body),
      redirect: "error",
      signal: AbortSignal.timeout(30000),
    });
    if (!response.ok)
      throw new Error(`Relay request failed: HTTP ${response.status}`);
    return response.json();
  }
  publish(request) {
    return this.call("/v1/exchanges", validateRequest(request));
  }
  get(id) {
    return this.call(`/v1/exchanges/${identifier(id)}`);
  }
  receipt(id, receipt) {
    return this.call(`/v1/exchanges/${identifier(id)}/receipt`, receipt);
  }
}

export async function prepareExchange(
  core,
  taskId,
  prompt,
  id = crypto.randomUUID(),
) {
  const context = await core.call("external.context", { taskId });
  requireValue(
    context.taskId === taskId && context.status === "active",
    "Task must have an active external run",
  );
  return validateRequest({
    id,
    taskId,
    runId: context.runId,
    revision: context.revision,
    prompt,
    context,
  });
}

export async function applyExchange(core, relay, authorizedRequest) {
  // This envelope must originate from the local manifest, not a remote task ID.
  validateRequest(authorizedRequest);
  const remote = await relay.get(authorizedRequest.id);
  requireValue(
    canonical(remote.request) === canonical(authorizedRequest),
    "Remote request differs from local authorization",
  );
  if (remote.receipt !== null)
    return { status: "acknowledged", receipt: remote.receipt };
  if (remote.reply === null) return { status: "waiting" };
  const reply = validateReply(remote.reply);
  const input = {
    taskId: authorizedRequest.taskId,
    runId: authorizedRequest.runId,
    revision: authorizedRequest.revision,
    requestId: authorizedRequest.id,
    arguments: reply.arguments,
  };
  // Core owns revision/permissions/idempotency. Never refresh fences or mint a
  // new request ID after a timeout: an admitted operation may already have run.
  const receipt = await core.call(reply.method, input);
  await relay.receipt(authorizedRequest.id, receipt);
  return { status: "acknowledged", receipt };
}
