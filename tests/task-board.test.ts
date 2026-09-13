import test from "node:test";
import assert from "node:assert/strict";
import {
  boardColumns,
  taskColumn,
  taskDirection,
  directionSchema,
} from "../src/shared/task-board";
import type { Task } from "../src/shared/types";

test("board preserves real execution and acceptance states", () => {
  for (const [status, column] of Object.entries({
    queued: "queued",
    running: "running",
    awaitingInput: "attention",
    failed: "attention",
    conflict: "attention",
    interrupted: "attention",
    completed: "review",
    rolledBack: "done",
  })) {
    assert.equal(taskColumn({ status } as Task), column);
    assert.ok(boardColumns.some((c) => c.id === column));
  }
  assert.equal(
    taskColumn({ status: "completed", accepted: true } as Task),
    "done",
  );
});
test("direction is explicit and does not guess from task prompt", () => {
  assert.equal(
    taskDirection({ capability: "code", prompt: "音乐剧情" } as Task),
    "general",
  );
  assert.equal(taskDirection({ capability: "review" } as Task), "review");
  assert.equal(
    taskDirection({ capability: "code", direction: "visual" } as Task),
    "visual",
  );
  assert.equal(directionSchema.safeParse("unknown").success, false);
});
