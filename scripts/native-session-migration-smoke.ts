import http from "node:http";
import fs from "node:fs/promises";
import path from "node:path";
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { Store } from "../src/core/store";
import { Files, ProjectLocks } from "../src/core/files";
import { Projects } from "../src/core/projects";
import { Preferences } from "../src/core/settings";
import { Tasks } from "../src/core/tasks";
import { defaultSettings } from "../src/shared/types";
import { findTool } from "../src/core/process";
import { inventory } from "./native-release";

function run(
  executable: string,
  args: string[],
  timeout = 150_000,
): Promise<string> {
  return new Promise((resolve, reject) => {
    const child = spawn("rtk", ["proxy", executable, ...args], {
      windowsHide: true,
      stdio: "pipe",
    });
    let stdout = "",
      stderr = "";
    child.stdout.on("data", (bytes: Buffer) => {
      stdout += bytes.toString();
    });
    child.stderr.on("data", (bytes: Buffer) => {
      stderr += bytes.toString();
    });
    const timer = setTimeout(() => {
      const killer = spawn(
        "rtk",
        ["proxy", "taskkill.exe", "/PID", String(child.pid), "/T", "/F"],
        { windowsHide: true, stdio: "ignore" },
      );
      killer.on("error", () => child.kill());
    }, timeout);
    child.on("error", (error) => {
      clearTimeout(timer);
      reject(error);
    });
    child.on("close", (code) => {
      clearTimeout(timer);
      if (code === 0) resolve(stdout);
      else
        reject(
          new Error(`Probe process failed (${code}): ${stderr.slice(-6000)}`),
        );
    });
  });
}

async function main() {
  const beaver = path.resolve(process.argv[2] ?? "target/release/Beaver.exe");
  const godot = process.argv[3]
    ? path.resolve(process.argv[3])
    : await findTool("godot");
  const probe = path.resolve("target/debug/examples/migrated_session.exe");
  const root = path.resolve(
    "output/validation",
    `native-session-migration-${Date.now()}`,
  );
  await fs.mkdir(root, { recursive: true });
  const requests: { stage: string; history: boolean; toolNames: string[] }[] =
    [];
  let stage = "legacy",
    toolCalled = false,
    sawWorkingDirectory = false;
  let expectedWorkspace = "";
  let serverError: unknown;
  const server = http.createServer(async (req, res) => {
    try {
      assert.equal(req.method, "POST");
      assert.ok(req.url?.endsWith("/responses"));
      let body = "";
      for await (const chunk of req) body += chunk;
      const request = JSON.parse(body) as {
        model: string;
        input: unknown[];
        tools?: { name?: string }[];
      };
      const history = JSON.stringify(request.input).includes(
        "BEAVER_NATIVE_HISTORY",
      );
      const toolNames =
        request.tools?.map((tool) => tool.name ?? "").filter(Boolean) ?? [];
      requests.push({ stage, history, toolNames });
      let item: Record<string, unknown>;
      if (stage === "relocated" && !toolCalled) {
        assert.ok(
          history,
          "compatible native assistant history must survive real thread resume",
        );
        assert.ok(
          toolNames.includes("exec_command"),
          `exec_command unavailable: ${toolNames.join(",")}`,
        );
        toolCalled = true;
        item = {
          id: "fc_cwd",
          type: "function_call",
          call_id: "call_cwd",
          name: "exec_command",
          status: "completed",
          arguments: JSON.stringify({
            cmd: "[System.IO.File]::WriteAllText((Join-Path (Get-Location).Path 'relocated-proof.md'), (Get-Location).Path)",
            max_output_tokens: 1000,
          }),
        };
      } else {
        if (stage === "relocated") {
          const marker = await fs.readFile(
            path.join(expectedWorkspace, "relocated-proof.md"),
            "utf8",
          );
          assert.equal(
            path.resolve(marker).toLowerCase(),
            path.resolve(expectedWorkspace).toLowerCase(),
          );
          sawWorkingDirectory = true;
        }
        item = {
          id: `msg_${stage}`,
          type: "message",
          role: "assistant",
          status: "completed",
          content: [
            {
              type: "output_text",
              text:
                stage === "legacy"
                  ? "BEAVER_LEGACY_HISTORY"
                  : stage === "upgrade"
                    ? "BEAVER_NATIVE_HISTORY"
                    : "BEAVER_RELOCATED_OK",
              annotations: [],
            },
          ],
        };
      }
      const response = {
        id: `resp_${requests.length}`,
        object: "response",
        status: "completed",
        model: request.model,
        output: [item],
        usage: { input_tokens: 1, output_tokens: 1, total_tokens: 2 },
      };
      res.writeHead(200, { "Content-Type": "text/event-stream" });
      for (const event of [
        {
          type: "response.created",
          response: { ...response, status: "in_progress", output: [] },
        },
        { type: "response.output_item.added", output_index: 0, item },
        { type: "response.output_item.done", output_index: 0, item },
        { type: "response.completed", response },
      ])
        res.write(`event: ${event.type}\ndata: ${JSON.stringify(event)}\n\n`);
      res.end();
    } catch (error) {
      serverError = error;
      res.writeHead(500).end("fixture assertion failed");
    }
  });
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
  const address = server.address();
  assert.ok(address && typeof address !== "string");
  const source = path.join(root, "source");
  const store = new Store(source);
  let sourceClosed = false;
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
  settings.tools.node = process.execPath;
  settings.mcp = { godot: false, blender: false };
  prefs.save(settings, {});
  const projects = new Projects(store, path.resolve("resources"));
  const project = await projects.create(root, "legacy-project", "blank");
  const tasks = new Tasks(
    store,
    new Files(source),
    new ProjectLocks(),
    prefs,
    path.resolve("resources"),
    path.resolve("dist/mcp.cjs"),
    () => {},
  );
  try {
    const task = await tasks.create({
      projectId: project.id,
      prompt: "BEAVER_LEGACY_TURN. Reply without editing any files.",
      decompose: false,
    });
    const deadline = Date.now() + 120_000;
    while (
      ["queued", "running"].includes(tasks.get(task.id).status) &&
      Date.now() < deadline
    )
      await new Promise((resolve) => setTimeout(resolve, 100));
    const old = tasks.get(task.id);
    assert.equal(old.status, "completed", old.error);
    assert.ok(old.threadId);
    await tasks.shutdown();
    await fs.writeFile(
      path.join(source, "migration-session-fixture.json"),
      JSON.stringify({ fixture: true, taskId: task.id }),
    );
    store.close();
    sourceClosed = true;
    const legacySessions = (await inventory(source)).filter((entry) =>
      entry.path.endsWith(`${old.threadId}.jsonl`),
    );
    assert.equal(
      legacySessions.length,
      1,
      "legacy transcript must be retained",
    );
    stage = "upgrade";
    const upgraded = JSON.parse(
      await run(probe, [source, beaver, "upgrade"]),
    ) as {
      passed: boolean;
      threadId: string;
      priorCallbackThreadId: string;
    };
    assert.equal(upgraded.passed, true);
    assert.notEqual(upgraded.threadId, old.threadId);
    assert.equal(upgraded.priorCallbackThreadId, old.threadId);
    assert.deepEqual(
      (await inventory(source)).filter((entry) =>
        entry.path.endsWith(`${old.threadId}.jsonl`),
      ),
      legacySessions,
    );
    const sourceBefore = await inventory(source),
      projectBefore = await inventory(project.path);
    const backup = path.join(root, "backup"),
      destination = path.join(root, "prepared");
    await run(beaver, ["--migration-bundle", "create", source, backup]);
    const archiveBefore = await inventory(backup);
    for (const directory of [source, project.path]) {
      assert.ok(path.resolve(directory).startsWith(root + path.sep));
      await fs.rename(directory, `${directory}-unavailable`);
    }
    await run(beaver, [
      "--migration-bundle",
      "prepare-import",
      backup,
      destination,
    ]);
    const data = path.join(destination, "data");
    expectedWorkspace = path.join(data, "workspaces", task.id);
    stage = "relocated";
    const result = JSON.parse(await run(probe, [data, beaver])) as {
      passed: boolean;
      threadId: string;
      status: string;
    };
    assert.equal(result.passed, true);
    assert.equal(result.threadId, upgraded.threadId);
    assert.equal(result.status, "completed");
    assert.ok(sawWorkingDirectory);
    assert.equal(
      await fs.readFile(
        path.join(destination, "projects", project.id, "relocated-proof.md"),
        "utf8",
      ),
      await fs.readFile(
        path.join(expectedWorkspace, "relocated-proof.md"),
        "utf8",
      ),
    );
    assert.deepEqual(await inventory(backup), archiveBefore);
    assert.deepEqual(await inventory(`${source}-unavailable`), sourceBefore);
    assert.deepEqual(
      await inventory(`${project.path}-unavailable`),
      projectBefore,
    );
    assert.ok(!serverError, String(serverError));
    const toolPaths = path.join(root, "tool-paths.json");
    await fs.writeFile(
      toolPaths,
      JSON.stringify({
        codex: path.join(root, "missing-codex.exe"),
        godot,
        blender: "",
        node: process.execPath,
      }),
    );
    await assert.rejects(
      run(beaver, [
        "--migration-bundle",
        "activate-import",
        backup,
        destination,
        toolPaths,
      ]),
      /required destination tool unavailable: codex/,
    );
    await fs.access(path.join(destination, ".beaver-migration-pending"));
    await assert.rejects(fs.access(path.join(destination, "ACTIVATION.json")));
    await fs.writeFile(
      toolPaths,
      JSON.stringify({
        codex: settings.tools.codex,
        godot,
        blender: "",
        node: process.execPath,
      }),
    );
    const activation = JSON.parse(
      await run(beaver, [
        "--migration-bundle",
        "activate-import",
        backup,
        destination,
        toolPaths,
      ]),
    ) as {
      ready_to_activate: boolean;
      default_data_directory_changed: boolean;
      live_model_request_made: boolean;
    };
    assert.equal(activation.ready_to_activate, true);
    assert.equal(activation.default_data_directory_changed, false);
    assert.equal(activation.live_model_request_made, false);
    await assert.rejects(
      fs.access(path.join(destination, ".beaver-migration-pending")),
    );
    assert.deepEqual(await inventory(backup), archiveBefore);
    assert.deepEqual(await inventory(`${source}-unavailable`), sourceBefore);
    assert.deepEqual(
      await inventory(`${project.path}-unavailable`),
      projectBefore,
    );
    const proof = {
      beaver,
      codex: settings.tools.codex,
      codexVersion: (await run(settings.tools.codex, ["--version"])).trim(),
      legacyThreadId: old.threadId,
      threadId: upgraded.threadId,
      dataDirectory: data,
      activation,
      checks: [
        "real legacy Codex session persisted",
        "native tool upgrade starts compatible thread and preserves legacy identity and transcript",
        "full bundle import while old roots unavailable",
        "native launch and executor resume identical compatible thread",
        "native assistant history reaches relocated Responses request",
        "real exec_command runs at relocated workspace",
        "native result merged only into relocated project",
        "archive and old source bytes unchanged",
        "missing required tool keeps pending and no activation receipt",
        "actual destination versions checked before explicit activation without changing default data root",
      ],
      requests,
      realCodexResumeVerified: true,
      liveModelQualityVerified: false,
      userDataTouched: false,
    };
    await fs.writeFile(
      path.join(root, "proof.json"),
      JSON.stringify(proof, null, 2),
    );
    console.log(JSON.stringify({ root, ...proof }, null, 2));
  } catch (error) {
    await fs.writeFile(
      path.join(root, "failure.json"),
      JSON.stringify(
        { error: String(error), serverError: String(serverError), requests },
        null,
        2,
      ),
    );
    throw error;
  } finally {
    if (!sourceClosed) {
      await tasks.shutdown();
      store.close();
    }
    server.closeAllConnections();
    await new Promise<void>((resolve) => server.close(() => resolve()));
  }
}
main().catch((error: unknown) => {
  console.error(error);
  process.exitCode = 1;
});
