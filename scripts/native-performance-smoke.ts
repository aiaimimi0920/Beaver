import assert from "node:assert/strict";
import fs from "node:fs/promises";
import path from "node:path";
import { createHash } from "node:crypto";
import type { Project, Task, TaskEvent, TaskStatus } from "../src/shared/types";
import { defaultSettings } from "../src/shared/types";
import type { State } from "../src/ui/api";
import { executionStatus } from "../src/shared/task-conversation";

type StartProof = {
  root: string;
  projectId: string;
  port: number;
  executable: string;
  exeSha256: string;
};
type Preview = {
  ok: boolean;
  report: string;
  model: string;
  modelSha256: string;
  definitionSha256: string;
  camera: { target: number[]; distance: number; fov: number; angles: number[] };
  screenshots: string[];
  grayscaleScreenshots: string[];
};

async function main() {
  const proofFile = path.resolve(process.argv[2] ?? "");
  const mode = process.argv[3] ?? "fixture";
  assert(
    ["fixture", "protocol", "live"].includes(mode),
    "Expected fixture, protocol or live mode",
  );
  const start = JSON.parse(await fs.readFile(proofFile, "utf8")) as StartProof;
  const token = process.env.BEAVER_API_TOKEN;
  assert(
    token && /^[A-Za-z0-9_-]{32,256}$/.test(token),
    "Set BEAVER_API_TOKEN in the calling process",
  );
  const sha = createHash("sha256")
    .update(await fs.readFile(start.executable))
    .digest("hex");
  assert.equal(sha, start.exeSha256.toLowerCase());
  const checks: string[] = [];
  const timings: { method: string; milliseconds: number }[] = [];
  async function call<T>(method: string, input: unknown = {}): Promise<T> {
    const time = performance.now();
    const response = await fetch(`http://127.0.0.1:${start.port}/v1/call`, {
      method: "POST",
      headers: {
        Authorization: `Bearer ${token}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify({ method, input }),
      signal: AbortSignal.timeout(300000),
    });
    timings.push({
      method,
      milliseconds: Math.round(performance.now() - time),
    });
    const body = (await response.json()) as {
      ok: boolean;
      result: T;
      error?: { message?: string };
    };
    if (!response.ok || !body.ok)
      throw new Error(
        `${method}: ${(body.error?.message ?? response.statusText).replaceAll(token!, "[REDACTED_SECRET]")}`,
      );
    return body.result;
  }
  async function task(id: string): Promise<Task> {
    const current = (await call<State>("state")).tasks.find((t) => t.id === id);
    assert(current, "Task disappeared");
    return current;
  }
  async function wait(
    id: string,
    statuses: TaskStatus[],
    limit = 60000,
  ): Promise<Task> {
    const until = Date.now() + limit;
    while (Date.now() < until) {
      const current = await task(id);
      if (statuses.includes(current.status)) return current;
      await new Promise((resolve) => setTimeout(resolve, 300));
    }
    throw new Error(
      `Task did not reach ${statuses.join("/")} within ${limit} ms`,
    );
  }
  let preview: Preview | undefined;
  let live:
    | {
        id: string;
        status: TaskStatus;
        milliseconds: number;
        error?: string;
        report?: string;
      }
    | undefined;
  try {
    const initial = await call<State>("state");
    if (!process.argv.includes("--resume")) {
      assert.equal(
        initial.tasks.length,
        0,
        "Each acceptance round must start empty",
      );
    } else {
      assert(
        !initial.tasks.some((task) =>
          ["running", "queued", "waitingChildren"].includes(task.status),
        ),
        "Stop existing tasks before resuming this round",
      );
    }
    assert.equal(initial.projects.length, 1);
    assert.equal(initial.projects[0]?.id, start.projectId);
    const settings = structuredClone(initial.settings);
    settings.maxParallel = 1;
    settings.mcp = { godot: false, blender: false };
    if (mode !== "live") {
      settings.mode = "local";
      settings.local.code = {
        baseUrl: "http://127.0.0.1:9/v1",
        model: "fixture",
        route: "",
      };
      settings.tools.codex = path.resolve(
        "target/debug/examples/executor_contract.exe",
      );
      await call("settings.save", {
        settings,
        keys: { code: "dummy-executor-secret" },
      });
      const created = await call<Task>("task.create", {
        projectId: start.projectId,
        prompt: "native-desktop-fixture:fresh",
        decompose: false,
      });
      let running = await wait(created.id, ["running"]);
      for (let i = 0; i < 100; i++) {
        const events = await call<TaskEvent[]>("task.events", {
          id: created.id,
        });
        if (events.some((event) => event.kind === "execution")) {
          assert.match(executionStatus(events)!, /等待模型输出/);
          break;
        }
        assert(i < 99, "Execution health was not published");
        await new Promise((resolve) => setTimeout(resolve, 100));
      }
      await assert.rejects(
        call("task.continue", { id: created.id, text: "", freshContext: true }),
      );
      running = await task(created.id);
      assert(running.threadId);
      await call("task.interrupt", { id: created.id });
      const stopped = await wait(created.id, ["interrupted"]);
      assert.equal(
        await fs.readFile(path.join(stopped.workspace, "partial.txt"), "utf8"),
        "preserve unfinished work",
      );
      await call("task.continue", {
        id: created.id,
        text: "继续检查已有结果。",
        freshContext: true,
      });
      const recovered = await wait(created.id, [
        "completed",
        "failed",
        "conflict",
      ]);
      assert.equal(recovered.status, "completed", recovered.error);
      assert.equal(recovered.workspace, stopped.workspace);
      assert.deepEqual(recovered.baseline, stopped.baseline);
      assert.equal(recovered.sessionHistory?.[0]?.threadId, stopped.threadId);
      assert.notEqual(recovered.threadId, stopped.threadId);
      const events = await call<TaskEvent[]>("task.events", { id: created.id });
      assert(events.some((event) => event.kind === "contextRestart"));
      await call("task.rollback", { id: created.id, keep: [] });
      const project = initial.projects[0]!;
      await assert.rejects(fs.stat(path.join(project.path, "partial.txt")));
      await assert.rejects(fs.stat(path.join(project.path, "result.txt")));
      checks.push(
        "Native API publishes health, rejects live fresh restart, preserves work/history/baseline through recovery, and rolls back all task-owned output",
      );
      const retrying = await call<Task>("task.create", {
        projectId: start.projectId,
        prompt: "native-desktop-fixture:retrying",
        decompose: false,
      });
      await wait(retrying.id, ["running"]);
      let retries: TaskEvent[] = [];
      for (let i = 0; i < 100; i++) {
        retries = await call<TaskEvent[]>("task.events", { id: retrying.id });
        if (retries.some((event) => event.kind === "providerError")) break;
        await new Promise((resolve) => setTimeout(resolve, 100));
      }
      assert(retries.some((event) => event.kind === "providerError"));
      assert(
        !JSON.stringify(retries).includes("dummy-executor-secret") &&
          !JSON.stringify(retries).includes("fake-token"),
      );
      await call("task.interrupt", { id: retrying.id });
      await wait(retrying.id, ["interrupted"]);
      checks.push(
        "Upstream retries are visible and secrets remain redacted through the real HTTP API",
      );
      if (mode === "fixture") {
        const godot = process.env.BEAVER_TEST_NPR_GODOT;
        assert(godot, "Set BEAVER_TEST_NPR_GODOT to the custom NPR engine");
        const installed = await call<Project>("project.npr.install", {
          id: start.projectId,
          godot,
        });
        assert.equal(installed.npr?.status, "ready");
        await call("settings.clearKey", { slot: "code" });
        const authorSettings = defaultSettings();
        authorSettings.maxParallel = 1;
        authorSettings.tools.codex = process.env.BEAVER_TEST_CODEX ?? "";
        authorSettings.tools.blender = process.env.BEAVER_TEST_BLENDER ?? "";
        authorSettings.tools.godot = godot;
        await call("settings.importLocalCodex", {
          settings: authorSettings,
          keys: {},
        });
        const configured = await call<State>("state");
        assert.equal(
          configured.settings.local.code.model,
          "gpt-6-astra",
          "Preserve the user-selected model",
        );
        const began = performance.now();
        const author = await call<Task>("task.create", {
          projectId: start.projectId,
          prompt:
            "为内置人偶保存一个可以用于 preview 的 NPR 定义文件，复用现有资源，只做最小接入。",
          decompose: false,
          maxMinutes: 6,
        });
        const authored = await wait(
          author.id,
          ["completed", "failed", "interrupted", "awaitingInput", "conflict"],
          420000,
        );
        live = {
          id: author.id,
          status: authored.status,
          milliseconds: Math.round(performance.now() - began),
          error: authored.error,
          report: authored.report,
        };
        assert.equal(
          authored.status,
          "completed",
          authored.error ?? `NPR preparation ended as ${authored.status}`,
        );
        let definition: string | undefined;
        for (const change of authored.changes.filter((change) =>
          change.path.endsWith(".tres"),
        )) {
          const content = await fs.readFile(
            path.join(project.path, change.path),
            "utf8",
          );
          if (content.includes("npr_character_definition.gd")) {
            definition = change.path;
            break;
          }
        }
        assert(definition, "Beaver did not produce an NPR definition");
        checks.push(
          "A real Beaver AI task prepared the diagnostic definition using only application APIs",
        );
        const full = await call<Preview>("workflow.run", {
          id: start.projectId,
          workflow: "npr-character",
          action: "preview",
          definition,
        });
        assert(full.ok, `Default preview failed: ${full.report}`);
        preview = await call<Preview>("workflow.run", {
          id: start.projectId,
          workflow: "npr-character",
          action: "preview",
          definition,
          camera: full.camera,
          grayscale: true,
        });
        assert(preview.ok, `Matched preview failed: ${preview.report}`);
        assert.deepEqual(preview.camera, full.camera);
        assert.equal(preview.modelSha256, full.modelSha256);
        assert.equal(preview.definitionSha256, full.definitionSha256);
        assert.match(preview.modelSha256, /^[a-f0-9]{64}$/);
        assert.equal(preview.screenshots.length, 3);
        assert.equal(preview.grayscaleScreenshots.length, 3);
        for (const relative of [
          ...preview.screenshots,
          ...preview.grayscaleScreenshots,
        ]) {
          const image = await fs.readFile(path.join(project.path, relative));
          assert.equal(image.subarray(1, 4).toString("ascii"), "PNG");
          assert.equal(image.readUInt32BE(16), 768);
          assert.equal(image.readUInt32BE(20), 768);
          if (relative.endsWith("-gray.png"))
            assert.equal(image[25], 0, "Expected grayscale PNG");
        }
        checks.push(
          "NPR API renders the Beaver-authored diagnostic definition with matching cameras, stable source hashes, color and grayscale images",
        );
      }
    } else {
      settings.tools.codex = process.env.BEAVER_TEST_CODEX ?? "";
      await call("settings.importLocalCodex", { settings, keys: {} });
      const configured = await call<State>("state");
      assert.equal(
        configured.settings.local.code.model,
        "gpt-6-astra",
        "Preserve the user-selected model",
      );
      const began = performance.now();
      const created = await call<Task>("task.create", {
        projectId: start.projectId,
        capability: "review",
        prompt: "请检查这个空白工程并简要说明当前状态，不要修改文件。",
        decompose: false,
        maxMinutes: 6,
      });
      const finished = await wait(
        created.id,
        ["completed", "failed", "interrupted", "awaitingInput", "conflict"],
        420000,
      );
      live = {
        id: created.id,
        status: finished.status,
        milliseconds: Math.round(performance.now() - began),
        error: finished.error,
        report: finished.report,
      };
      assert.equal(
        finished.status,
        "completed",
        finished.error ?? `Live task ended as ${finished.status}`,
      );
      assert.equal(
        finished.changes.length,
        0,
        "Review task changed the project",
      );
      checks.push(
        "Packaged host completed a real user-selected model review through Beaver API",
      );
    }
    const proof = {
      passed: true,
      mode,
      executable: start.executable,
      exeSha256: sha,
      checks,
      timings,
      preview,
      live,
      realModelTaskVerified: live?.status === "completed",
      diagnosticAssetOnly: mode === "fixture",
      gameplayVerified: false,
    };
    await fs.writeFile(
      path.join(start.root, "performance-proof.json"),
      JSON.stringify(proof, null, 2),
    );
    console.log(
      JSON.stringify({
        passed: true,
        mode,
        checks,
        proof: path.join(start.root, "performance-proof.json"),
        live,
      }),
    );
  } catch (error) {
    await fs.writeFile(
      path.join(start.root, "performance-proof.json"),
      JSON.stringify(
        {
          passed: false,
          mode,
          checks,
          timings,
          preview,
          live,
          error: String(error).replaceAll(token, "[REDACTED_SECRET]"),
        },
        null,
        2,
      ),
    );
    for (const current of (await call<State>("state")).tasks.filter((t) =>
      ["running", "queued"].includes(t.status),
    ))
      await call("task.interrupt", { id: current.id });
    throw error;
  }
}

main().catch((error: unknown) => {
  const text = String(error).replaceAll(
    process.env.BEAVER_API_TOKEN ?? "unused-token",
    "[REDACTED_SECRET]",
  );
  console.error(text);
  process.exitCode = 1;
});
