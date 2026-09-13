import assert from "node:assert/strict";
import fs from "node:fs/promises";
import path from "node:path";
import os from "node:os";
import test from "node:test";
import { enginePackages, enginePackageId } from "../src/shared/engine-packages";
import {
  planningPackages,
  filterPackages,
} from "../src/shared/function-packages";
import {
  blueprintSchema,
  defaultBlueprint,
} from "../src/shared/project-blueprint";
import { Projects } from "../src/core/projects";
import { Store } from "../src/core/store";
import { Tasks } from "../src/core/tasks";
import { Files, ProjectLocks } from "../src/core/files";
import { Preferences } from "../src/core/settings";

test("engine catalog covers all 68 source capability keys without headings or executable manifests", () => {
  const expected =
    `OPEN_WORLD SANDBOX GESTURE_CONTROL MOTION_CONTROL FACIAL_RECOGNITION SIMPLE_ANTICHEAT AI_POWERED_ANTICHEAT CROSS_PLAY AUDIO_RECOGNITION LOW_LATENCY_SERVER HIGH_DEF_DISPLAY ULTRA_HIGH_DEF_DISPLAY AR_32BITS AI_POWERED_DIFFICULTY REALISTIC_CHARACTER ADS IAP 2D_1BITCOLOR 2D_8BITCOLOR 2D_16BITCOLOR 2D_24BITCOLOR 2D_32BITCOLOR 3D_1BITCOLOR 3D_8BITCOLOR 3D_16BITCOLOR 3D_24BITCOLOR 3D_32BITCOLOR VR_32BITCOLOR VOXEL_32BITCOLOR BASIC_DIALOG INTERACTIVE_DIALOG AI_GENERATED_DIALOG JOYSTICK GAMEPAD MOUSE PREBUILT_CHARACTER CUSTOMIZABLE_CHARACTER DIFFICULTY DYNAMIC_DIFFICULTY MAP_EDITOR MOD_SUPPORT MONO_SOUND MIDI_SOUND STEREO_SOUND 51_SOUND BASIC_MUSIC ORCHESTRAL_MUSIC INTERACTIVE_MUSIC PASSWORD_SAVE BASIC_SAVE CLOUD_SAVE BASIC_PHYSICS ADVANCED_PHYSICS REALISTIC_PHYSICS DEDICATED_SERVER SOCIAL_NETWORK_INTEGRATION SIMPLE_ANIMATION DYNAMIC_ANIMATION AI_BASED_ANIMATION BASIC_TUTORIEL INTERACTIVE_TUTORIEL LOADING_SCREEN BACKGROUND_LOADING SIMPLE_CINEMATIC ADVANCED_CINEMATIC REALISTIC_CINEMATIC LEADERBOARD CLOUD_LEADERBOARD`.split(
      " ",
    );
  assert.equal(enginePackages.length, 68);
  assert.equal(new Set(enginePackages.map((p) => p.id)).size, 68);
  assert.deepEqual(
    enginePackages.map((p) => p.sourceKey).sort(),
    expected.map((k) => `LABEL_GAME_ENGINE_${k}`).sort(),
  );
  for (const p of enginePackages) {
    assert.equal(p.kind, "planning");
    assert.equal(enginePackageId(p.id), true);
    assert.match(p.id, /^[a-z0-9-]+$/);
    assert.ok(p.name && p.category && p.description.length > 15);
    assert.ok(
      !("version" in p),
      "Do not manufacture an adopted implementation version",
    );
  }
});

test("package filters distinguish planning, source, meanings and selected items", () => {
  const packages = planningPackages([
    {
      id: "inventory",
      name: "物品与背包",
      description: "库存规划",
      version: "1.0.0",
    },
  ]);
  assert.equal(filterPackages(packages, "", "", "planning").length, 68);
  assert.equal(filterPackages(packages, "", "", "source").length, 1);
  assert.equal(
    filterPackages(packages, " label_game_engine_cloud_save ", "", "")[0]?.id,
    "engine-cloud-save",
  );
  assert.ok(
    filterPackages(packages, "", "商业化", "planning").every(
      (p) => p.id === "engine-ads" || p.id === "engine-iap",
    ),
  );
  assert.equal(filterPackages(packages, "", "", "", []).length, 0);
  assert.deepEqual(
    filterPackages(packages, "", "", "", ["engine-cloud-save"]).map(
      (p) => p.id,
    ),
    ["engine-cloud-save"],
  );
  assert.equal(filterPackages(packages, "不存在的包名", "", "").length, 0);
});

test("all 78 packages persist as planning while every engine package is rejected by the real task entry", async () => {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), "beaver-engine-plan-"));
  const store = new Store(path.join(root, "data"));
  const files = new Files(store.root);
  const prefs = new Preferences(store, {
    encrypt: (s) => s,
    decrypt: (s) => s,
  });
  const tasks = new Tasks(
    store,
    files,
    new ProjectLocks(),
    prefs,
    path.resolve("resources"),
    path.resolve("dist/mcp.cjs"),
    () => {},
  );
  try {
    await tasks.ready;
    const projects = new Projects(store, path.resolve("resources"));
    const sources = await projects.features();
    assert.equal(sources.length, 10);
    assert.ok(sources.every((p) => !enginePackageId(p.id)));
    const packages = planningPackages(sources);
    assert.equal(packages.length, 78);
    const plan = blueprintSchema.parse({
      ...defaultBlueprint(),
      plannedFeatures: packages.map((p) => p.id),
    });
    const project = await projects.create(
      root,
      "AllPackages",
      "blank",
      undefined,
      plan,
    );
    const before = await files.capture(project.path);
    for (const p of enginePackages)
      await assert.rejects(tasks.feature(project.id, p.id), /仅支持规划/);
    assert.equal(store.list("task").length, 0);
    assert.deepEqual(await files.capture(project.path), before);
    const saved = projects.saveBlueprint(
      project.id,
      { ...plan, plannedFeatures: ["engine-cloud-save", "inventory"] },
      1,
    );
    const reopened = new Store(store.root);
    try {
      assert.deepEqual(
        new Projects(reopened, path.resolve("resources")).get(project.id)
          .blueprint,
        saved.blueprint,
      );
    } finally {
      reopened.close();
    }
  } finally {
    await tasks.shutdown();
    store.close();
    await fs.rm(root, { recursive: true, force: true });
  }
});
