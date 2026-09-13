import test from "node:test";
import assert from "node:assert/strict";
import {
  conversationText,
  executionStatus,
} from "../src/shared/task-conversation";
import type { TaskEvent } from "../src/shared/types";

const time = "2026-09-12T00:00:00.000Z";
function health(fields: Record<string, unknown>): TaskEvent {
  return {
    time,
    kind: "execution",
    text: JSON.stringify({
      phase: "model",
      lastProgressAt: time,
      activeTools: 0,
      retries: 0,
      pauseAfterSeconds: 300,
      ...fields,
    }),
  };
}

test("execution status distinguishes model waiting, tool execution and compaction without resetting on retry notices", () => {
  const now = Date.parse(time) + 210000;
  const errors = {
    time,
    kind: "providerError",
    text: "upstream busy, retrying",
  };
  assert.match(
    executionStatus([health({ retries: 2 }), errors], now)!,
    /210 秒.*重试 2 次.*300 秒/,
  );
  assert.match(
    executionStatus([health({ phase: "tool", activeTools: 2 })], now)!,
    /工具运行中（2 个）.*暂停模型等待计时/,
  );
  assert.match(
    executionStatus([health({ phase: "compacting" })], now)!,
    /正在压缩上下文/,
  );
  assert.equal(executionStatus([]), undefined);
  assert.equal(
    executionStatus([health({})], now, now),
    undefined,
    "prior execution must not be shown as current progress",
  );
  assert.match(
    executionStatus([health({ phase: "finishing" })], now)!,
    /保存任务结果/,
  );
  assert.equal(
    executionStatus([{ time, kind: "execution", text: "not json" }]),
    undefined,
  );
  assert.equal(
    executionStatus([health({ lastProgressAt: "invalid" })]),
    undefined,
  );
});

test("provider failures and automatic pauses are visible in the normal conversation but raw diagnostics are not", () => {
  const text = conversationText([
    health({}),
    { time, kind: "providerError", text: "upstream busy" },
    { time, kind: "watchdog", text: "workspace preserved" },
    { time, kind: "contextRestart", text: "new session, same baseline" },
    { time, kind: "reasoning", text: "private reasoning" },
  ]);
  assert.match(text, /执行状态\nupstream busy/);
  assert.match(text, /workspace preserved/);
  assert.match(text, /new session, same baseline/);
  assert.doesNotMatch(text, /private reasoning|lastProgressAt/);
});
