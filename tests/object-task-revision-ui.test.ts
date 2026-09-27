import assert from "node:assert/strict";
import test from "node:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { ObjectTaskDefinitionEditor } from "../src/ui/object-tasks/ObjectTaskDefinitionEditor";
import { ObjectTaskList } from "../src/ui/object-tasks/ObjectTaskList";
import { ObjectTaskRevisionDialog } from "../src/ui/object-tasks/ObjectTaskRevisionDialog";
import { ObjectTaskRevisionHistory } from "../src/ui/object-tasks/ObjectTaskRevisionHistory";
import { ObjectTaskRevision } from "../src/ui/object-tasks/object-task-revision";
import { ObjectTaskRevisionHistoryStore } from "../src/ui/object-tasks/object-task-revision-history";
import {
  editedRevision,
  revisionDefinition,
  revisionReceipt as receipt,
  revisionReason,
} from "./fixtures/object-task-revisions";
import {
  cancelledObjectTaskSnapshot,
  objectTaskSnapshot,
} from "./fixtures/object-tasks";

const renderDialog = (session: ObjectTaskRevision) =>
  renderToStaticMarkup(
    createElement(ObjectTaskRevisionDialog, { session, close: () => {} }),
  );

test("revision UI separates editable input from before/after review and explicit confirmation", () => {
  const session = editedRevision(async () => {
    throw new Error("Unexpected API call");
  });
  const editing = renderDialog(session);
  assert.match(editing, /role="dialog"/);
  assert.match(editing, /<input[^>]*value="Playable hero"/);
  assert.match(editing, /<textarea[^>]*>Add keyboard movement<\/textarea>/);
  assert.match(editing, /核对修订/);
  assert.doesNotMatch(editing, /<button[^>]*>确认修订<\/button>/);
  assert.equal(session.review(), true);
  const reviewing = renderDialog(session);
  assert.match(reviewing, /aria-label="修订前"/);
  assert.match(reviewing, /aria-label="修订后"/);
  for (const text of [
    "Make a hero",
    revisionDefinition.prompt,
    revisionDefinition.acceptance,
    revisionReason,
    "项目所有者（owner）",
    "确认修订",
    "返回编辑",
  ]) {
    assert.ok(reviewing.includes(text), text);
  }
  assert.doesNotMatch(reviewing, /<textarea|<input/);
  const impact = reviewing.match(
    /aria-label="本次修订影响范围">(.*?)<\/section>/,
  )?.[1];
  assert.ok(impact);
  assert.match(impact, /影响范围：2 项任务/);
  assert.match(impact, /<code>medium<\/code>/);
  assert.match(impact, /<code>fine<\/code>/);
  assert.doesNotMatch(impact, /independent|coarse/);
});

test("cancelled tasks expose read-only history while planned tasks expose revision controls", () => {
  const snapshot = cancelledObjectTaskSnapshot();
  const list = renderToStaticMarkup(
    createElement(ObjectTaskList, { snapshot, onRevise: () => {} }),
  );
  assert.match(list, /aria-label="修订任务 Game 的定义"/);
  assert.match(list, /aria-label="查看任务 Hero 的修订历史"/);
  assert.match(list, /<dt>任务版本<\/dt><dd>1<\/dd>/);
  assert.doesNotMatch(list, /aria-label="修订任务 Hero 的定义"/);
  const disabled = renderToStaticMarkup(
    createElement(ObjectTaskList, {
      snapshot,
      onRevise: () => {},
      revisionDisabled: true,
    }),
  );
  assert.equal((disabled.match(/<button[^>]*disabled=""/g) ?? []).length, 4);
  const session = new ObjectTaskRevision(
    "p",
    snapshot,
    "medium",
    async () => [],
    async () => true,
  );
  const dialog = renderDialog(session);
  assert.match(dialog, /当前任务定义/);
  assert.match(dialog, /aria-label="任务定义修订历史"/);
  assert.doesNotMatch(dialog, /<textarea|核对修订|确认修订|本次修订影响范围/);
});

test("conflict UI shows original, local and remote definitions and requires explicit adoption", async () => {
  const latest = objectTaskSnapshot();
  latest.planRevision = 2;
  Object.assign(
    latest.tasks.find((task) => task.id === "medium")!,
    { title: "Remote hero", prompt: "Remote controls", revision: 1 },
  );
  const session = editedRevision(async (method) => {
    if (method === "objectTask.snapshot") return latest;
    throw new Error("OBJECT_TASK_REVISION_CONFLICT");
  });
  session.review();
  await session.submit();
  const conflict = renderDialog(session);
  for (const text of [
    "Make a hero",
    revisionDefinition.prompt,
    "Remote hero",
    "Remote controls",
    "最新远端定义",
    "最新计划版本 2",
    "保留本地修改并采用最新版本",
  ]) {
    assert.ok(conflict.includes(text), text);
  }
  assert.doesNotMatch(conflict, /<button[^>]*>确认修订<\/button>/);
  assert.equal(session.rebase(), true);
  assert.equal(session.review(), true);
  const rebased = renderDialog(session);
  assert.match(rebased, /aria-label="修订前"[^]*?Remote controls/);
  assert.match(rebased, /已核对计划版本 2/);
  assert.doesNotMatch(rebased, /Make a hero/);
});

test("confirmed write with refresh failure offers refresh without a resubmission control", async () => {
  const session = editedRevision(
    async (method) => {
      if (method === "objectTask.revisions") throw new Error("History offline");
      return receipt;
    },
    { refresh: async () => false },
  );
  session.review();
  await session.submit();
  const html = renderDialog(session);
  assert.match(html, /修订已确认，任务版本 1/);
  assert.match(html, /回执已确认，但刷新失败/);
  assert.match(html, /重试刷新任务列表/);
  assert.match(html, /修订历史读取失败：History offline/);
  assert.doesNotMatch(
    html,
    /<button[^>]*>确认修订<\/button>|使用原请求重试修订/,
  );
});

test("history renders immutable audit metadata and retains it beside retryable read errors", async () => {
  let fail = false;
  const store = new ObjectTaskRevisionHistoryStore("p", "medium", async () => {
    if (fail) throw new Error("History offline");
    return [receipt];
  });
  assert.equal(await store.load(), true);
  fail = true;
  assert.equal(await store.load(), false);
  const html = renderToStaticMarkup(
    createElement(ObjectTaskRevisionHistory, { store }),
  );
  for (const text of [
    "任务版本 0 → 1",
    "计划版本 1 → 2",
    "项目所有者（owner）",
    receipt.createdAt,
    receipt.requestId,
    revisionReason,
    "Make a hero",
    revisionDefinition.prompt,
    "当时的影响任务",
    "History offline",
    "重新读取修订历史",
  ]) {
    assert.ok(html.includes(text), text);
  }
  assert.match(html, /<time dateTime="2026-09-23T12:00:00.000Z"/);
  assert.match(html, /<code>fine<\/code>/);
  assert.match(html, /<code>medium<\/code>/);
  assert.doesNotMatch(html, /尚无定义修订记录/);
});

test("selected cancelled and unavailable dependencies remain removable while new cancelled choices are disabled", () => {
  const snapshot = objectTaskSnapshot();
  for (const task of snapshot.tasks) {
    if (["coarse", "independent"].includes(task.id)) task.status = "cancelled";
  }
  const html = renderToStaticMarkup(
    createElement(ObjectTaskDefinitionEditor, {
      snapshot,
      taskId: "medium",
      definition: {
        ...revisionDefinition,
        dependsOn: ["independent", "missing"],
      },
      reason: revisionReason,
      onDefinition: () => {},
      onReason: () => {},
    }),
  );
  const choices = [
    ...html.matchAll(
      /<label>(<input[^>]*type="checkbox"[^>]*\/>)[^]*?<span>(.*?)<\/span><\/label>/g,
    ),
  ];
  for (const id of ["independent", "missing"]) {
    const input = choices.find((choice) =>
      choice[2]?.includes(`<code>${id}</code>`),
    )?.[1];
    assert.ok(input, id);
    assert.match(input, /checked=""/);
    assert.doesNotMatch(input, /disabled/);
  }
  const unselected = choices.find((choice) =>
    choice[2]?.includes("<code>coarse</code>"),
  )?.[1];
  assert.ok(unselected);
  assert.match(unselected, /disabled=""/);
  assert.doesNotMatch(unselected, /checked/);
});
