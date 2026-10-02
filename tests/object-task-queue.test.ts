import assert from "node:assert/strict";
import test from "node:test";
import {
  createElement,
  isValidElement,
  Children,
  type ReactNode,
  type ReactElement,
  type ComponentProps,
} from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { ObjectTaskQueueSession } from "../src/ui/object-tasks/object-task-queue-session";
import { ObjectTaskQueueList } from "../src/ui/object-tasks/ObjectTaskQueueList";
import {
  parseQueueView,
  previewQueueMove,
  queueMoveSchema,
  queueReceiptSchema,
  type QueueView,
  type QueueMove,
} from "../src/shared/object-task-queue";

const view = (): QueueView => ({
  projectId: "p",
  version: "a".repeat(64),
  items: [
    {
      taskId: "a",
      objectId: "hero",
      title: "First",
      state: "queued",
      blockers: [],
    },
    {
      taskId: "b",
      objectId: "hero",
      title: "Second",
      state: "queued",
      blockers: ["earlierQueued"],
    },
    {
      taskId: "c",
      objectId: "rival",
      title: "Independent",
      state: "queued",
      blockers: [],
    },
  ],
});
function receipt(request: QueueMove) {
  const items = view().items.filter((item) => item.taskId !== request.taskId);
  const index =
    request.previousTaskId === null
      ? 0
      : items.findIndex((item) => item.taskId === request.previousTaskId) + 1;
  items.splice(
    index,
    0,
    view().items.find((item) => item.taskId === request.taskId)!,
  );
  return { request, result: { ...view(), version: "b".repeat(64), items } };
}
function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}

test("review and receipt reject cross-project, duplicate, unknown and misplaced results", () => {
  assert.deepEqual(parseQueueView(view(), "p"), view());
  assert.throws(() => parseQueueView(view(), "other"));
  assert.throws(() =>
    parseQueueView(
      { ...view(), items: [view().items[0], view().items[0]] },
      "p",
    ),
  );
  assert.throws(() => parseQueueView({ ...view(), extra: true }, "p"));
  const request = queueMoveSchema.parse({
    projectId: "p",
    requestId: "move",
    expectedVersion: view().version,
    taskId: "c",
    previousTaskId: null,
    nextTaskId: "a",
  });
  assert.ok(queueReceiptSchema.safeParse(receipt(request)).success);
  assert.equal(
    queueReceiptSchema.safeParse({ ...receipt(request), result: view() })
      .success,
    false,
  );
  assert.equal(
    queueMoveSchema.safeParse({ ...request, nextTaskId: undefined }).success,
    false,
  );
});

test("preview protects each blocked object head while permitting independent work", () => {
  for (const blocker of [
    "dependencies",
    "paused",
    "coarsePaused",
    "objectHeld",
  ] as const) {
    const current = view();
    assert.ok(current.items[0]);
    current.items[0].blockers = [blocker];
    assert.throws(() => previewQueueMove(current, "b", 0), /不能绕过/);
    assert.throws(() => previewQueueMove(current, "a", 2), /不能绕过/);
    assert.equal(previewQueueMove(current, "c", 0).nextTaskId, "a");
  }
  const active = view();
  assert.ok(active.items[0]);
  active.items[0].state = "running";
  assert.throws(() => previewQueueMove(active, "a", 1), /只能移动/);
});

test("confirmation is explicit and ambiguous retries preserve the exact cloned request", async () => {
  const submitted: QueueMove[] = [];
  let reads = 0;
  const session = new ObjectTaskQueueSession(
    "p",
    async (method, input) => {
      if (method === "objectTask.queueView") {
        reads++;
        return view();
      }
      const request = queueMoveSchema.parse(input);
      submitted.push(request);
      if (submitted.length === 1) {
        (input as QueueMove).taskId = "mutated";
        throw new Error("response lost");
      }
      return receipt(request);
    },
    () => "stable",
  );
  await session.refresh();
  session.preview("c", 0);
  assert.equal(submitted.length, 0);
  await session.confirm();
  assert.equal(session.getSnapshot().retry, true);
  session.notified();
  await session.refresh();
  session.preview("b", 0);
  assert.equal(reads, 1);
  await session.retry();
  assert.deepEqual(submitted[0], submitted[1]);
  assert.equal(session.getSnapshot().receipt?.request.taskId, "c");
  assert.equal(session.getSnapshot().view?.version, view().version);
  assert.equal(session.getSnapshot().phase, "ready");
});

test("concurrent queue changes require refresh and a new confirmation", async () => {
  let calls = 0;
  const session = new ObjectTaskQueueSession("p", async (method) => {
    if (method === "objectTask.queueView") return view();
    calls++;
    throw new Error("OBJECT_TASK_QUEUE_VERSION_CONFLICT");
  });
  await session.refresh();
  session.preview("c", 0);
  await session.confirm();
  assert.equal(session.getSnapshot().retry, false);
  await session.retry();
  assert.equal(calls, 1);
  session.preview("b", 0);
  await session.confirm();
  assert.equal(calls, 1);
  await session.refresh();
  session.preview("c", 0);
  await session.confirm();
  assert.equal(calls, 2);
});

test("invalid receipt retains original request while post-success refresh failure cannot repeat the mutation", async () => {
  let mutation = 0,
    reads = 0;
  const session = new ObjectTaskQueueSession("p", async (method, input) => {
    if (method === "objectTask.queueView") {
      if (++reads > 1) throw new Error("offline");
      return view();
    }
    mutation++;
    const result = receipt(queueMoveSchema.parse(input));
    if (mutation === 1) result.request.requestId = "unrelated";
    return result;
  });
  await session.refresh();
  session.preview("c", 0);
  await session.confirm();
  assert.equal(session.getSnapshot().retry, true);
  await session.retry();
  assert.ok(session.getSnapshot().receipt);
  assert.equal(session.getSnapshot().phase, "failed");
  assert.equal(session.getSnapshot().retry, false);
  await session.retry();
  assert.equal(mutation, 2);
});

test("old reads and mutation completions are ignored after disposal", async () => {
  const pending = deferred<unknown>();
  const session = new ObjectTaskQueueSession("p", async () => pending.promise);
  const read = session.refresh();
  session.cancel();
  pending.resolve(view());
  await read;
  assert.equal(session.getSnapshot().view, null);
  const completion = deferred<unknown>();
  let sent: QueueMove | undefined;
  const writer = new ObjectTaskQueueSession("p", async (method, input) => {
    if (method === "objectTask.queueView") return view();
    sent = queueMoveSchema.parse(input);
    return completion.promise;
  });
  await writer.refresh();
  writer.preview("c", 0);
  const submit = writer.confirm();
  writer.cancel();
  completion.resolve(receipt(sent!));
  await submit;
  assert.equal(writer.getSnapshot().receipt, null);
});

test("published tasks reload as accepted history while waiting tasks remain reorderable", async () => {
  const current = view();
  const published = {
    taskId: "published",
    objectId: "crate",
    title: "Published crate",
    state: "awaitingAcceptance" as QueueView["items"][number]["state"],
    blockers: [],
  };
  current.items.unshift(published);
  const session = new ObjectTaskQueueSession("p", async (method) => {
    assert.equal(method, "objectTask.queueView");
    return structuredClone(current);
  });
  await session.refresh();
  assert.equal(session.getSnapshot().phase, "ready");
  published.state = "accepted";
  current.version = "b".repeat(64);
  await session.refresh();
  const state = session.getSnapshot();
  assert.equal(state.phase, "ready");
  assert.equal(state.error, "");
  assert.deepEqual(state.view, current);
  assert(state.view);
  const html = renderToStaticMarkup(
    createElement(ObjectTaskQueueList, {
      items: state.view.items,
      disabled: false,
      dragId: null,
      start: () => {},
      preview: session.preview,
      drop: () => {},
      end: () => {},
    }),
  );
  assert.match(html, /Published crate · 已验收/);
  assert.doesNotMatch(html, /aria-label="(?:上移|下移) Published crate"/);
  assert.equal((html.match(/draggable="true"/g) ?? []).length, 3);
  session.preview("published", 0);
  assert.equal(session.getSnapshot().proposal, null);
  assert.match(session.getSnapshot().error, /只能移动待执行任务/);
  session.preview("c", 0);
  assert.equal(session.getSnapshot().error, "");
  assert.equal(session.getSnapshot().proposal?.previousTaskId, null);
  assert.equal(session.getSnapshot().proposal?.nextTaskId, "a");
});

test("enqueue has its own exact retry and suppresses double submission", async () => {
  const sent: unknown[] = [];
  const pending = deferred<unknown>();
  const session = new ObjectTaskQueueSession("p", async (method, input) => {
    if (method === "objectTask.queueView") return view();
    assert.equal(method, "objectTask.enqueue");
    sent.push(structuredClone(input));
    if (sent.length === 1) throw new Error("lost");
    return pending.promise;
  });
  await session.refresh();
  await session.enqueue("new");
  const retry = session.retry();
  await session.enqueue("different");
  await session.retry();
  pending.resolve([]);
  await retry;
  assert.deepEqual(sent, [
    { projectId: "p", taskIds: ["new"] },
    { projectId: "p", taskIds: ["new"] },
  ]);
});

function elements(node: ReactNode): ReactElement<{ children?: ReactNode }>[] {
  return Children.toArray(node).flatMap((child) =>
    isValidElement<{ children?: ReactNode }>(child)
      ? [child, ...elements(child.props.children)]
      : [],
  );
}
test("production queue actions preview keyboard moves and explain blockers without moving active rows", async () => {
  const current = view();
  assert.ok(current.items[0]);
  current.items[0].blockers = ["dependencies"];
  current.items.push({
    taskId: "active",
    objectId: "other",
    title: "Active",
    state: "running",
    blockers: [],
  });
  const session = new ObjectTaskQueueSession("p", async () => current);
  await session.refresh();
  const props = {
    items: current.items,
    disabled: false,
    dragId: null,
    start: () => {},
    preview: session.preview,
    drop: () => {},
    end: () => {},
  };
  const html = renderToStaticMarkup(createElement(ObjectTaskQueueList, props));
  assert.match(html, /依赖尚未验收/);
  assert.match(html, /等待同对象前序任务/);
  assert.match(html, /执行中/);
  assert.doesNotMatch(html, /aria-label="上移 Active"/);
  const button = elements(ObjectTaskQueueList(props)).find(
    (element) =>
      element.type === "button" &&
      (element.props as ComponentProps<"button">)["aria-label"] ===
        "上移 Independent",
  ) as ReactElement<ComponentProps<"button">>;
  assert.ok(button);
  // Production callback is intentionally event-independent.
  (button.props.onClick as () => void)();
  assert.equal(session.getSnapshot().proposal?.previousTaskId, "a");
  assert.equal(session.getSnapshot().proposal?.nextTaskId, "b");
  const disabled = renderToStaticMarkup(
    createElement(ObjectTaskQueueList, { ...props, disabled: true }),
  );
  assert.equal((disabled.match(/draggable="false"/g) ?? []).length, 3);
});
