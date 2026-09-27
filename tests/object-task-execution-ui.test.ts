import assert from "node:assert/strict";
import test from "node:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { ObjectTaskExecutionDialog } from "../src/ui/object-tasks/ObjectTaskExecutionDialog";
import { ObjectTaskList } from "../src/ui/object-tasks/ObjectTaskList";
import {
  execution,
  interruptReceipt,
  session,
} from "./fixtures/object-attempts";
import { objectTaskSnapshot } from "./fixtures/object-tasks";

test("execution history controls appear only on medium tasks and respect shared busy state", () => {
  const props = { snapshot: objectTaskSnapshot(), onExecution: () => {} };
  const html = renderToStaticMarkup(createElement(ObjectTaskList, props));
  assert.equal((html.match(/>执行记录<\/button>/g) ?? []).length, 2);
  assert.match(html, /aria-label="查看任务 Hero 的执行记录"/);
  assert.match(html, /aria-label="查看任务 Independent iteration 的执行记录"/);
  assert.doesNotMatch(html, /aria-label="查看任务 (Game|Movement) 的执行记录"/);
  const disabled = renderToStaticMarkup(
    createElement(ObjectTaskList, { ...props, executionDisabled: true }),
  );
  assert.equal((disabled.match(/disabled=""/g) ?? []).length, 2);
});

test("execution dialog distinguishes live running, recovery, and frozen results", async () => {
  const cases = [
    { item: execution(), label: "执行中", interrupt: true },
    {
      item: execution("running", "recoveryRequired"),
      label: "需要恢复处理",
      interrupt: false,
    },
    {
      item: execution("awaitingGate"),
      label: "输出已保存，等待验收",
      interrupt: false,
    },
    { item: execution("failed"), label: "执行失败", interrupt: false },
    { item: execution("interrupted"), label: "已中断", interrupt: false },
  ];
  for (const { item, label, interrupt } of cases) {
    const controller = session(async () => [item]);
    await controller.refresh();
    const html = renderToStaticMarkup(
      createElement(ObjectTaskExecutionDialog, {
        session: controller,
        close: () => {},
      }),
    );
    assert.ok(html.includes(label), label);
    assert.equal(html.includes(">中断执行</button>"), interrupt);
    assert.match(html, /Movement/);
    if (item.availability === "recoveryRequired")
      assert.match(html, /重新打开项目不会自动重新执行/);
    if (item.attempt.outputCaptured)
      assert.match(html, /当前接受版本与对象占用状态请查看最终接受与发布记录/);
    assert.doesNotMatch(html, />自动重试<|>继续执行<|>接受<|>发布</);
  }
});

test("ambiguous interruption exposes exact retry while stale rejection asks for refresh", async () => {
  for (const stale of [false, true]) {
    const controller = session(async (method) => {
      if (method === "objectTask.attempts") return [execution()];
      throw new Error(stale ? "OBJECT_ATTEMPT_STALE_TARGET" : "Response lost");
    });
    await controller.refresh();
    await controller.interrupt("attempt-1");
    const html = renderToStaticMarkup(
      createElement(ObjectTaskExecutionDialog, {
        session: controller,
        close: () => {},
      }),
    );
    assert.equal(html.includes(">重试同一中断请求</button>"), !stale);
    assert.match(html, />刷新执行记录<\/button>/);
    if (stale) assert.match(html, /刷新执行记录后重新确认/);
  }
});

test("a late completed turn is shown as awaiting acceptance after interruption acknowledgement", async () => {
  const controller = session(async (method, input) =>
    method === "objectTask.attempts"
      ? [execution()]
      : interruptReceipt(input, "awaitingGate"),
  );
  await controller.refresh();
  await controller.interrupt("attempt-1");
  const html = renderToStaticMarkup(
    createElement(ObjectTaskExecutionDialog, {
      session: controller,
      close: () => {},
    }),
  );
  assert.match(html, /中断请求已确认/);
  assert.match(html, /输出已保存，等待验收/);
  assert.doesNotMatch(html, /class="object-task-status">已中断/);
  assert.doesNotMatch(html, />中断执行<\/button>|>重试同一中断请求<\/button>/);
  assert.match(html, />刷新任务列表<\/button>/);
  assert.match(html, /Implement movement/);
});

test("reopened history renders each frozen definition instead of the current fine task", async () => {
  const old = execution("failed");
  old.definition = {
    title: "Original movement",
    prompt: "First line\n<script>keep as text</script>",
    acceptance: "Original criteria",
    revision: 7,
  };
  const next = execution("awaitingGate");
  next.attempt.target.attemptId = "attempt-2";
  next.definition = {
    title: "Retry movement",
    prompt: "Retry requirement",
    acceptance: "",
    revision: 8,
  };
  for (let reopen = 0; reopen < 2; reopen++) {
    const calls: string[] = [];
    const controller = session(async (method) => {
      calls.push(method);
      return [old, next];
    });
    assert.equal(await controller.refresh(), true);
    const html = renderToStaticMarkup(
      createElement(ObjectTaskExecutionDialog, {
        session: controller,
        close: () => {},
      }),
    );
    assert.match(html, /Original movement/);
    assert.match(html, /Retry movement/);
    assert.match(
      html,
      /First line\n&lt;script&gt;keep as text&lt;\/script&gt;/,
    );
    assert.match(html, /Original criteria/);
    assert.match(html, /Retry requirement/);
    assert.match(html, /未填写验收要求/);
    assert.match(html, /定义 revision：7/);
    assert.match(html, /定义 revision：8/);
    assert.match(html, /尝试 ID：attempt-1/);
    assert.match(html, /尝试 ID：attempt-2/);
    assert.doesNotMatch(html, /<h3>Movement<\/h3>|<script>/);
    assert.deepEqual(calls, ["objectTask.attempts"]);
    controller.cancel();
  }
});
