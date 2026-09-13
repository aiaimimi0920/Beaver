import readline from "node:readline";
import fs from "node:fs/promises";
import path from "node:path";
import { z } from "zod";
import { safePath } from "../core/files";
import { validBase } from "../core/settings";

const root = process.env.BEAVER_PROJECT_ROOT;
const providerSchema = z.record(
  z.string(),
  z
    .object({
      baseUrl: z.string(),
      model: z.string(),
      route: z.string(),
      key: z.string(),
    })
    .nullable(),
);
let providers: z.infer<typeof providerSchema> = {};
let configurationError = false;
try {
  providers = providerSchema.parse(
    JSON.parse(process.env.BEAVER_MEDIA_PROVIDERS ?? "{}"),
  );
} catch {
  configurationError = true;
}
async function boundedBody(
  response: Response,
  limit = 140 * 1024 * 1024,
): Promise<Buffer> {
  if (!response.body) throw new Error("Empty AI response");
  const reader = response.body.getReader();
  const chunks: Uint8Array[] = [];
  let size = 0;
  try {
    while (true) {
      const part = await reader.read();
      if (part.done) return Buffer.concat(chunks);
      size += part.value.byteLength;
      if (size > limit) throw new Error("AI response exceeds size limit");
      chunks.push(part.value);
    }
  } finally {
    await reader.cancel();
  }
}
const tools = [
  {
    name: "generate_image",
    description:
      "Generate or edit a project image using the user-configured image service. References are project-relative image files.",
    inputSchema: {
      type: "object",
      properties: {
        prompt: { type: "string" },
        output: { type: "string" },
        references: { type: "array", items: { type: "string" } },
      },
      required: ["prompt", "output"],
      additionalProperties: false,
    },
  },
  {
    name: "generate_speech",
    description: "Generate speech audio through the configured speech service.",
    inputSchema: {
      type: "object",
      properties: {
        prompt: { type: "string" },
        output: { type: "string" },
        voice: { type: "string" },
      },
      required: ["prompt", "output"],
      additionalProperties: false,
    },
  },
  {
    name: "generate_music",
    description:
      "Generate music with the explicitly configured custom music endpoint. Requires a provider supporting the documented Beaver media response contract.",
    inputSchema: {
      type: "object",
      properties: { prompt: { type: "string" }, output: { type: "string" } },
      required: ["prompt", "output"],
      additionalProperties: false,
    },
  },
  {
    name: "translate_text",
    description:
      "Translate text through the configured translation Responses API service.",
    inputSchema: {
      type: "object",
      properties: { prompt: { type: "string" } },
      required: ["prompt"],
      additionalProperties: false,
    },
  },
];
const inputSchema = z.object({
  prompt: z.string().min(1).max(30000),
  output: z.string().optional(),
  references: z.array(z.string()).max(5).optional(),
  voice: z.string().optional(),
});
async function call(name: string, raw: unknown): Promise<string> {
  if (configurationError)
    throw new Error("Invalid media provider configuration");
  const input = inputSchema.parse(raw);
  const capability =
    name === "generate_image"
      ? "image"
      : name === "generate_speech"
        ? "speech"
        : name === "generate_music"
          ? "music"
          : "translation";
  const provider = providers[capability];
  if (!provider)
    throw new Error(
      `${capability} service is not configured. Do not claim generation succeeded.`,
    );
  if (!root) throw new Error("Project root missing");
  let output: string | undefined;
  if (capability !== "translation") {
    if (!input.output) throw new Error("Output path required");
    output = await safePath(root, input.output);
    try {
      await fs.access(output);
      throw new Error("Output already exists; choose a new asset path");
    } catch (e) {
      if ((e as NodeJS.ErrnoException).code !== "ENOENT") throw e;
    }
  }
  let route =
    provider.route || (capability === "translation" ? "/responses" : "");
  if (!route.startsWith("/") || route.startsWith("//"))
    throw new Error("Configure an explicit API route for this capability");
  const headers: Record<string, string> = {};
  if (provider.key) headers.Authorization = `Bearer ${provider.key}`;
  let body: BodyInit;
  if (capability === "image" && input.references?.length) {
    if (route === "/images/generations") route = "/images/edits";
    const form = new FormData();
    form.set("model", provider.model);
    form.set("prompt", input.prompt);
    for (const reference of input.references) {
      const file = await safePath(root, reference);
      if ((await fs.stat(file)).size > 20 * 1024 * 1024)
        throw new Error("Reference image exceeds 20 MB");
      form.append(
        "image[]",
        new Blob([await fs.readFile(file)], {
          type: /\.jpe?g$/i.test(file)
            ? "image/jpeg"
            : /\.webp$/i.test(file)
              ? "image/webp"
              : "image/png",
        }),
        path.basename(file),
      );
    }
    body = form;
  } else {
    headers["Content-Type"] = "application/json";
    body = JSON.stringify(
      capability === "speech"
        ? {
            model: provider.model,
            input: input.prompt,
            voice: input.voice || "alloy",
            response_format: "wav",
          }
        : capability === "translation"
          ? { model: provider.model, input: input.prompt }
          : { model: provider.model, prompt: input.prompt },
    );
  }
  const response = await fetch(validBase(provider.baseUrl) + route, {
    method: "POST",
    headers,
    body,
    signal: AbortSignal.timeout(180000),
  });
  if (!response.ok)
    throw new Error(`AI request failed: HTTP ${response.status}`);
  if (capability === "translation")
    return JSON.stringify(
      JSON.parse(
        (await boundedBody(response, 4 * 1024 * 1024)).toString("utf8"),
      ),
    );
  if (!input.output) throw new Error("Output path required");
  let bytes: Buffer;
  if (response.headers.get("content-type")?.includes("json")) {
    const result = z
      .object({
        data: z
          .array(
            z.object({
              b64_json: z.string().optional(),
              url: z.string().optional(),
            }),
          )
          .min(1),
      })
      .parse(JSON.parse((await boundedBody(response)).toString("utf8")));
    const item = result.data[0]!;
    if (item.b64_json) bytes = Buffer.from(item.b64_json, "base64");
    else if (item.url) {
      const url = new URL(item.url);
      if (url.protocol !== "https:")
        throw new Error("Media download requires HTTPS");
      const media = await fetch(url, { signal: AbortSignal.timeout(120000) });
      if (!media.ok) throw new Error("Media download failed");
      bytes = await boundedBody(media, 100 * 1024 * 1024);
    } else throw new Error("Provider returned no media");
  } else bytes = await boundedBody(response, 100 * 1024 * 1024);
  if (!bytes.length || bytes.length > 100 * 1024 * 1024)
    throw new Error("Invalid media size (maximum 100 MB)");
  await fs.mkdir(path.dirname(output!), { recursive: true });
  await fs.writeFile(output!, bytes, { flag: "wx" });
  return JSON.stringify({ path: input.output, bytes: bytes.length });
}
readline.createInterface({ input: process.stdin }).on("line", (line) => {
  void (async () => {
    let id: unknown;
    try {
      const message = z
        .object({
          id: z.union([z.string(), z.number()]).optional(),
          method: z.string(),
          params: z.record(z.string(), z.unknown()).optional(),
        })
        .parse(JSON.parse(line));
      id = message.id;
      if (id === undefined) return;
      let result: unknown;
      if (message.method === "initialize")
        result = {
          protocolVersion: message.params?.protocolVersion ?? "2024-11-05",
          capabilities: { tools: {} },
          serverInfo: { name: "beaver-media", version: "0.1.0" },
        };
      else if (message.method === "tools/list") result = { tools };
      else if (message.method === "ping") result = {};
      else if (message.method === "tools/call") {
        const name = String(message.params?.name);
        if (!tools.some((t) => t.name === name))
          throw new Error("Unknown tool");
        try {
          result = {
            content: [
              {
                type: "text",
                text: await call(name, message.params?.arguments),
              },
            ],
          };
        } catch (e) {
          result = {
            isError: true,
            content: [{ type: "text", text: String(e) }],
          };
        }
      } else throw new Error("Unsupported MCP method");
      process.stdout.write(
        JSON.stringify({ jsonrpc: "2.0", id, result }) + "\n",
      );
    } catch {
      if (id !== undefined)
        process.stdout.write(
          JSON.stringify({
            jsonrpc: "2.0",
            id,
            error: { code: -32603, message: "MCP request failed" },
          }) + "\n",
        );
    }
  })();
});
