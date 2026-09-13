import test from "node:test";
import assert from "node:assert/strict";
import { defaultBlueprint } from "../src/shared/project-blueprint";
import { formatTaskBrief } from "../src/shared/task-brief";
import { defaultGameBrief } from "../src/shared/game-design";

test("frozen project brief includes audience, online design and all phase priorities without claiming implementation", () => {
  const blueprint = defaultBlueprint();
  blueprint.theme = { mode: "custom", value: "海底图书馆" };
  blueprint.audience = "young";
  blueprint.online.enabled = true;
  blueprint.online.playersPerSession = 16;
  blueprint.online.peakCcu = 2048;
  blueprint.priorities.story = 5;
  blueprint.plannedFeatures = ["engine-physics"];
  blueprint.campaigns.advertising.notes = "准备宣传封面";
  const text = formatTaskBrief({
    projectContext: { name: "潜航", revision: 7, blueprint },
    design: defaultGameBrief(),
  });
  for (const expected of [
    "潜航",
    "规划版本 7",
    "海底图书馆",
    "青少年",
    "每房间 16",
    "2048",
    "故事性 5",
    "过场动画",
    "engine-physics",
    "准备宣传封面",
    "不自动发布",
    "不代表已安装",
    "beaver_ask_user",
  ])
    assert.ok(text.includes(expected), expected);
  assert.ok(
    !text.includes("5–10 分钟"),
    "legacy scope must not override current blueprint",
  );
});

test("offline blueprint does not leak inactive server targets, legacy tasks retain their original brief", () => {
  const text = formatTaskBrief({
    projectContext: {
      name: "test",
      revision: 0,
      blueprint: defaultBlueprint(),
    },
  });
  assert.match(text, /在线多人：关闭/);
  assert.ok(!text.includes("每房间"));
  assert.match(formatTaskBrief({ design: defaultGameBrief() }), /赛博朋克/);
  assert.equal(formatTaskBrief({}), "");
});
