import assert from "node:assert/strict";
import test from "node:test";
import {
  objectTaskRevisionImpact,
  validateObjectTaskRevisionPlan,
} from "../src/shared/object-task-revision-plan";
import {
  objectTaskDefinitionSchema,
  parseObjectTaskRevisionHistory,
  parseObjectTaskRevisionReceipt,
  revisePlannedObjectTaskSchema,
} from "../src/shared/object-task-revisions";
import {
  revisionDefinition,
  revisionReceipt as receipt,
  revisionRequest,
} from "./fixtures/object-task-revisions";
import { objectTaskSnapshot } from "./fixtures/object-tasks";

test("revision contracts limit editable fields, UTF-8 bytes, IDs and safe revisions", () => {
  assert.deepEqual(
    revisePlannedObjectTaskSchema.parse(revisionRequest),
    revisionRequest,
  );
  assert.equal(
    objectTaskDefinitionSchema.safeParse({
      ...revisionDefinition,
      title: "界".repeat(100),
    }).success,
    true,
  );
  for (const definition of [
    { ...revisionDefinition, title: "界".repeat(101) },
    { ...revisionDefinition, title: "  " },
    { ...revisionDefinition, prompt: "\n\t" },
    { ...revisionDefinition, prompt: "界".repeat(6_667) },
    { ...revisionDefinition, acceptance: "界".repeat(3_334) },
    { ...revisionDefinition, dependsOn: ["a", "a"] },
    { ...revisionDefinition, dependsOn: ["../outside"] },
    { ...revisionDefinition, dependsOn: undefined },
    {
      ...revisionDefinition,
      dependsOn: Array.from({ length: 501 }, (_, i) => `task-${i}`),
    },
    ...["objectId", "parentTaskId", "runId", "stageId", "identity"].map(
      (field) => ({ ...revisionDefinition, [field]: "changed" }),
    ),
  ])
    assert.equal(
      objectTaskDefinitionSchema.safeParse(definition).success,
      false,
    );
  for (const patch of [
    { reason: " " },
    { reason: "界".repeat(667) },
    { expectedTaskRevision: Number.MAX_SAFE_INTEGER },
    { expectedPlanRevision: Number.MAX_SAFE_INTEGER + 1 },
    { expectedTaskRevision: -1 },
    { requestId: "bad/id" },
    { adoptedBy: "codex" },
  ])
    assert.equal(
      revisePlannedObjectTaskSchema.safeParse({ ...revisionRequest, ...patch })
        .success,
      false,
    );
});

test("revision impact follows descendants and reverse dependencies across cancelled nodes", () => {
  const snapshot = objectTaskSnapshot();
  assert.deepEqual(objectTaskRevisionImpact(snapshot, "medium"), [
    "fine",
    "medium",
  ]);
  const fine = snapshot.tasks.find((task) => task.id === "fine")!;
  fine.status = "cancelled";
  fine.dependsOn = [];
  snapshot.tasks.find((task) => task.id === "independent")!.dependsOn = [
    "fine",
  ];
  snapshot.tasks.push({
    ...snapshot.tasks.find((task) => task.id === "coarse")!,
    id: "downstream",
    dependsOn: ["independent"],
  });
  assert.deepEqual(objectTaskRevisionImpact(snapshot, "medium"), [
    "downstream",
    "independent",
    "medium",
  ]);
});

test("revision planning rejects unavailable dependencies and cycles without conflating responsibility edges", () => {
  const snapshot = objectTaskSnapshot();
  assert.doesNotThrow(() =>
    validateObjectTaskRevisionPlan(snapshot, "medium", revisionDefinition),
  );
  for (const dependsOn of [["medium"], ["missing"]]) {
    assert.throws(() =>
      validateObjectTaskRevisionPlan(snapshot, "medium", {
        ...revisionDefinition,
        dependsOn,
      }),
    );
  }
  assert.throws(
    () =>
      validateObjectTaskRevisionPlan(snapshot, "independent", {
        ...revisionDefinition,
        dependsOn: ["fine"],
      }),
    /环路/,
  );
  snapshot.tasks.find((task) => task.id === "independent")!.status =
    "cancelled";
  assert.throws(
    () =>
      validateObjectTaskRevisionPlan(snapshot, "medium", revisionDefinition),
    /已撤销/,
  );
});

test("revision receipts must match the full reviewed request, original definition and exact impact", () => {
  const snapshot = objectTaskSnapshot();
  assert.deepEqual(
    parseObjectTaskRevisionReceipt(receipt, revisionRequest, snapshot),
    receipt,
  );
  const mismatches = [
    { projectId: "other" },
    { taskId: "fine" },
    { requestId: "other" },
    { previousTaskRevision: 1, taskRevision: 2 },
    { previousPlanRevision: 2, planRevision: 3 },
    { taskRevision: 2 },
    { before: { ...receipt.before, prompt: "Different original" } },
    { after: { ...receipt.after, dependsOn: [] } },
    { reason: "Different reason" },
    { adoptedBy: "codex" },
    { createdAt: "not-a-date" },
    { affectedTaskIds: ["medium"] },
    { affectedTaskIds: ["medium", "fine"] },
    { affectedTaskIds: ["fine", "independent", "medium"] },
    { affectedTaskIds: ["fine", "medium", "medium"] },
    { unexpected: true },
  ];
  for (const patch of mismatches) {
    assert.throws(() =>
      parseObjectTaskRevisionReceipt(
        { ...receipt, ...patch },
        revisionRequest,
        snapshot,
      ),
    );
  }
});

test("history preserves known immutable records and validates identity, sequence and definition continuity", () => {
  const next = {
    ...receipt,
    requestId: "revision-2",
    previousTaskRevision: 1,
    taskRevision: 2,
    previousPlanRevision: 3,
    planRevision: 4,
    before: receipt.after,
    after: { ...receipt.after, title: "Second definition" },
  };
  const history = [receipt, next];
  assert.deepEqual(
    parseObjectTaskRevisionHistory(history, "p", "medium", [receipt]),
    history,
  );
  for (const input of [
    [receipt, receipt],
    [next, receipt],
    [{ ...receipt, projectId: "other" }],
    [{ ...receipt, taskId: "other" }],
    [receipt, { ...next, before: receipt.before }],
    [{ ...receipt, taskRevision: 0 }],
  ])
    assert.throws(() => parseObjectTaskRevisionHistory(input, "p", "medium"));
  for (const input of [
    [],
    [next],
    [{ ...receipt, reason: "Rewritten history" }],
  ]) {
    assert.throws(
      () => parseObjectTaskRevisionHistory(input, "p", "medium", [receipt]),
      /缺少或改写/,
    );
  }
});
