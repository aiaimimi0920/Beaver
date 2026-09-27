import assert from "node:assert/strict";
import test from "node:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { ObjectTaskDispatchDialog } from "../src/ui/object-tasks/ObjectTaskDispatchDialog";
import { ObjectTaskList } from "../src/ui/object-tasks/ObjectTaskList";
import {
  dispatchControl,
  dispatchReceipt,
  dispatchSession,
} from "./fixtures/object-task-dispatch";
import { objectTaskSnapshot } from "./fixtures/object-tasks";

test("dispatch actions belong to coarse and live mediums and preserve lifecycle labels", () => {
  const snapshot = objectTaskSnapshot();
  snapshot.dispatchControls = [dispatchControl()];
  snapshot.tasks.find((task) => task.id === "medium")!.status = "running";
  const props = { snapshot, onDispatch: () => {} };
  const html = renderToStaticMarkup(createElement(ObjectTaskList, props));
  assert.equal((html.match(/>派发控制<\/button>/g) ?? []).length, 3);
  assert.match(html, /aria-label="管理任务 Hero 的派发"/);
  assert.match(html, /aria-label="管理任务 Game 的派发"/);
  assert.doesNotMatch(html, /aria-label="管理任务 Movement 的派发"/);
  assert.match(html, /派发已暂停/);
  assert.match(html, /class="object-task-status">执行中/);
  const disabled = renderToStaticMarkup(
    createElement(ObjectTaskList, { ...props, dispatchDisabled: true }),
  );
  assert.equal((disabled.match(/disabled=""/g) ?? []).length, 3);
  snapshot.tasks.find((task) => task.id === "medium")!.status = "cancelled";
  const cancelled = renderToStaticMarkup(createElement(ObjectTaskList, props));
  assert.doesNotMatch(cancelled, /aria-label="管理任务 Hero 的派发"/);
});

test("dispatch dialog explains queued eligibility and in-flight continuation before confirmation", async () => {
  for (const paused of [false, true]) {
    const snapshot = objectTaskSnapshot();
    snapshot.dispatchControls = paused ? [dispatchControl()] : [];
    const session = dispatchSession(async () => snapshot);
    await session.refresh();
    const html = renderToStaticMarkup(
      createElement(ObjectTaskDispatchDialog, { session, close: () => {} }),
    );
    assert.ok(html.includes(paused ? "确认解除暂停" : "确认暂停派发"));
    assert.match(html, /已经派发的准备和执行继续完成/);
    assert.match(html, /同对象的后续迭代继续等待/);
    assert.match(html, /解除暂停只恢复排队资格/);
  }
});

test("ambiguous control exposes exact retry; explicit conflict requires refresh", async () => {
  for (const stale of [false, true]) {
    const session = dispatchSession(async (method) => {
      if (method === "objectTask.snapshot") return objectTaskSnapshot();
      throw new Error(
        stale ? "OBJECT_TASK_DISPATCH_REVISION_CONFLICT" : "Response lost",
      );
    });
    await session.refresh();
    await session.confirm();
    const html = renderToStaticMarkup(
      createElement(ObjectTaskDispatchDialog, { session, close: () => {} }),
    );
    assert.equal(html.includes(">重试同一派发控制请求</button>"), !stale);
    if (stale) assert.match(html, /刷新派发状态后重新确认/);
  }
});

test("receipt display cannot imply an old pause response is the latest persisted state", async () => {
  const session = dispatchSession(async (method, input) =>
    method === "objectTask.snapshot"
      ? objectTaskSnapshot()
      : dispatchReceipt(input),
  );
  await session.refresh();
  await session.confirm();
  const html = renderToStaticMarkup(
    createElement(ObjectTaskDispatchDialog, { session, close: () => {} }),
  );
  assert.match(html, /暂停派发请求已确认/);
  assert.match(html, /回执记录本次操作结果，最新状态请查看任务列表/);
  assert.match(html, />刷新任务列表<\/button>/);
  assert.doesNotMatch(html, /当前派发状态|>确认暂停派发<|>确认解除暂停</);
});
