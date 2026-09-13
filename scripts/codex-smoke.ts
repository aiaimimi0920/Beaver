import http from "node:http";
import fs from "node:fs/promises";
import path from "node:path";
import assert from "node:assert/strict";
import { Store } from "../src/core/store";
import { Files, ProjectLocks } from "../src/core/files";
import { Projects } from "../src/core/projects";
import { Preferences } from "../src/core/settings";
import { Tasks } from "../src/core/tasks";
import { defaultSettings } from "../src/shared/types";
import { findTool } from "../src/core/process";

// Protocol fixture, NOT a real model: validate the installed Codex process and app lifecycle without user credentials.
async function main() {
  const output = path.resolve("output/validation", `codex-${Date.now()}`);
  await fs.mkdir(output, { recursive: true });
  const requests: string[] = [];
  let holdResponses = false;
  let askNext = false;
  const server = http.createServer(async (req, res) => {
    let input = "";
    for await (const chunk of req) input += chunk;
    requests.push(`${req.method} ${req.url}`);
    if (req.method !== "POST" || !req.url?.endsWith("/responses")) {
      res.writeHead(404).end();
      return;
    }
    if (holdResponses) {
      res.writeHead(200, { "Content-Type": "text/event-stream" });
      res.write(": fixture deliberately waits for cancellation\n\n");
      return;
    }
    const parsed = JSON.parse(input) as { model: string };
    if (askNext) {
      askNext = false;
      assert.match(input, /beaver_ask_user/);
      const item = {
        id: "fc_question",
        type: "function_call",
        call_id: "call_question",
        name: "beaver_ask_user",
        arguments: JSON.stringify({
          questions: [{ id: "tone", question: "主角的语气是什么？" }],
        }),
        status: "completed",
      };
      const response = {
        id: "resp_question",
        object: "response",
        status: "completed",
        model: parsed.model,
        output: [item],
        usage: { input_tokens: 1, output_tokens: 1, total_tokens: 2 },
      };
      res.writeHead(200, { "Content-Type": "text/event-stream" });
      for (const event of [
        {
          type: "response.created",
          response: { ...response, status: "in_progress", output: [] },
        },
        {
          type: "response.output_item.added",
          output_index: 0,
          item: { ...item, arguments: "", status: "in_progress" },
        },
        {
          type: "response.function_call_arguments.delta",
          item_id: item.id,
          output_index: 0,
          delta: item.arguments,
        },
        { type: "response.output_item.done", output_index: 0, item },
        { type: "response.completed", response },
      ])
        res.write(`event: ${event.type}\ndata: ${JSON.stringify(event)}\n\n`);
      res.end();
      return;
    }
    const message = {
      id: "msg_fixture",
      type: "message",
      role: "assistant",
      status: "completed",
      content: [
        {
          type: "output_text",
          text: "BEAVER_PROTOCOL_FIXTURE_OK - no real AI generation was performed.",
          annotations: [],
        },
      ],
    };
    const response = {
      id: "resp_fixture",
      object: "response",
      status: "completed",
      model: parsed.model,
      output: [message],
      usage: {
        input_tokens: 1,
        output_tokens: 1,
        total_tokens: 2,
        input_tokens_details: { cached_tokens: 0 },
        output_tokens_details: { reasoning_tokens: 0 },
      },
    };
    res.writeHead(200, {
      "Content-Type": "text/event-stream",
      "Cache-Control": "no-cache",
    });
    for (const event of [
      {
        type: "response.created",
        response: { ...response, status: "in_progress", output: [] },
      },
      { type: "response.output_item.added", output_index: 0, item: message },
      {
        type: "response.output_text.delta",
        item_id: message.id,
        output_index: 0,
        content_index: 0,
        delta: message.content[0]!.text,
      },
      { type: "response.output_item.done", output_index: 0, item: message },
      { type: "response.completed", response },
    ])
      res.write(`event: ${event.type}\ndata: ${JSON.stringify(event)}\n\n`);
    res.end();
  });
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
  const address = server.address();
  if (!address || typeof address === "string")
    throw new Error("Fixture server unavailable");
  const store = new Store(path.join(output, "data"));
  const prefs = new Preferences(store, {
    encrypt: (s) => s,
    decrypt: (s) => s,
  });
  const settings = defaultSettings();
  settings.local.code = {
    baseUrl: `http://127.0.0.1:${address.port}/v1`,
    model: "gpt-5.4",
    route: "",
  };
  settings.tools.codex = await findTool("codex");
  prefs.save(settings, {});
  const projects = new Projects(store, path.resolve("resources"));
  const project = await projects.create(output, "protocol-project", "blank");
  const tasks = new Tasks(
    store,
    new Files(store.root),
    new ProjectLocks(),
    prefs,
    path.resolve("resources"),
    path.resolve("dist/mcp.cjs"),
    () => {},
  );
  try {
    const task = await tasks.create({
      projectId: project.id,
      prompt: "Respond to the protocol fixture. Do not modify files.",
    });
    const deadline = Date.now() + 90000;
    while (
      ["queued", "running"].includes(tasks.get(task.id).status) &&
      Date.now() < deadline
    )
      await new Promise((r) => setTimeout(r, 250));
    const result = tasks.get(task.id);
    await fs.writeFile(
      path.join(output, "protocol-proof.json"),
      JSON.stringify(
        {
          kind: "local protocol fixture, not live AI",
          codex: settings.tools.codex,
          status: result.status,
          error: result.error,
          report: result.report,
          threadId: result.threadId,
          requests,
          events: store.events(task.id),
        },
        null,
        2,
      ),
    );
    assert.equal(result.status, "completed", result.error);
    assert.match(result.report ?? "", /BEAVER_PROTOCOL_FIXTURE_OK/);
    assert.ok(result.threadId);
    assert.ok(requests.some((r) => r.endsWith("/responses")));
    const until = async (condition: () => boolean) => {
      const stop = Date.now() + 60000;
      while (!condition() && Date.now() < stop)
        await new Promise((r) => setTimeout(r, 100));
      assert.ok(condition(), "Timed out waiting for Codex lifecycle condition");
    };
    askNext = true;
    const questionTask = await tasks.create({
      projectId: project.id,
      prompt: "Ask for missing character intent using beaver_ask_user.",
    });
    await until(
      () =>
        tasks.get(questionTask.id).status === "awaitingInput" ||
        tasks.get(questionTask.id).status === "failed",
    );
    assert.equal(
      tasks.get(questionTask.id).status,
      "awaitingInput",
      tasks.get(questionTask.id).error,
    );
    const question = tasks.get(questionTask.id).clarifications![0]!;
    await tasks.answer(questionTask.id, question.id, { tone: "温柔但坚定" });
    await until(
      () => !["queued", "running"].includes(tasks.get(questionTask.id).status),
    );
    assert.equal(
      tasks.get(questionTask.id).status,
      "completed",
      tasks.get(questionTask.id).error,
    );
    assert.equal(
      tasks.get(questionTask.id).clarifications![0]!.answers?.tone,
      "温柔但坚定",
    );
    holdResponses = true;
    const count = requests.length;
    const a = await tasks.create({
      projectId: project.id,
      prompt: "Parallel protocol fixture A",
    });
    const b = await tasks.create({
      projectId: project.id,
      prompt: "Parallel protocol fixture B",
    });
    await until(
      () =>
        requests.length >= count + 2 &&
        !!tasks.get(a.id).turnId &&
        !!tasks.get(b.id).turnId,
    );
    assert.equal(tasks.get(a.id).status, "running");
    assert.equal(tasks.get(b.id).status, "running");
    const originalThread = tasks.get(a.id).threadId;
    await tasks.continue(a.id, "BEAVER_STEER_PERSISTENCE_FIXTURE");
    await Promise.all([tasks.interrupt(a.id), tasks.interrupt(b.id)]);
    assert.equal(tasks.get(a.id).status, "interrupted");
    assert.equal(tasks.get(b.id).status, "interrupted");
    assert.match(tasks.get(a.id).prompt, /BEAVER_STEER_PERSISTENCE_FIXTURE/);
    holdResponses = false;
    await tasks.continue(
      a.id,
      "Resume protocol fixture without repeating work",
    );
    await until(() => !["queued", "running"].includes(tasks.get(a.id).status));
    assert.equal(tasks.get(a.id).status, "completed", tasks.get(a.id).error);
    assert.equal(tasks.get(a.id).threadId, originalThread);
    await fs.writeFile(
      path.join(output, "lifecycle-proof.json"),
      JSON.stringify(
        {
          kind: "real Codex, fixture Responses API",
          checks: [
            "real Codex dynamic tool asks and parks task",
            "answer persists and resumed thread completes",
            "two concurrent tasks in one project",
            "steer accepted and persisted",
            "both interrupted",
            "resume existing thread",
            "completion after resume",
          ],
          tasks: [a.id, b.id],
          originalThread,
          requests,
        },
        null,
        2,
      ),
    );
    console.log(
      JSON.stringify({
        success: true,
        output,
        kind: "real Codex / fixture Responses API",
      }),
    );
  } finally {
    await tasks.shutdown();
    store.close();
    server.closeAllConnections();
    await new Promise<void>((resolve) => server.close(() => resolve()));
  }
}
main().catch((e: unknown) => {
  console.error(e);
  process.exitCode = 1;
});
