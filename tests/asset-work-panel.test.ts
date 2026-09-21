import assert from "node:assert/strict";
import test from "node:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import type { AssetWork, WorkAttempt } from "../src/shared/asset-work";
import { WorkPanel } from "../src/ui/asset-task/WorkPanel";

function render(inputFiles: WorkAttempt["inputFiles"]): string {
  const definition = { title: "Model", goal: "Edit", acceptance: "Review" };
  const work: AssetWork = {
    subtasks: [
      {
        id: "step",
        stageId: "model",
        definition,
        status: "completed",
        lastAttemptId: "attempt",
        note: "",
      },
    ],
    attempts: [
      {
        id: "attempt",
        subtaskId: "step",
        stageId: "model",
        definition,
        status: "completed",
        threadId: "thread",
        turnId: "turn",
        sessionId: null,
        checkpoint: null,
        endCheckpoint: null,
        assetRevision: 1,
        inputCandidates: [],
        inputFiles,
        inputs: { sha256: "model-reported-only" },
        outputs: {},
        tools: [],
        summary: "Done",
        recoveryNote: "",
        startedAt: "2026-09-14",
        endedAt: "2026-09-14",
      },
    ],
  };
  return renderToStaticMarkup(
    createElement(WorkPanel, { work, currentStage: "model" }),
  );
}

test("work view distinguishes missing historical manifests from declared empty inputs", () => {
  for (const manifest of [undefined, null]) {
    const html = render(manifest);
    assert.match(html, /历史尝试未采集输入文件清单/);
    assert.doesNotMatch(html, /本次明确声明无文件输入/);
  }
  const html = render([]);
  assert.match(html, /本次明确声明无文件输入/);
  assert.doesNotMatch(html, /历史尝试未采集输入文件清单/);
});

test("work view separates host hashes from model reports and escapes file names", () => {
  const html = render([
    { path: "model.blend", role: "source", sha256: "a".repeat(64) },
    { path: "refs/<script>.png", role: "dependency", sha256: "b".repeat(64) },
  ]);
  assert.match(html, /可编辑源文件/);
  assert.match(html, /只读依赖/);
  assert.match(html, /refs\/&lt;script&gt;\.png/);
  assert.doesNotMatch(html, /<script>/);
  const evidence = html.slice(
    html.indexOf("Beaver 保存的输入文件版本"),
    html.indexOf("Codex 报告的输入、输出与工具"),
  );
  assert.ok(evidence.includes("a".repeat(64)));
  assert.ok(evidence.includes("b".repeat(64)));
  assert.ok(!evidence.includes("model-reported-only"));
  assert.match(html, /清单完整性仍需检查/);
});
