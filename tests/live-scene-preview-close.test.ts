import assert from "node:assert/strict";
import test from "node:test";
import {
  LiveScenePreview,
  initialCamera,
} from "../src/ui/object-preview/live-scene-preview";

const source = {
  projectId: "p",
  runId: "r",
  snapshotId: "snapshot",
  sessionId: "s",
  status: "ready",
  error: null,
  frame: null,
};
const receipt = { projectId: "p", sessionId: "s", closed: true };
const settle = () => new Promise<void>((done) => setImmediate(done));
function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}

test("explicit close stays pending until the exact worker receipt confirms, coalesces clicks, and blocks new work", async () => {
  const finishing = deferred<unknown>();
  const requests: { method: string; input: unknown }[] = [];
  const session = new LiveScenePreview(
    "p",
    "r",
    "snapshot",
    async (method, input) => {
      requests.push({ method, input });
      if (!method.endsWith("close")) return source;
      return requests.filter((r) => r.method.endsWith("close")).length === 1
        ? { ...receipt, closed: false }
        : finishing.promise;
    },
  );
  await session.refresh();
  const closing = session.requestClose();
  assert.equal(session.requestClose(), closing);
  assert.equal(session.getSnapshot().closeStatus, "pending");
  await session.refresh();
  session.setCamera({ ...initialCamera(), yaw: 5 });
  session.setResolution("720p");
  session.setFrozen(true);
  await assert.rejects(session.capture(), /PREVIEW_VIEW_PENDING/);
  assert.equal(requests.length, 2);
  assert.equal(session.getSnapshot().status, "ready");
  await new Promise<void>((done) => setTimeout(done, 120));
  assert.equal(session.getSnapshot().closeStatus, "pending");
  finishing.resolve(receipt);
  assert.equal(await closing, true);
  assert.equal(session.getSnapshot().status, "closed");
  assert.deepEqual(
    requests.slice(1).map((r) => r.input),
    [
      { projectId: "p", sessionId: "s" },
      { projectId: "p", sessionId: "s" },
    ],
  );
  assert.equal(await session.requestClose(), true);
  session.close();
  assert.equal(
    requests.length,
    3,
    "confirmed cleanup must not send another stop",
  );
});

test("lost close response retains exact identity and exposes manual retry without reopening", async () => {
  const requests: unknown[] = [];
  let opens = 0;
  const session = new LiveScenePreview(
    "p",
    "r",
    "snapshot",
    async (method, input) => {
      if (!method.endsWith("close")) {
        opens++;
        return source;
      }
      requests.push(input);
      if (requests.length === 1) throw new Error("lost close response");
      return receipt;
    },
  );
  await session.refresh();
  assert.equal(await session.requestClose(), false);
  assert.equal(session.getSnapshot().closeStatus, "failed");
  assert.match(session.getSnapshot().closeError, /lost close response/);
  await session.refresh();
  assert.equal(await session.requestClose(), true);
  assert.deepEqual(requests, [
    { projectId: "p", sessionId: "s" },
    { projectId: "p", sessionId: "s" },
  ]);
  assert.equal(opens, 1);
  assert.equal(session.getSnapshot().closeError, "");
});

test("foreign or malformed close receipts never confirm closure", async () => {
  for (const invalid of [
    { ...receipt, projectId: "other" },
    { ...receipt, sessionId: "other" },
    { closed: true },
    { ...receipt, closed: "true" },
  ]) {
    const session = new LiveScenePreview(
      "p",
      "r",
      "snapshot",
      async (method) => (method.endsWith("close") ? invalid : source),
    );
    await session.refresh();
    assert.equal(await session.requestClose(), false);
    assert.equal(session.getSnapshot().closeStatus, "failed");
    assert.equal(session.getSnapshot().status, "ready");
    assert.notEqual(session.getSnapshot().closeError, "");
    session.close();
  }
});

test("close waits for an in-flight open and does not display its late frame", async () => {
  const opening = deferred<unknown>();
  const requests: { method: string; input: unknown }[] = [];
  const session = new LiveScenePreview(
    "p",
    "r",
    "snapshot",
    async (method, input) => {
      requests.push({ method, input });
      return method.endsWith("close") ? receipt : opening.promise;
    },
  );
  const reading = session.refresh();
  const closing = session.requestClose();
  await settle();
  assert.equal(requests.length, 1);
  assert.equal(session.getSnapshot().status, "idle");
  opening.resolve(source);
  await reading;
  assert.equal(await closing, true);
  assert.equal(session.getSnapshot().status, "closed");
  assert.deepEqual(requests[1], {
    method: "validation.preview.close",
    input: { projectId: "p", sessionId: "s" },
  });
});

test("closing recovers an uncertain open with the same request but never opens an unused viewer", async () => {
  const opens: unknown[] = [];
  const stops: unknown[] = [];
  const call = async (method: string, input: unknown) => {
    if (method.endsWith("close")) {
      stops.push(input);
      return receipt;
    }
    opens.push(input);
    if (opens.length === 1) throw new Error("lost open response");
    return source;
  };
  const unused = new LiveScenePreview("p", "r", "snapshot", call);
  assert.equal(await unused.requestClose(), true);
  assert.equal(opens.length, 0);
  const session = new LiveScenePreview("p", "r", "snapshot", call);
  await session.refresh();
  assert.equal(await session.requestClose(), true);
  assert.deepEqual(opens[0], opens[1]);
  assert.deepEqual(stops, [{ projectId: "p", sessionId: "s" }]);
});

test("timed-out close cannot be confirmed by a late reply; retry keeps the same target", async (t) => {
  t.mock.timers.enable({ apis: ["setTimeout", "Date"], now: 1000 });
  const late = deferred<unknown>();
  const stops: unknown[] = [];
  const session = new LiveScenePreview(
    "p",
    "r",
    "snapshot",
    async (method, input) => {
      if (!method.endsWith("close")) return source;
      stops.push(input);
      return stops.length === 1 ? late.promise : receipt;
    },
  );
  await session.refresh();
  const closing = session.requestClose();
  await settle();
  t.mock.timers.tick(10_000);
  assert.equal(await closing, false);
  assert.match(session.getSnapshot().closeError, /PREVIEW_CLOSE_TIMEOUT/);
  late.resolve(receipt);
  await settle();
  assert.equal(session.getSnapshot().closeStatus, "failed");
  assert.equal(await session.requestClose(), true);
  assert.deepEqual(stops[0], stops[1]);
});

test("an open that outlives the close deadline keeps its identity without resuming the viewer", async (t) => {
  t.mock.timers.enable({ apis: ["setTimeout", "Date"], now: 1000 });
  const late = deferred<unknown>();
  let opens = 0;
  let stops = 0;
  const session = new LiveScenePreview("p", "r", "snapshot", async (method) => {
    if (method.endsWith("close")) {
      stops++;
      return receipt;
    }
    opens++;
    return late.promise;
  });
  const reading = session.refresh();
  const closing = session.requestClose();
  t.mock.timers.tick(10_000);
  assert.equal(await closing, false);
  late.resolve(source);
  await reading;
  assert.equal(session.getSnapshot().status, "idle");
  assert.equal(session.getSnapshot().closeStatus, "failed");
  assert.equal(stops, 0);
  assert.equal(await session.requestClose(), true);
  assert.equal(opens, 1);
  assert.equal(stops, 1);
});

test("pending worker receipts have a bounded wait instead of claiming success", async (t) => {
  t.mock.timers.enable({ apis: ["setTimeout", "Date"], now: 1000 });
  const session = new LiveScenePreview("p", "r", "snapshot", async (method) =>
    method.endsWith("close") ? { ...receipt, closed: false } : source,
  );
  await session.refresh();
  const closing = session.requestClose();
  await settle();
  t.mock.timers.tick(10_000);
  assert.equal(await closing, false);
  assert.equal(session.getSnapshot().closeStatus, "failed");
  assert.match(session.getSnapshot().closeError, /PREVIEW_CLOSE_TIMEOUT/);
});

test("a pending read finishes before stop and cannot revive the closing view", async () => {
  const late = deferred<unknown>();
  const methods: string[] = [];
  const session = new LiveScenePreview("p", "r", "snapshot", async (method) => {
    methods.push(method);
    if (method.endsWith("open")) return source;
    if (method.endsWith("read")) return late.promise;
    return receipt;
  });
  await session.refresh();
  const reading = session.refresh();
  const closing = session.requestClose();
  await settle();
  assert.deepEqual(methods, [
    "validation.preview.open",
    "validation.preview.read",
  ]);
  late.resolve({ ...source, error: "late read error" });
  await reading;
  assert.equal(await closing, true);
  assert.equal(session.getSnapshot().error, "");
  assert.equal(session.getSnapshot().status, "closed");
  await session.refresh();
  assert.equal(methods.length, 3);
});
