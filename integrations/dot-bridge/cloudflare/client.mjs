import { mkdir, readFile, writeFile } from "node:fs/promises";
import { resolve, isAbsolute } from "node:path";
import { setTimeout as delay } from "node:timers/promises";
import { CoreTransport } from "../../../src/direct/transport.cjs";
import { RelayClient, prepareExchange, applyExchange } from "./adapter.mjs";

// The Core host must remain alive across all turns; a restart fences the run.
const [directory, promptPath] = process.argv.slice(2);
if (!directory || !promptPath || !isAbsolute(directory))
  throw new Error("Usage: client.mjs NEW_ABSOLUTE_DIRECTORY PROMPT_FILE");
const {
  BEAVER_RELAY_ORIGIN: origin,
  BEAVER_RELAY_TOKEN: token,
  BEAVER_HEADLESS: binary,
} = process.env;
const relay = new RelayClient(origin, token);
if (!isAbsolute(binary ?? ""))
  throw new Error("BEAVER_HEADLESS must be absolute");
const prompt = await readFile(promptPath, "utf8");
await mkdir(directory, { recursive: false });
const data = resolve(directory, "data");
await mkdir(data);
const core = new CoreTransport(binary, data);
let stopping = false;
process.on("SIGINT", () => {
  stopping = true;
});
process.on("SIGTERM", () => {
  stopping = true;
});
try {
  const project = await core.call("project.create", {
    parent: directory,
    name: "Dot Relay",
    template: "blank",
  });
  const task = await core.call("task.create", {
    projectId: project.id,
    prompt,
    executionMode: "external-agent",
    capability: "code",
    decompose: false,
    autoAccept: false,
    askRatio: 0,
    maxMinutes: 30,
  });
  const deadline = Date.now() + 30 * 60 * 1000;
  let ready = false;
  for (let attempt = 0; attempt < 60 && !stopping; attempt++) {
    const pending = await core.call("external.pending");
    if (pending.some((item) => item.taskId === task.id)) {
      ready = true;
      break;
    }
    await delay(500);
  }
  if (!ready) throw new Error("External run did not become ready");
  for (let turn = 0; turn < 20 && !stopping && Date.now() < deadline; turn++) {
    const request = await prepareExchange(core, task.id, prompt);
    await writeFile(
      resolve(directory, request.id + ".json"),
      JSON.stringify(request, null, 2),
      {
        encoding: "utf8",
        flag: "wx",
        mode: 0o600,
      },
    );
    await relay.publish(request);
    let result;
    while (!stopping && Date.now() < deadline) {
      result = await applyExchange(core, relay, request);
      if (result.status === "acknowledged") break;
      console.log(
        JSON.stringify({ exchangeId: request.id, status: "waiting" }),
      );
      await delay(3000);
    }
    if (!result || result.status !== "acknowledged") break;
    console.log(JSON.stringify(result));
    const current = await relay.get(request.id);
    if (
      current.reply.method === "external.finish" ||
      result.receipt.status === "failed"
    )
      break;
  }
} finally {
  if (core && !core.closed) await core.call("shutdown");
}
