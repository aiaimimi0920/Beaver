import assert from "node:assert/strict";
import test from "node:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { parseObjectTaskSnapshot } from "../src/shared/object-task-snapshot";
import {
  commitObjectTaskSchema,
  objectTaskDraftSchema,
  objectTaskPlanSchema,
  saveObjectTaskDraftSchema,
  type ObjectTaskSnapshot,
} from "../src/shared/object-tasks";
import { ObjectTaskList } from "../src/ui/object-tasks/ObjectTaskList";
import { objectTaskSnapshot } from "./fixtures/object-tasks";

function firstRecord<T>(records: readonly T[]): T {
  const record = records[0];
  assert.ok(record, "Fixture must include a record");
  return record;
}

test("object task snapshot preserves real hierarchy and independent medium identities", () => {
  const snapshot = objectTaskSnapshot();
  assert.deepEqual(parseObjectTaskSnapshot(snapshot, "p"), snapshot);
  assert.equal(
    snapshot.tasks.find((task) => task.id === "independent")?.parentTaskId,
    null,
  );
  assert.deepEqual(
    parseObjectTaskSnapshot(
      {
        planRevision: 0,
        tasks: [],
        runs: [],
        coarseDispatchControls: [],
        dispatchControls: [],
      },
      "p",
    ),
    {
      planRevision: 0,
      tasks: [],
      runs: [],
      assumptions: [],
      coarseDispatchControls: [],
      dispatchControls: [],
    },
  );
});

test("plan assumptions preserve their basis and source with strict validation", () => {
  const assumption = {
    id: "assumption-camera",
    statement: "The camera remains fixed during the first scene.",
    basis: "The current design brief specifies a fixed camera.",
    source: "codex",
    sourceDetail: "Derived from the scene description",
  } as const;
  assert.deepEqual(
    objectTaskPlanSchema.parse({ assumptions: [assumption] }).assumptions,
    [assumption],
  );
  assert.deepEqual(
    objectTaskPlanSchema.parse({
      assumptions: [
        { id: "assumption-default", statement: "Default", basis: "User input" },
      ],
    }).assumptions,
    [
      {
        id: "assumption-default",
        statement: "Default",
        basis: "User input",
        source: "user",
        sourceDetail: null,
      },
    ],
  );
  for (const invalid of [
    { ...assumption, statement: "  " },
    { ...assumption, basis: "" },
    { ...assumption, source: "system" },
    { ...assumption, undocumented: true },
  ]) {
    assert.throws(() => objectTaskPlanSchema.parse({ assumptions: [invalid] }));
  }
});

test("object task snapshot rejects foreign projects and inconsistent task/run relationships", () => {
  const mutations: ((value: ObjectTaskSnapshot) => void)[] = [
    (value) => {
      firstRecord(value.tasks).projectId = "other";
    },
    (value) => {
      firstRecord(value.runs).projectId = "other";
    },
    (value) => {
      value.tasks.push(firstRecord(value.tasks));
    },
    (value) => {
      value.runs.push(firstRecord(value.runs));
    },
    (value) => {
      firstRecord(value.tasks).objectId = "another-object";
    },
    (value) => {
      firstRecord(value.tasks).parentTaskId = "coarse";
    },
    (value) => {
      firstRecord(value.tasks).runId = "run-independent";
    },
    (value) => {
      firstRecord(value.tasks).stageId = "wrong-stage";
    },
    (value) => {
      firstRecord(value.tasks).dependsOn = ["missing"];
    },
    (value) => {
      value.runs.pop();
    },
    (value) => {
      firstRecord(value.runs).mediumTaskId = "fine";
    },
  ];
  for (const mutate of mutations) {
    const value = objectTaskSnapshot();
    mutate(value);
    assert.throws(() => parseObjectTaskSnapshot(value, "p"));
  }
  const value = objectTaskSnapshot();
  assert.throws(() =>
    parseObjectTaskSnapshot(
      { ...value, tasks: [{ ...firstRecord(value.tasks), status: "running" }] },
      "p",
    ),
  );
});

test("draft and commit contracts accept persistence fields but reject execution fields", () => {
  const input = {
    projectId: "p",
    draftId: "draft",
    expectedRevision: 0,
    expectedPlanRevision: 0,
    plan: {},
  };
  const draft = saveObjectTaskDraftSchema.parse(input);
  assert.deepEqual(draft.plan, { objects: [], tasks: [], assumptions: [] });
  assert.deepEqual(
    objectTaskDraftSchema.parse({
      projectId: "p",
      id: "draft",
      revision: 1,
      planRevision: 0,
      plan: draft.plan,
    }).plan,
    draft.plan,
  );
  assert.throws(() =>
    saveObjectTaskDraftSchema.parse({ ...input, autoAccept: true }),
  );
  assert.throws(() =>
    saveObjectTaskDraftSchema.parse({ ...input, expectedRevision: -1 }),
  );
  assert.throws(() =>
    saveObjectTaskDraftSchema.parse({
      ...input,
      plan: { tasks: [firstRecord(objectTaskSnapshot().tasks)] },
    }),
  );
  const commit = {
    projectId: "p",
    draftId: "draft",
    requestId: "commit",
    expectedDraftRevision: 1,
    expectedPlanRevision: 0,
  };
  assert.deepEqual(commitObjectTaskSchema.parse(commit), commit);
  assert.throws(() =>
    commitObjectTaskSchema.parse({ ...commit, execute: true }),
  );
});

test("object task list renders all persisted levels and run/dependency IDs without execution controls", () => {
  const snapshot = parseObjectTaskSnapshot(objectTaskSnapshot(), "p");
  const html = renderToStaticMarkup(
    createElement(ObjectTaskList, { snapshot }),
  );
  for (const text of [
    "粗修",
    "中修",
    "精修",
    "Movement",
    "Independent iteration",
    "run-hero",
    "run-independent",
    "责任父任务",
    "依赖",
    "待执行时确定",
  ]) {
    assert.ok(html.includes(text), text);
  }
  assert.equal((html.match(/class="object-task-record"/g) ?? []).length, 4);
  assert.ok(html.indexOf("Game") < html.indexOf("Hero"));
  assert.ok(html.indexOf("Hero") < html.indexOf("Movement"));
  assert.ok(!/<button|progressbar|\d+%/.test(html));
});
