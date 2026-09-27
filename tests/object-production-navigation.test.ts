import assert from "node:assert/strict";
import test from "node:test";
import { Children, isValidElement, type ReactNode } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { ObjectProductionTasks } from "../src/ui/object-preview/ObjectProductionTasks";
import { ObjectTaskExecution } from "../src/ui/object-tasks/object-task-execution";
import { parseObjectTaskSnapshot } from "../src/shared/object-task-snapshot";
import { objectTaskSnapshot } from "./fixtures/object-tasks";

function props() {
  return {
    query: { kind: "ready" as const, snapshot: objectTaskSnapshot() },
    manufacture: true,
    select: () => {},
    openObject: () => {},
    execute: () => {},
    refresh: () => {},
  };
}
function click(node: ReactNode, label: string): boolean {
  for (const child of Children.toArray(node)) {
    if (!isValidElement<{ children?: ReactNode; onClick?: () => void }>(child))
      continue;
    if (
      child.type === "button" &&
      renderToStaticMarkup(child).includes(label)
    ) {
      child.props.onClick?.();
      return true;
    }
    if (click(child.props.children, label)) return true;
  }
  return false;
}

test("real iteration selection reaches the same run, stages, execution session and object", () => {
  const input = props();
  let selected = "";
  const objects = ObjectProductionTasks({
    ...input,
    objectId: "hero",
    manufacture: false,
    select: (task) => {
      selected = task.id;
    },
  });
  assert.equal(click(objects, "Hero"), true);
  assert.equal(selected, "medium");
  let returned = "";
  let session: ObjectTaskExecution | undefined;
  const manufacture = ObjectProductionTasks({
    ...input,
    objectId: "hero",
    taskId: selected,
    openObject: (id) => {
      returned = id;
    },
    execute: (task) => {
      session = new ObjectTaskExecution(
        "p",
        input.query.snapshot,
        task.id,
        async () => [],
        async () => true,
      );
    },
  });
  const html = renderToStaticMarkup(manufacture);
  assert.match(html, /run-hero/);
  assert.match(html, /Movement/);
  assert.match(html, /责任粗修：Game/);
  assert.match(html, /依赖：Independent iteration/);
  assert.equal(click(manufacture, "查看执行记录与恢复"), true);
  assert.equal(session?.task.id, "medium");
  assert.deepEqual(
    session?.fineTasks.map((task) => task.id),
    ["fine"],
  );
  assert.equal(click(manufacture, "返回此对象"), true);
  assert.equal(returned, "hero");
});

test("accepted publication snapshots remain readable and navigable", () => {
  const input = props();
  for (const task of input.query.snapshot.tasks)
    if (task.runId === "run-hero") task.status = "accepted";
  for (const run of input.query.snapshot.runs)
    if (run.id === "run-hero") run.status = "accepted";
  const snapshot = parseObjectTaskSnapshot(input.query.snapshot, "p");
  const html = renderToStaticMarkup(
    ObjectProductionTasks({
      ...input,
      query: { kind: "ready", snapshot },
      objectId: "hero",
      taskId: "medium",
    }),
  );
  assert.match(html, /已接受/);
  assert.match(html, /查看执行记录与恢复/);
  assert.throws(
    () => parseObjectTaskSnapshot(snapshot, "other"),
    /不属于当前项目/,
  );
});

test("empty object and stale selection never fall back to another object's task", () => {
  const html = renderToStaticMarkup(
    ObjectProductionTasks({
      ...props(),
      objectId: "no-tasks",
      taskId: "medium",
    }),
  );
  assert.match(html, /尚无制作任务/);
  assert.match(html, /所选迭代不在当前范围/);
  assert.doesNotMatch(html, /查看执行记录与恢复|Movement|Make a hero/);
});

test("query failure exposes retry without stale task controls", () => {
  let retried = false;
  const view = ObjectProductionTasks({
    ...props(),
    query: { kind: "error", message: "offline" },
    taskId: "medium",
    refresh: () => {
      retried = true;
    },
  });
  assert.match(renderToStaticMarkup(view), /offline/);
  assert.equal(click(view, "重试读取任务"), true);
  assert.equal(retried, true);
});
