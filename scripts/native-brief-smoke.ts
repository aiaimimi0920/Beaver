import fs from "node:fs/promises";
import path from "node:path";
import { spawnSync } from "node:child_process";
import {
  defaultBlueprint,
  legacyGenreChoices,
} from "../src/shared/project-blueprint";
import {
  defaultGameBrief,
  genres,
  themes,
  styles,
  scopes,
} from "../src/shared/game-design";
import { genreChoices, themeChoices } from "../src/shared/blueprint-catalog";
import { formatTaskBrief } from "../src/shared/task-brief";
import type { Task } from "../src/shared/types";

type Context = Pick<Task, "design" | "projectContext">;
const cases: { task: Context; expected: string }[] = [];
function add(task: Context) {
  cases.push({ task, expected: formatTaskBrief(task) });
}
add({});
for (const group of [genres, themes, styles, scopes]) {
  for (const item of group) {
    const design = defaultGameBrief();
    if (group === genres) design.genres = [item.id];
    else if (group === themes) design.theme = item.id;
    else if (group === styles) design.style = item.id;
    else design.scope = item.id;
    add({ design });
  }
}
for (const genre of [...genreChoices, ...legacyGenreChoices]) {
  const blueprint = defaultBlueprint();
  blueprint.genres = [genre.id];
  add({ projectContext: { name: "夜航调饮室", revision: 7, blueprint } });
}
for (const theme of themeChoices) {
  const blueprint = defaultBlueprint();
  blueprint.theme = { mode: "preset", value: theme.id };
  add({ projectContext: { name: "新游戏", revision: 2, blueprint } });
}
for (const topology of ["dedicated", "host-relay"] as const) {
  const blueprint = defaultBlueprint();
  blueprint.genres = ["visual-novel", "action"];
  blueprint.theme = { mode: "custom", value: '中文与引号"\n多行' };
  blueprint.online = {
    enabled: true,
    topology,
    playersPerSession: 16,
    peakCcu: 3000,
    serverCount: 8,
    region: "亚洲 / 欧洲",
  };
  blueprint.plannedFeatures = ["dialogue", "save-slot"];
  blueprint.audience = "mature";
  blueprint.size = "aaa";
  blueprint.campaigns.press = {
    channels: ["媒体一", "媒体二"],
    notes: "不自动发布",
  };
  blueprint.campaigns.demo = { channels: [], notes: "试玩版计划" };
  blueprint.campaigns.advertising = { channels: ["渠道"], notes: "" };
  add({ projectContext: { name: "冻结上下文", revision: 19, blueprint } });
}
async function main() {
  const directory = path.resolve(
    "output/validation",
    `native-brief-${Date.now()}`,
  );
  await fs.mkdir(directory, { recursive: true });
  const fixture = path.join(directory, "fixture.json");
  const catalog: unknown = JSON.parse(
    await fs.readFile("dist-native/blueprint-catalog.json", "utf8"),
  );
  await fs.writeFile(fixture, JSON.stringify({ catalog, cases }));
  const run = spawnSync(
    "rtk",
    [
      "cargo",
      "run",
      "--quiet",
      "--locked",
      "-p",
      "beaver-core",
      "--example",
      "brief_contract",
      "--",
      fixture,
    ],
    { encoding: "utf8", timeout: 180000 },
  );
  if (run.error || run.status !== 0)
    throw new Error(run.error?.message ?? run.stderr + run.stdout);
  const result: unknown = JSON.parse(run.stdout.trim());
  await fs.writeFile(
    path.join(directory, "proof.json"),
    JSON.stringify(
      { result, reference: "src/shared/task-brief.ts", cases: cases.length },
      null,
      2,
    ),
  );
  console.log(directory, run.stdout.trim());
}
void main();
