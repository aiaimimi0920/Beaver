import assert from "node:assert/strict";
import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import {
  audiences,
  gameSizes,
  genreChoices,
  phases,
  themeChoices,
} from "../src/shared/blueprint-catalog";
import {
  blueprintSchema,
  defaultBlueprint,
} from "../src/shared/project-blueprint";
import { Projects } from "../src/core/projects";
import { Files } from "../src/core/files";
import { Store } from "../src/core/store";

test("blueprint taxonomy has 64 genres, 49 themes, 3 audiences, 4 sizes and all 15 priorities", () => {
  assert.equal(genreChoices.length, 64);
  assert.equal(themeChoices.length, 49);
  assert.equal(new Set(genreChoices.map((g) => g.sourceKey)).size, 64);
  assert.equal(new Set(themeChoices.map((g) => g.sourceKey)).size, 49);
  assert.ok(!genreChoices.some((g) => g.sourceKey === "LABEL_TYPE_BUILDING"));
  assert.ok(!themeChoices.some((g) => g.sourceKey === "LABEL_THEME_LIST"));
  assert.equal(audiences.length, 3);
  assert.equal(gameSizes.length, 4);
  assert.deepEqual(
    phases.map((p) => p.metrics.length),
    [5, 5, 5],
  );
  assert.deepEqual(
    Object.keys(defaultBlueprint().priorities).sort(),
    phases.flatMap((p) => p.metrics.map((m) => m.id)).sort(),
  );
});

test("blueprint validates combinations, custom theme and independent priorities", () => {
  const plan = defaultBlueprint();
  assert.deepEqual(blueprintSchema.parse(plan), plan);
  assert.throws(() =>
    blueprintSchema.parse({ ...plan, genres: ["rpg", "rpg"] }),
  );
  assert.throws(() =>
    blueprintSchema.parse({ ...plan, theme: { mode: "custom", value: "   " } }),
  );
  assert.throws(() =>
    blueprintSchema.parse({
      ...plan,
      theme: { mode: "custom", value: "x".repeat(121) },
    }),
  );
  const custom = blueprintSchema.parse({
    ...plan,
    theme: { mode: "custom", value: "  原创雨夜车站  " },
    priorities: { ...plan.priorities, story: 5, graphics: 5 },
  });
  assert.equal(custom.theme.value, "原创雨夜车站");
  assert.equal(custom.priorities.story, 5);
  assert.equal(custom.priorities.graphics, 5);
  assert.throws(() =>
    blueprintSchema.parse({
      ...plan,
      priorities: { ...plan.priorities, story: 6 },
    }),
  );
  assert.throws(
    () => blueprintSchema.parse({ ...plan, genres: ["eroge"] }),
    /成年人/,
  );
  assert.doesNotThrow(() =>
    blueprintSchema.parse({ ...plan, genres: ["eroge"], audience: "mature" }),
  );
});

test("online blueprint distinguishes per-session capacity from CCU and never accepts impossible bounds", () => {
  const plan = defaultBlueprint();
  assert.throws(
    () => blueprintSchema.parse({ ...plan, genres: ["mmorpg"] }),
    /在线多人/,
  );
  const online = {
    ...plan,
    genres: ["mmorpg"],
    online: {
      ...plan.online,
      enabled: true,
      playersPerSession: 16,
      peakCcu: 1000,
      serverCount: 4,
    },
  };
  assert.doesNotThrow(() => blueprintSchema.parse(online));
  assert.throws(
    () =>
      blueprintSchema.parse({
        ...online,
        online: { ...online.online, peakCcu: 8 },
      }),
    /单局人数/,
  );
  assert.throws(() =>
    blueprintSchema.parse({
      ...online,
      online: { ...online.online, serverCount: 0 },
    }),
  );
  assert.throws(() =>
    blueprintSchema.parse({
      ...online,
      online: { ...online.online, playersPerSession: NaN },
    }),
  );
});

test("legacy design seeds a separate planning draft without changing original intent", () => {
  const legacy = {
    genres: ["card"],
    theme: "campus",
    scope: "short-game",
    style: "pixel",
  };
  const before = structuredClone(legacy);
  const draft = defaultBlueprint(legacy);
  assert.deepEqual(draft.genres, ["card"]);
  assert.deepEqual(draft.theme, { mode: "custom", value: "校园青春" });
  assert.deepEqual(legacy, before);
  assert.doesNotThrow(() => blueprintSchema.parse(draft));
});

test("import cannot overwrite a blueprint saved while reading legacy design", async (t) => {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), "beaver-import-plan-"));
  const store = new Store(path.join(root, "data"));
  try {
    const projects = new Projects(store, path.resolve("resources"));
    const plan = defaultBlueprint();
    const project = await projects.create(
      root,
      "Import",
      "blank",
      undefined,
      plan,
    );
    const originalRead = fs.readFile.bind(fs);
    const replacement = (...args: Parameters<typeof fs.readFile>) => {
      if (String(args[0]) === path.join(project.path, "beaver.project.json")) {
        projects.saveBlueprint(
          project.id,
          { ...plan, priorities: { ...plan.priorities, story: 5 } },
          1,
        );
      }
      return originalRead(...args);
    };
    t.mock.method(fs, "readFile", replacement);
    const imported = await projects.import(project.path);
    assert.equal(imported.blueprint?.priorities.story, 5);
    assert.equal(imported.blueprintRevision, 2);
  } finally {
    t.mock.restoreAll();
    store.close();
    await fs.rm(root, { recursive: true, force: true });
  }
});

test("planning persists with revision checks but cannot create tasks or change game files", async () => {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), "beaver-plan-"));
  const store = new Store(path.join(root, "data"));
  try {
    const projects = new Projects(store, path.resolve("resources"));
    const plan = defaultBlueprint();
    plan.plannedFeatures = ["inventory", "dialogue"];
    const p = await projects.create(root, "Planning", "blank", undefined, plan);
    assert.equal(p.blueprintRevision, 1);
    assert.equal(p.design, undefined);
    const files = new Files(store.root);
    const before = await files.capture(p.path);
    const saved = projects.saveBlueprint(
      p.id,
      {
        ...plan,
        priorities: { ...plan.priorities, story: 5 },
        campaigns: {
          ...plan.campaigns,
          advertising: {
            notes: "预算仅待讨论，不投放",
            channels: ["社交媒体"],
          },
        },
      },
      1,
    );
    assert.equal(saved.blueprintRevision, 2);
    assert.throws(() => projects.saveBlueprint(p.id, plan, 1), /已有更新/);
    assert.equal(projects.get(p.id).blueprint?.priorities.story, 5);
    assert.deepEqual(await files.capture(p.path), before);
    assert.equal(store.list("task").length, 0);
    const reopened = new Store(store.root);
    try {
      assert.deepEqual(
        new Projects(reopened, path.resolve("resources")).get(p.id).blueprint,
        saved.blueprint,
      );
    } finally {
      reopened.close();
    }
    const imported = await projects.import(p.path);
    assert.deepEqual(imported.blueprint, saved.blueprint);
    await assert.rejects(
      projects.create(root, "Invalid", "blank", undefined, {
        ...plan,
        genres: [],
      }),
    );
    await assert.rejects(fs.access(path.join(root, "Invalid")));
  } finally {
    store.close();
    await fs.rm(root, { recursive: true, force: true });
  }
});
