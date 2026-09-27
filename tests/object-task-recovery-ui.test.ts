import assert from "node:assert/strict";
import test from "node:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { ObjectTaskExecutionDialog } from "../src/ui/object-tasks/ObjectTaskExecutionDialog";
import { ObjectTaskRecoveryPanel } from "../src/ui/object-tasks/ObjectTaskRecoveryPanel";
import {
  execution,
  session as executionSession,
} from "./fixtures/object-attempts";
import {
  pendingView,
  recoveryReceipt,
  recoverySession,
  recoveryView,
  verifiedView,
} from "./fixtures/object-recovery";

test("unprepared records explain why verification is unavailable", async () => {
  const session = recoverySession(async () => null);
  await session.refresh();
  const html = renderToStaticMarkup(
    createElement(ObjectTaskRecoveryPanel, { session }),
  );
  assert.match(html, /尚无工作区准备记录/);
  assert.match(html, /<button disabled="">核验当前记录与工作区<\/button>/);
  assert.match(html, /<button>刷新核验记录<\/button>/);
  assert.doesNotMatch(html, /最近核验报告/);
});

test("the production execution dialog displays verification evidence and later pause expiry", async () => {
  const view = verifiedView();
  const session = executionSession(async (method) =>
    method === "objectTask.attempts" ? [execution("failed")] : view,
  );
  await session.refresh();
  await session.recovery.refresh();
  const render = () =>
    renderToStaticMarkup(
      createElement(ObjectTaskExecutionDialog, { session, close: () => {} }),
    );
  let html = render();
  assert.match(html, /aria-label="恢复核验"/);
  for (const label of [
    "已有停止与输出保存记录",
    "冻结内容已核验",
    "与保存的检查点一致",
    "查询未重新检查文件",
    "取消并保留成果",
  ])
    assert.ok(html.includes(label), label);
  assert.doesNotMatch(html, />恢复执行<|>释放对象<|>接受版本<|>发布</);
  view.paused = true;
  view.target.controlRevision++;
  view.reportMatchesRecords = false;
  view.canDispose = false;
  await session.recovery.refresh();
  html = render();
  assert.match(html, /调度已暂停/);
  assert.match(html, /核验时的暂停状态<\/dt><dd>未暂停/);
  assert.match(html, /报告已过期/);
  assert.doesNotMatch(html, /本次证据满足后续人工处置条件/);
});

test("pending and unknown outcomes expose exact retry without presenting a completed report", async () => {
  for (const pending of [false, true]) {
    const session = recoverySession(async (method) => {
      if (method === "objectTask.recovery")
        return pending ? pendingView() : recoveryView();
      throw new Error("response lost");
    });
    await session.refresh();
    if (!pending) await session.verify();
    const html = renderToStaticMarkup(
      createElement(ObjectTaskRecoveryPanel, { session }),
    );
    assert.match(html, /重试将沿用原请求标识和目标/);
    assert.match(html, /<button>重试同一核验请求<\/button>/);
    assert.doesNotMatch(html, /最近核验报告/);
  }
});

test("a retained report stays visible when the current record query fails", async () => {
  let queries = 0;
  const session = recoverySession(async (method, input) => {
    if (method === "objectTask.verifyRecovery") return recoveryReceipt(input);
    if (++queries === 1) return recoveryView();
    throw new Error("query offline");
  });
  await session.refresh();
  await session.verify();
  const html = renderToStaticMarkup(
    createElement(ObjectTaskRecoveryPanel, { session }),
  );
  assert.match(html, /核验回执已保留/);
  assert.match(html, /最近核验报告（第 1 次）/);
  assert.match(html, /当前记录尚未确认/);
  assert.match(html, /<button>刷新核验记录<\/button>/);
  assert.doesNotMatch(html, /重试同一核验请求|本次证据满足后续人工处置条件/);
});

test("writer uncertainty and integrity issues remain visible instead of offering disposition", async () => {
  const view = verifiedView();
  const report = view.operation!.result!;
  report.writerStatus = "unconfirmed";
  report.contentStatus = "invalid";
  report.workspaceStatus = "drifted";
  report.issues = [
    "CONTENT_HASH_MISMATCH: partial.txt",
    "<script>not markup</script>",
  ];
  view.canDispose = false;
  const session = recoverySession(async () => view);
  await session.refresh();
  const html = renderToStaticMarkup(
    createElement(ObjectTaskRecoveryPanel, { session }),
  );
  for (const label of [
    "尚未确认停止",
    "冻结内容无效",
    "已偏离保存的检查点",
    "CONTENT_HASH_MISMATCH",
    "当前证据不足",
  ])
    assert.ok(html.includes(label), label);
  assert.match(html, /&lt;script&gt;not markup&lt;\/script&gt;/);
  assert.doesNotMatch(html, /<script>|本次证据满足后续人工处置条件/);
});
