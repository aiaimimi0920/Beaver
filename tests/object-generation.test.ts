import assert from "node:assert/strict";
import test from "node:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { ObjectGenerateDialog } from "../src/ui/object-preview/ObjectGenerateDialog";
import { ObjectGenerationSession } from "../src/ui/object-preview/object-generation-session";
import {
  objectTaskDraftSchema,
  type ObjectTaskDraft,
} from "../src/shared/object-tasks";

function fixture() {
  const values = new Map<string, string>();
  const storage = {
    getItem: (key: string) => values.get(key) ?? null,
    setItem: (key: string, value: string) => {
      values.set(key, value);
    },
  };
  let draft: ObjectTaskDraft | null = null;
  let lose = "";
  let creations = 0;
  let committed = false;
  const calls: { method: string; input: unknown }[] = [];
  const api = async (method: string, input: unknown): Promise<unknown> => {
    calls.push({ method, input });
    if (method === "objectTask.snapshot")
      return {
        planRevision: 0,
        tasks: [],
        runs: [],
        assumptions: [],
        dispatchControls: [],
        coarseDispatchControls: [],
      };
    if (method === "objectTask.getDraft") return draft;
    if (method === "objectTask.saveDraft") {
      const request = input as { draftId: string; plan: unknown };
      assert.equal(draft, null);
      draft = objectTaskDraftSchema.parse({
        projectId: "project",
        id: request.draftId,
        revision: 1,
        planRevision: 0,
        plan: request.plan,
      });
      if (lose === "save") {
        lose = "";
        throw new Error("response lost");
      }
      return draft;
    }
    if (method === "objectTask.commit") {
      assert.ok(draft);
      if (!committed) {
        creations++;
        committed = true;
        draft = { ...draft, revision: 2, committedRequestId: draft.id };
      }
      if (lose === "commit") {
        lose = "";
        throw new Error("response lost");
      }
      return {
        projectId: "project",
        requestId: draft.id,
        draftId: draft.id,
        draftRevision: 1,
        previousPlanRevision: 0,
        planRevision: 1,
        objectIds: draft.plan.objects.map((object) => object.id),
        taskIds: draft.plan.tasks.map((task) => task.id),
        runs: [
          {
            id: "run-test",
            projectId: "project",
            objectId: `${draft.id}.object`,
            mediumTaskId: `${draft.id}.medium`,
            baselineVersionId: null,
            status: "planned",
            revision: 0,
          },
        ],
      };
    }
    throw new Error(method);
  };
  return {
    storage,
    api,
    calls,
    lose: (phase: string) => {
      lose = phase;
    },
    creations: () => creations,
    session: () =>
      new ObjectGenerationSession("project", api, storage, "generation-test"),
  };
}

test("close and reopen restores all fields without remote calls, isolated by target", () => {
  const f = fixture();
  f.session().edit({
    name: "角色",
    category: "场景",
    prompt: "制作角色",
    acceptance: "可加载",
  });
  const session = f.session();
  assert.deepEqual(session.getSnapshot().saved.fields, {
    name: "角色",
    category: "场景",
    prompt: "制作角色",
    acceptance: "可加载",
  });
  assert.equal(f.calls.length, 0);
  const other = new ObjectGenerationSession(
    "other",
    f.api,
    f.storage,
    "generation-other",
  );
  assert.equal(other.getSnapshot().saved.fields.prompt, "");
});

for (const phase of ["save", "commit"]) {
  test(`lost ${phase} response reopens with exact identity and completes once`, async () => {
    const f = fixture();
    const session = f.session();
    session.edit({ prompt: "制作场景", acceptance: "可以在 Godot 中打开" });
    f.lose(phase);
    await session.submit();
    assert.match(session.getSnapshot().error, /response lost/);
    const frozen = session.getSnapshot().saved.pending;
    assert.ok(frozen);
    assert.deepEqual(frozen.plan.tasks[0]?.baseline, { basePolicy: "empty" });
    const reopened = f.session();
    reopened.edit({ prompt: "不得修改冻结内容" });
    assert.equal(reopened.getSnapshot().saved.fields.prompt, "制作场景");
    await reopened.submit();
    assert.equal(reopened.getSnapshot().error, "");
    assert.equal(f.creations(), 1);
    assert.deepEqual(reopened.getSnapshot().saved.pending?.plan, frozen.plan);
    assert.equal(
      f.calls.filter((call) => call.method === "objectTask.saveDraft").length,
      1,
    );
    const commits = f.calls.filter(
      (call) => call.method === "objectTask.commit",
    );
    if (phase === "commit")
      assert.deepEqual(commits[0]?.input, commits[1]?.input);
    assert.equal(
      f.session().getSnapshot().saved.receipt?.objectIds[0],
      "generation-test.object",
    );
    const count = f.calls.length;
    await reopened.submit();
    assert.equal(f.calls.length, count);
    assert.equal(frozen.plan.tasks[1]?.stageId, "generation-test.stage");
    assert.equal(frozen.plan.tasks[1]?.parentTaskId, frozen.plan.tasks[0]?.id);
  });
}

test("storage failure prevents remote writes and preserves editable fields", async () => {
  const f = fixture();
  const session = new ObjectGenerationSession("project", f.api, {
    getItem: () => null,
    setItem: () => {
      throw new Error("disk full");
    },
  });
  session.edit({ prompt: "制作场景" });
  await session.submit();
  assert.match(session.getSnapshot().error, /disk full/);
  assert.equal(session.getSnapshot().saved.fields.prompt, "制作场景");
  assert.equal(f.calls.length, 0);
});

test("wrong receipt identity is rejected and keeps original request", async () => {
  const f = fixture();
  const session = new ObjectGenerationSession(
    "project",
    async (method, input) => {
      const result = await f.api(method, input);
      return method === "objectTask.commit"
        ? { ...(result as object), projectId: "wrong" }
        : result;
    },
    f.storage,
    "generation-test",
  );
  session.edit({ prompt: "制作场景" });
  await session.submit();
  assert.match(session.getSnapshot().error, /回执身份不匹配/);
  assert.equal(session.getSnapshot().saved.receipt, null);
  assert.ok(session.getSnapshot().saved.pending);
});

test("closing while snapshot is pending preserves frozen fields before any remote write", async () => {
  const f = fixture();
  let release: (() => void) | undefined;
  const gate = new Promise<void>((resolve) => {
    release = resolve;
  });
  const session = new ObjectGenerationSession(
    "project",
    async (method, input) => {
      if (method === "objectTask.snapshot") await gate;
      return f.api(method, input);
    },
    f.storage,
    "generation-test",
  );
  session.edit({ prompt: "original" });
  const submission = session.submit();
  const reopened = f.session();
  assert.ok(reopened.getSnapshot().saved.pending);
  reopened.edit({ prompt: "changed" });
  assert.equal(reopened.getSnapshot().saved.fields.prompt, "original");
  assert.equal(f.calls.length, 0);
  release?.();
  await submission;
  assert.ok(session.getSnapshot().saved.receipt);
});

test("dialog tells users confirmation creates planned tasks and retains drafts", () => {
  const html = renderToStaticMarkup(
    createElement(ObjectGenerateDialog, {
      projectId: "project",
      close: () => {},
      openObject: () => {},
      openManufacture: () => {},
    }),
  );
  assert.match(html, /确认创建对象和任务/);
  assert.match(html, /关闭并保留草稿/);
  assert.match(html, /不会立即调用模型或生成文件/);
});
