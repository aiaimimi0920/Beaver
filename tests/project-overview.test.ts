import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { Store } from "../src/core/store";
import { Projects } from "../src/core/projects";
import { Files } from "../src/core/files";
import { defaultBlueprint } from "../src/shared/project-blueprint";
import {
  changedOverviewFields,
  overviewFromPlan,
  overviewSchema,
  projectOverview,
} from "../src/shared/project-overview";

test("overview validates names and reports only changed foundations", () => {
  const value = overviewFromPlan("夜航", defaultBlueprint());
  assert.equal(overviewSchema.parse({ ...value, name: " 夜航 " }).name, "夜航");
  for (const name of [" ", "a".repeat(81), "name\u0000"])
    assert.equal(overviewSchema.safeParse({ ...value, name }).success, false);
  assert.deepEqual(
    changedOverviewFields(value, { ...value, name: "新夜航", size: "big" }),
    ["游戏名称", "游戏规模"],
  );
});

test("overview atomically saves metadata, preserves plans and rejects stale or bypass edits", async () => {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), "beaver-overview-"));
  const store = new Store(path.join(root, "data"));
  try {
    const projects = new Projects(store, path.resolve("resources"));
    const plan = defaultBlueprint();
    plan.priorities.story = 5;
    plan.plannedFeatures = ["dialogue", "engine-cloud-save"];
    plan.campaigns.press.notes = "仅记录媒体计划";
    const project = await projects.create(
      root,
      "Original",
      "blank",
      undefined,
      plan,
    );
    const before = await new Files(store.root).capture(project.path);
    const value = projectOverview(project);
    assert.equal(
      projects.saveOverview(project.id, value, 1).blueprintRevision,
      1,
    );
    for (const patch of [
      { size: "big" },
      { audience: "mature" },
      { online: { ...plan.online, enabled: true } },
    ])
      assert.throws(
        () => projects.saveBlueprint(project.id, { ...plan, ...patch }, 1),
        /基础设定已锁定/,
      );
    const saved = projects.saveOverview(
      project.id,
      {
        ...value,
        name: " 新夜航：失物招领 ",
        size: "big",
        online: { ...value.online, enabled: true },
      },
      1,
    );
    assert.equal(saved.name, "新夜航：失物招领");
    assert.equal(saved.path, project.path);
    assert.equal(saved.blueprintRevision, 2);
    assert.deepEqual(saved.blueprint?.priorities, plan.priorities);
    assert.deepEqual(saved.blueprint?.campaigns, plan.campaigns);
    assert.deepEqual(saved.blueprint?.plannedFeatures, plan.plannedFeatures);
    assert.throws(
      () => projects.saveOverview(project.id, value, 1),
      /已有更新/,
    );
    assert.equal((await projects.import(project.path)).name, saved.name);
    const reopened = new Store(store.root);
    try {
      assert.deepEqual(
        new Projects(reopened, path.resolve("resources")).get(project.id),
        saved,
      );
    } finally {
      reopened.close();
    }
    assert.deepEqual(await new Files(store.root).capture(project.path), before);
    assert.equal(store.list("task").length, 0);
  } finally {
    store.close();
    await fs.rm(root, { recursive: true, force: true });
  }
});

test("imported projects can save an overview while retaining legacy direction and cross-field checks", async () => {
  const root = await fs.mkdtemp(
    path.join(os.tmpdir(), "beaver-overview-import-"),
  );
  const store = new Store(path.join(root, "data"));
  try {
    const projects = new Projects(store, path.resolve("resources"));
    const design = {
      genres: ["card"],
      theme: "campus",
      scope: "short-game",
      style: "pixel",
    };
    const project = await projects.create(root, "Legacy", "blank", design);
    const value = projectOverview(project);
    assert.throws(
      () =>
        projects.saveOverview(
          project.id,
          { ...value, genres: ["eroge"], audience: "all" },
          0,
        ),
      /成年人/,
    );
    assert.throws(
      () =>
        projects.saveOverview(
          project.id,
          {
            ...value,
            genres: ["mmorpg"],
            online: { ...value.online, enabled: false },
          },
          0,
        ),
      /在线/,
    );
    assert.equal(projects.get(project.id).blueprintRevision, undefined);
    const saved = projects.saveOverview(
      project.id,
      { ...value, size: "big" },
      0,
    );
    assert.equal(saved.blueprintRevision, 1);
    assert.deepEqual(saved.design, design);
    assert.deepEqual(saved.blueprint?.genres, ["card"]);
  } finally {
    store.close();
    await fs.rm(root, { recursive: true, force: true });
  }
});
