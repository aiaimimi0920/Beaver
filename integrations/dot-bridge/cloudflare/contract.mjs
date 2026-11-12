export const methods = [
  "external.submitPlan",
  "external.tool",
  "external.finish",
];
export function requireValue(condition, message, status = 400) {
  if (!condition) throw Object.assign(new Error(message), { status });
}
export function identifier(value) {
  requireValue(
    typeof value === "string" && /^[A-Za-z0-9_.:-]{1,160}$/.test(value),
    "Invalid identity",
  );
  return value;
}
export function canonical(value) {
  if (Array.isArray(value)) return `[${value.map(canonical).join(",")}]`;
  if (value && typeof value === "object") {
    return `{${Object.keys(value)
      .sort()
      .map((key) => `${JSON.stringify(key)}:${canonical(value[key])}`)
      .join(",")}}`;
  }
  return JSON.stringify(value);
}
export function validateRequest(value) {
  requireValue(value && typeof value === "object", "Expected request");
  identifier(value.id);
  identifier(value.taskId);
  identifier(value.runId);
  requireValue(
    Number.isSafeInteger(value.revision) && value.revision >= 0,
    "Invalid revision",
  );
  requireValue(
    typeof value.prompt === "string" && value.prompt.length > 0,
    "Expected prompt",
  );
  requireValue(
    value.context && typeof value.context === "object",
    "Expected context",
  );
  requireValue(
    Object.keys(value).every((key) =>
      ["id", "taskId", "runId", "revision", "prompt", "context"].includes(key),
    ),
    "Unknown request field",
  );
  return value;
}
export function validateReply(value) {
  requireValue(
    value && methods.includes(value.method),
    "Unsupported external method",
  );
  requireValue(
    value.arguments &&
      typeof value.arguments === "object" &&
      !Array.isArray(value.arguments),
    "Expected arguments object",
  );
  requireValue(
    Object.keys(value).every((key) => ["method", "arguments"].includes(key)),
    "Unknown reply field",
  );
  return value;
}
export async function readJSON(request) {
  requireValue(
    request.headers.get("content-type")?.split(";")[0] === "application/json",
    "Expected application/json",
    415,
  );
  const reader = request.body?.getReader();
  requireValue(reader, "Expected body");
  const chunks = [];
  let size = 0;
  while (true) {
    const { value, done } = await reader.read();
    if (done) break;
    size += value.byteLength;
    if (size > 524288) {
      await reader.cancel();
      requireValue(false, "Message exceeds 512 KiB", 413);
    }
    chunks.push(value);
  }
  const bytes = new Uint8Array(size);
  let offset = 0;
  for (const chunk of chunks) {
    bytes.set(chunk, offset);
    offset += chunk.byteLength;
  }
  try {
    return JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(bytes));
  } catch {
    requireValue(false, "Invalid JSON");
  }
}
export async function sha256(value) {
  const bytes = await crypto.subtle.digest(
    "SHA-256",
    new TextEncoder().encode(value),
  );
  return Array.from(new Uint8Array(bytes), (byte) =>
    byte.toString(16).padStart(2, "0"),
  ).join("");
}
