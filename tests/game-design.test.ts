import assert from "node:assert/strict";
import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import {
  defaultGameBrief,
  gameBriefSchema,
  genres,
  themes,
  recommendedFeatures,
  formatGameBrief,
} from "../src/shared/game-design";
import { Projects } from "../src/core/projects";
import { Store } from "../src/core/store";

test("game direction validates combinations and formats actual task context", () => {
  const brief = { ...defaultGameBrief(), genres: ["narrative", "management"] };
  assert.deepEqual(gameBriefSchema.parse(brief), brief);
  assert.equal(genres.length, 12);
  assert.equal(themes.length, 12);
  assert.throws(() =>
    gameBriefSchema.parse({ ...brief, genres: ["rpg", "rpg"] }),
  );
  assert.throws(() =>
    gameBriefSchema.parse({ ...brief, theme: "not-in-catalog" }),
  );
  assert.throws(() => gameBriefSchema.parse({ ...brief, genres: [] }));
  const recommended = recommendedFeatures(brief);
  assert.ok(
    recommended.includes("dialogue") && recommended.includes("economy"),
  );
  assert.equal(new Set(recommended).size, recommended.length);
  assert.match(formatGameBrief(brief), /叙事.*经营管理/);
  assert.match(formatGameBrief(brief), /不是已实现功能/);
  assert.equal(formatGameBrief(undefined), "");
});

test("project direction survives exportable metadata and existing project adoption", async () => {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), "beaver-design-"));
  const first = new Store(path.join(root, "db-one"));
  const second = new Store(path.join(root, "db-two"));
  try {
    const projects = new Projects(first, path.resolve("resources"));
    const brief = {
      ...defaultGameBrief(),
      genres: ["survival", "management"],
      theme: "wasteland",
    };
    const created = await projects.create(root, "Original", "blank", brief);
    assert.deepEqual(created.design, brief);
    const metadata = JSON.parse(
      await fs.readFile(path.join(created.path, "beaver.project.json"), "utf8"),
    );
    assert.deepEqual(metadata.design, brief);
    const reimported = await new Projects(
      second,
      path.resolve("resources"),
    ).import(created.path);
    assert.deepEqual(reimported.design, brief);
    const updated = { ...brief, theme: "nature" };
    await fs.writeFile(
      path.join(created.path, "beaver.project.json"),
      JSON.stringify({ schemaVersion: 1, design: updated }),
    );
    const refreshed = await projects.import(created.path);
    assert.equal(refreshed.id, created.id);
    assert.deepEqual(refreshed.design, updated);
    const legacy = await projects.create(root, "Legacy", "blank");
    assert.equal(legacy.design, undefined);
    await fs.writeFile(
      path.join(legacy.path, "beaver.project.json"),
      JSON.stringify({ schemaVersion: 99, design: brief }),
    );
    await assert.rejects(
      new Projects(second, path.resolve("resources")).import(legacy.path),
      /beaver.project.json/,
    );
    await assert.rejects(projects.import(legacy.path), /beaver.project.json/);
    await assert.rejects(
      projects.create(root, "Invalid", "blank", { ...brief, genres: [] }),
    );
    await assert.rejects(fs.access(path.join(root, "Invalid")));
    const catalog = await projects.features();
    assert.equal(catalog.length, 10);
    assert.equal(new Set(catalog.map((f) => f.id)).size, catalog.length);
    for (const genre of genres)
      for (const id of recommendedFeatures({ ...brief, genres: [genre.id] }))
        assert.ok(
          catalog.some((f) => f.id === id),
          `Recommendation exists: ${id}`,
        );
    for (const feature of catalog) {
      const entries = await fs.readdir(
        path.join("resources/features", feature.id),
      );
      assert.ok(
        entries.some(
          (entry) =>
            entry !== "feature.json" && /\.(gd|tscn|json)$/.test(entry),
        ),
        `Feature has actual source or data: ${feature.id}`,
      );
      assert.ok(
        (
          await fs.stat(
            path.join("resources/features", feature.id, "README.md"),
          )
        ).isFile(),
      );
    }
  } finally {
    first.close();
    second.close();
    await fs.rm(root, { recursive: true, force: true });
  }
});
