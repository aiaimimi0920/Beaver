import assert from "node:assert/strict";
import fs from "node:fs/promises";
import path from "node:path";
import { createHash } from "node:crypto";
import { verifyPackages } from "./packages-smoke.mjs";

async function tree(root, relative = "") {
  const result = {};
  for (const entry of await fs.readdir(path.join(root, relative), {
    withFileTypes: true,
  })) {
    if (entry.name === ".godot") continue;
    const name = path.join(relative, entry.name);
    if (entry.isDirectory()) Object.assign(result, await tree(root, name));
    else
      result[name] = createHash("sha256")
        .update(await fs.readFile(path.join(root, name)))
        .digest("hex");
  }
  return result;
}

export async function verifyBlueprint({
  page,
  app,
  output,
  project,
  api,
  waitState,
}) {
  const baseline = await tree(project.path);
  const initial = await api("state");
  const plan = () => page.locator(".blueprint-page");
  await page.getByRole("region", { name: "游戏总览", exact: true }).waitFor();
  const overview = () => page.getByRole("region", { name: "游戏总览" });
  const editSwitch = () => page.getByRole("switch", { name: "编辑游戏设定" });
  assert.equal(await editSwitch().isChecked(), false);
  assert.equal(await page.locator(".page-header,.overview-grid").count(), 0);
  assert.equal(await page.locator(".overview-reserved").textContent(), "");
  assert.equal(await page.locator(".overview-configure:disabled").count(), 6);
  const layout = await overview().evaluate((n) => {
    const name = n.querySelector(".overview-name").getBoundingClientRect();
    const card = n.querySelector(".overview-card").getBoundingClientRect();
    return {
      sameTop: Math.abs(name.top - card.top) < 1,
      separated: name.right < card.left,
      cardWidth: card.width,
    };
  });
  assert.ok(layout.sameTop && layout.separated && layout.cardWidth <= 361);
  assert.equal(
    await overview().locator("input:not([role=switch]),select").count(),
    0,
  );
  await page.screenshot({ path: path.join(output, "19-overview-locked.png") });
  await editSwitch().check();
  assert.equal(
    await page
      .getByRole("button", { name: "保存设定", exact: true })
      .isDisabled(),
    true,
  );
  await page.getByLabel("游戏名称", { exact: true }).fill("夜航：失物招领");
  await editSwitch().click();
  await page.getByRole("dialog", { name: "放弃设定草稿？" }).waitFor();
  await page.getByRole("button", { name: "继续编辑", exact: true }).click();
  assert.equal(await editSwitch().isChecked(), true);
  await page
    .locator(".rail-nav")
    .getByRole("button", { name: "任务", exact: true })
    .click();
  await page
    .locator(".rail-nav")
    .getByRole("button", { name: "总览", exact: true })
    .click();
  assert.equal(await editSwitch().isChecked(), false);
  await editSwitch().check();
  assert.equal(
    await page.getByLabel("游戏名称", { exact: true }).inputValue(),
    "夜航：失物招领",
  );
  assert.equal(
    initial.tasks.length,
    0,
    "Creating a project with planned features must not start AI",
  );
  assert.equal(project.blueprint.plannedFeatures.length, 8);
  const dialog = () => page.getByRole("dialog");
  const open = async (name) => {
    await page.getByRole("button", { name, exact: true }).click();
    await dialog().waitFor();
  };
  const apply = async () => {
    await dialog().getByRole("button", { name: "应用", exact: true }).click();
    await dialog().waitFor({ state: "hidden" });
  };
  await open("修改类别");
  assert.equal(await dialog().locator(".choice-card").count(), 64);
  await dialog().getByLabel("搜索卡牌").fill("不存在的类别xyz");
  assert.equal(await dialog().locator(".choice-card").count(), 0);
  await dialog().getByText("没有匹配的选项", { exact: true }).waitFor();
  await dialog().getByLabel("搜索卡牌").fill("");
  await dialog().getByLabel("卡牌排序").selectOption("name");
  await dialog()
    .getByRole("button", { name: "成人向视觉小说", exact: true })
    .click();
  await dialog().getByRole("alert").filter({ hasText: "成年人" }).waitFor();
  assert.equal(
    await dialog()
      .getByRole("button", { name: "应用", exact: true })
      .isDisabled(),
    true,
  );
  await page.keyboard.press("Escape");
  await dialog().waitFor({ state: "hidden" });
  assert.equal(
    await page
      .getByRole("button", { name: "修改类别", exact: true })
      .evaluate((n) => n === document.activeElement),
    true,
  );
  await open("修改类别");
  assert.equal(
    await dialog()
      .getByRole("button", { name: "视觉小说", exact: true })
      .getAttribute("aria-pressed"),
    "true",
  );
  await dialog()
    .locator(".genre-slots")
    .getByRole("button", { name: /副类别/ })
    .click();
  assert.equal(
    await dialog()
      .getByRole("button", { name: "视觉小说", exact: true })
      .isDisabled(),
    true,
  );
  await dialog().getByRole("button", { name: "动作", exact: true }).click();
  await page.screenshot({ path: path.join(output, "23-genre-cards.png") });
  await apply();
  await open("修改类别");
  await dialog()
    .locator(".genre-slots")
    .getByRole("button", { name: /副类别/ })
    .click();
  await dialog().getByRole("button", { name: "无副类别", exact: true }).click();
  await apply();
  await page
    .locator(".overview-card-copy")
    .getByText("主类别 · 无副类别", { exact: true })
    .waitFor();
  await open("修改类别");
  await dialog()
    .locator(".genre-slots")
    .getByRole("button", { name: /副类别/ })
    .click();
  await dialog().getByRole("button", { name: "经营管理", exact: true }).click();
  await apply();
  await open("修改主题");
  assert.equal(await dialog().locator(".choice-card").count(), 50);
  await app.evaluate(({ BrowserWindow }) =>
    BrowserWindow.getAllWindows()[0].setContentSize(960, 680),
  );
  assert.ok(await dialog().evaluate((n) => n.scrollWidth <= n.clientWidth + 1));
  await page.screenshot({
    path: path.join(output, "24-theme-cards-compact.png"),
  });
  await dialog()
    .getByRole("button", { name: "自定义主题", exact: true })
    .click();
  assert.equal(
    await dialog()
      .getByRole("button", { name: "应用", exact: true })
      .isDisabled(),
    true,
  );
  await dialog()
    .getByRole("textbox", { name: "自定义主题", exact: true })
    .fill("雨夜车站里的失物招领");
  assert.ok(
    await dialog()
      .getByRole("button", { name: "应用", exact: true })
      .evaluate((n) => {
        const r = n.getBoundingClientRect();
        return r.top >= 50 && r.bottom <= innerHeight;
      }),
  );
  await apply();
  await app.evaluate(({ BrowserWindow }) =>
    BrowserWindow.getAllWindows()[0].setContentSize(1120, 760),
  );
  for (const [entry, name, count] of [
    ["修改评级", "青少年", 3],
    ["修改游戏大小", "大型", 4],
    ["修改表现风格", "手绘 2D", 6],
  ]) {
    await open(entry);
    assert.equal(await dialog().locator(".choice-card").count(), count);
    await dialog().getByRole("button", { name, exact: true }).click();
    await apply();
  }
  assert.equal(
    (await api("state")).projects.find((p) => p.id === project.id)
      .blueprintRevision,
    1,
    "Applying cards is local-only, not a project save",
  );
  await open("修改在线多人设定");
  await page
    .getByRole("checkbox", { name: "在线多人游戏", exact: true })
    .check();
  await page.getByLabel("每局 / 房间人数", { exact: true }).fill("16");
  await page.getByLabel("峰值同时在线人数", { exact: true }).fill("8");
  await dialog().getByRole("alert").filter({ hasText: "单局人数" }).waitFor();
  await page.getByLabel("峰值同时在线人数", { exact: true }).fill("1200");
  await page.getByLabel("初期服务器实例数", { exact: true }).fill("4");
  await page.getByLabel("每局 / 房间人数", { exact: true }).fill("");
  await page
    .getByRole("checkbox", { name: "在线多人游戏", exact: true })
    .uncheck();
  await dialog().getByRole("alert").filter({ hasText: "请填写每局" }).waitFor();
  assert.equal(
    await page.getByLabel("每局 / 房间人数", { exact: true }).isVisible(),
    true,
  );
  assert.equal(
    await dialog()
      .getByRole("button", { name: "应用", exact: true })
      .isDisabled(),
    true,
  );
  await page.getByLabel("每局 / 房间人数", { exact: true }).fill("16");
  await page
    .getByLabel("每局 / 房间人数", { exact: true })
    .waitFor({ state: "hidden" });
  await page
    .getByRole("checkbox", { name: "在线多人游戏", exact: true })
    .check();
  assert.equal(
    await page.getByLabel("峰值同时在线人数", { exact: true }).inputValue(),
    "1200",
  );
  await apply();
  await open("修改在线多人设定");
  await dialog().getByLabel("峰值同时在线人数", { exact: true }).fill("5000");
  await dialog().getByRole("button", { name: "取消", exact: true }).click();
  await page
    .locator(".overview-online-summary")
    .getByText(/峰值 1200 人/)
    .waitFor();
  await page.screenshot({ path: path.join(output, "20-overview-editing.png") });
  await page.getByRole("button", { name: "保存设定", exact: true }).click();
  await waitState(
    page,
    (state) =>
      state.projects.find((p) => p.id === project.id).blueprintRevision === 2,
  );
  const risk = page.getByRole("dialog", { name: "项目设定已更改，存在风险" });
  await risk.waitFor();
  await page.waitForTimeout(4500);
  assert.equal(
    await risk.isVisible(),
    true,
    "Risk warning must not expire like a toast",
  );
  await page.screenshot({ path: path.join(output, "21-overview-risk.png") });
  await risk.getByRole("button", { name: "我已知晓", exact: true }).click();
  assert.equal(await editSwitch().isChecked(), false);
  assert.equal(
    await overview().locator("input:not([role=switch]),select").count(),
    0,
  );
  const overviewSaved = (await api("state")).projects.find(
    (p) => p.id === project.id,
  );
  assert.equal(overviewSaved.name, "夜航：失物招领");
  await assert.rejects(
    api("project.blueprint.save", {
      id: project.id,
      blueprint: { ...overviewSaved.blueprint, size: "aaa" },
      expectedRevision: 2,
    }),
    /基础设定已锁定/,
  );
  const { name, genres, theme, audience, size, style, online } = {
    ...overviewSaved.blueprint,
    name: overviewSaved.name,
  };
  await assert.rejects(
    api("project.overview.save", {
      id: project.id,
      overview: { name, genres, theme, audience, size, style, online },
      expectedRevision: 2,
    }),
    /true|allowRiskyChanges/,
  );
  await app.evaluate(({ BrowserWindow }) =>
    BrowserWindow.getAllWindows()[0].setContentSize(960, 680),
  );
  assert.ok(
    await overview().evaluate((n) => n.scrollWidth <= n.clientWidth + 1),
  );
  await page.screenshot({ path: path.join(output, "22-overview-compact.png") });
  await app.evaluate(({ BrowserWindow }) =>
    BrowserWindow.getAllWindows()[0].setContentSize(1120, 760),
  );
  await editSwitch().check();
  await page.getByLabel("游戏名称", { exact: true }).fill("不保存此名字");
  await editSwitch().click();
  await page
    .getByRole("button", { name: "放弃修改并锁定", exact: true })
    .click();
  assert.equal(await editSwitch().isChecked(), false);
  await page
    .getByRole("heading", { name: "夜航：失物招领", exact: true })
    .waitFor();
  await page
    .locator(".rail-nav")
    .getByRole("button", { name: "项目设置", exact: true })
    .click();
  assert.equal(await page.getByLabel("主类别", { exact: true }).count(), 0);
  await page.getByRole("button", { name: "前往总览编辑", exact: true }).click();
  assert.equal(await editSwitch().isChecked(), false);
  await page
    .locator(".rail-nav")
    .getByRole("button", { name: "项目设置", exact: true })
    .click();
  await page.screenshot({ path: path.join(output, "09-blueprint-basics.png") });
  for (const [phase, slider, image] of [
    ["设计", "故事性", "10-blueprint-design.png"],
    ["开发", "人工智能", "11-blueprint-development.png"],
    ["后期", "优化", "12-blueprint-post.png"],
  ]) {
    await page.getByRole("tab", { name: phase, exact: true }).click();
    assert.equal(await plan().getByRole("slider").count(), 5);
    await page
      .getByRole("slider", { name: `${slider}投入`, exact: true })
      .focus();
    await page.keyboard.press("End");
    assert.equal(
      await page
        .getByRole("slider", { name: `${slider}投入`, exact: true })
        .inputValue(),
      "5",
    );
    await page
      .getByRole("button", { name: "预览阶段任务", exact: true })
      .click();
    await page
      .getByRole("dialog")
      .getByText("规划预览，不调用 Codex，不生成素材或修改项目。", {
        exact: true,
      })
      .waitFor();
    assert.equal(await page.locator(".planned-task-list > div").count(), 5);
    await page.keyboard.press("Escape");
    await page.getByRole("dialog").waitFor({ state: "hidden" });
    await page.screenshot({ path: path.join(output, image) });
  }
  await page
    .locator(".rail-nav")
    .getByRole("button", { name: "素材", exact: true })
    .click();
  await page
    .locator(".rail-nav")
    .getByRole("button", { name: "项目设置", exact: true })
    .click();
  await page.getByRole("tab", { name: "设计", exact: true }).click();
  assert.equal(
    await page
      .getByRole("slider", { name: "故事性投入", exact: true })
      .inputValue(),
    "5",
    "Unsaved planning survives workspace navigation",
  );
  await page.getByRole("tab", { name: "发行准备", exact: true }).click();
  for (const name of ["联系媒体发布", "发布试玩版", "进行广告宣传"]) {
    await page
      .locator(".campaign-row")
      .filter({ hasText: name })
      .getByRole("button", { name: "规划", exact: true })
      .click();
    const dialog = page.getByRole("dialog");
    await dialog.getByRole("checkbox").first().check();
    await dialog
      .getByLabel("目标与约束", { exact: true })
      .fill(`${name}：仅记录验收草案，不向外发送或付费。`);
    await dialog.getByRole("button", { name: "保留草案", exact: true }).click();
    await dialog.waitFor({ state: "hidden" });
  }
  await page.getByRole("button", { name: "保存规划", exact: true }).click();
  const saved = await waitState(
    page,
    (state) =>
      state.projects.find((p) => p.id === project.id).blueprintRevision === 3,
  );
  let blueprint = saved.projects.find((p) => p.id === project.id).blueprint;
  assert.equal(blueprint.priorities.story, 5);
  assert.equal(blueprint.priorities.ai, 5);
  assert.equal(blueprint.priorities.optimization, 5);
  assert.equal(
    Object.values(blueprint.campaigns).filter((c) => c.notes.length).length,
    3,
  );
  assert.deepEqual(
    await tree(project.path),
    baseline,
    "Planning must not modify the Godot project",
  );
  assert.equal(
    saved.tasks.length,
    initial.tasks.length,
    "Phase previews and campaign plans must not enqueue execution",
  );
  await page.screenshot({ path: path.join(output, "13-blueprint-launch.png") });
  await app.evaluate(({ BrowserWindow }) =>
    BrowserWindow.getAllWindows()[0].setContentSize(960, 680),
  );
  await page.getByRole("tab", { name: "开发", exact: true }).click();
  assert.ok(await plan().evaluate((n) => n.scrollWidth <= n.clientWidth + 1));
  assert.ok(
    await page
      .getByRole("button", { name: "保存规划", exact: true })
      .evaluate((n) => {
        const r = n.getBoundingClientRect();
        return r.top >= 50 && r.bottom <= innerHeight;
      }),
  );
  await page.screenshot({
    path: path.join(output, "14-blueprint-compact.png"),
  });
  await app.evaluate(({ BrowserWindow }) =>
    BrowserWindow.getAllWindows()[0].setContentSize(1120, 760),
  );
  blueprint = await verifyPackages({
    page,
    app,
    output,
    project,
    api,
    waitState,
  });
  assert.deepEqual(
    await tree(project.path),
    baseline,
    "Function package planning must not change game files",
  );
  await page
    .locator(".rail-nav")
    .getByRole("button", { name: "任务", exact: true })
    .click();
  return {
    blueprint,
    unchangedGameFiles: Object.keys(baseline).length,
    tasksCreated: 0,
    checks: [
      "overview defaults locked and relocks after navigation, save and confirmed discard",
      "overview draft survives navigation and canceled discard",
      "foundation edits require explicit risky-save API and cannot bypass project-settings lock",
      "successful foundation changes show a non-expiring manually acknowledged risk dialog",
      "renamed overview persists without renaming or modifying game files",
      "minimum window displays all overview fields without horizontal overflow",
      "overview uses a single top-left name and a separate right-top settings card with empty left body",
      "secondary card dialogs support search, sorting, main-secondary roles, custom themes and keyboard cancel",
      "card Apply updates only the draft and Cancel preserves previous choices with a fixed visible footer",
      "64 genre choices and 49 presets plus custom theme",
      "audience conflicts and online capacity validation",
      "15 independent stage priorities and meanings",
      "phase task previews do not execute",
      "campaign plans persist without sending, publishing or spending",
      "unsaved planning survives workspace navigation",
      "planning does not change Godot files or create tasks",
      "minimum window retains readable stage controls and save action",
      "all 68 engine capabilities and 10 source packages browse as distinct planning items",
      "package search, categories, selected filter, select-all, meanings and cross-page drafts",
      "engine package IDs are rejected by the real integration API without creating tasks",
    ],
  };
}
