import test from "node:test";
import assert from "node:assert/strict";
import { paginateText } from "../src/shared/pagination";
import { conversationText } from "../src/shared/task-conversation";

test("pagination preserves every character including Unicode and blank lines", () => {
  for (const text of [
    "",
    "中文剧情🙂".repeat(500),
    "\n".repeat(100),
    "long code ".repeat(1000),
    "a\nb\n\nc\t尾部\n",
  ]) {
    for (const [columns, rows] of [
      [20, 3],
      [1, 1],
      [30, 10],
    ]) {
      const pages = paginateText(text, columns!, rows!);
      assert.equal(pages.join(""), text);
      assert.ok(pages.length > 0);
      assert.ok(pages.every((page) => page.split("\n").length <= rows! + 1));
    }
  }
});
test("streaming fragments form one message and final report is not duplicated", () => {
  const events = [
    { kind: "user", text: "保留角色", time: "" },
    { kind: "assistant", text: "已", time: "" },
    { kind: "assistant", text: "保留", time: "" },
  ];
  assert.equal(
    conversationText(events, "已保留"),
    "你\n保留角色\n\nCodex\n已保留",
  );
  assert.match(conversationText([], "独立汇报"), /独立汇报/);
});
