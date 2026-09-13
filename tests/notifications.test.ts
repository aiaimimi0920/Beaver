import test from "node:test";
import assert from "node:assert/strict";
import {
  appendNotification,
  notificationLifetime,
  errorMessage,
  type Notification,
} from "../src/ui/notification-state";

test("notifications append in order and deduplicate matching feedback without losing errors", () => {
  const first: Notification = {
    id: 1,
    tone: "error",
    text: "工具不可用",
    count: 1,
  };
  const second: Notification = {
    id: 2,
    tone: "success",
    text: "设置已保存",
    count: 1,
  };
  const state = appendNotification([first], second);
  assert.deepEqual(
    state.map((item) => item.id),
    [1, 2],
  );
  const repeated = appendNotification(state, { ...first, id: 3 });
  assert.deepEqual(
    repeated.map((item) => [item.id, item.count]),
    [
      [1, 2],
      [2, 1],
    ],
  );
  assert.equal(first.count, 1);
  assert.equal(appendNotification(state, { ...second, text: " " }), state);
  assert.equal(
    appendNotification(state, { ...first, id: 4, tone: "warning" }).length,
    3,
  );
});

test("notification errors remain readable until dismissed; transient tones follow Loom timings", () => {
  assert.equal(notificationLifetime("error"), null);
  assert.equal(notificationLifetime("warning"), 4200);
  assert.equal(notificationLifetime("success"), 3200);
  assert.equal(notificationLifetime("info"), 3200);
  assert.equal(errorMessage(new Error("无法导出")), "无法导出");
  assert.equal(errorMessage("窗口不可用"), "窗口不可用");
});
