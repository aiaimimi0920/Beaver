import assert from "node:assert/strict";
import path from "node:path";

export async function verifyPackages({
  page,
  app,
  output,
  project,
  api,
  waitState,
}) {
  const nav = (name) =>
    page
      .locator(".rail-nav")
      .getByRole("button", { name, exact: true })
      .click();
  await nav("功能块");
  const planner = page.getByRole("region", {
    name: "功能包规划目录",
    exact: true,
  });
  const choices = () => planner.locator(".package-choice");
  assert.equal(await choices().count(), 78);
  assert.equal(
    await page
      .getByRole("button", { name: "接入 / 更新", exact: true })
      .count(),
    0,
  );
  await planner
    .getByLabel("功能包来源", { exact: true })
    .selectOption("planning");
  assert.equal(await choices().count(), 68);
  await planner.getByRole("button", { name: "全选当前", exact: true }).click();
  assert.equal(
    await planner.locator(".package-picker input:checked").count(),
    68,
  );
  await planner.getByLabel("功能包来源", { exact: true }).selectOption("");
  await planner.getByRole("button", { name: "全选当前", exact: true }).click();
  assert.equal(
    await planner.locator(".package-picker input:checked").count(),
    78,
  );
  await planner.getByRole("button", { name: "清空", exact: true }).click();
  await planner.getByRole("button", { name: "加入推荐", exact: true }).click();
  await planner.getByLabel("搜索功能包", { exact: true }).fill("cloud_save");
  assert.equal(await choices().count(), 1);
  await planner
    .getByRole("checkbox", { name: "规划 云端存档", exact: true })
    .check();
  await planner
    .getByRole("button", { name: "云端存档含义", exact: true })
    .click();
  await planner
    .locator(".package-detail")
    .getByText(/不上传文件/)
    .waitFor();
  await planner.getByLabel("搜索功能包", { exact: true }).fill("");
  await planner
    .getByLabel("功能包分组", { exact: true })
    .selectOption("商业化");
  assert.equal(await choices().count(), 2);
  await planner
    .getByRole("checkbox", { name: "规划 游戏内购买", exact: true })
    .check();
  await planner
    .locator(".package-detail")
    .getByText(/不接入真实支付或扣款/)
    .waitFor();
  await page.screenshot({
    path: path.join(output, "15-engine-package-meaning.png"),
  });
  await planner.getByLabel("功能包分组", { exact: true }).selectOption("");
  await planner.getByRole("checkbox", { name: "仅已选", exact: true }).check();
  assert.equal(await choices().count(), 10);
  await page.screenshot({
    path: path.join(output, "16-engine-packages-selected.png"),
  });
  await page.getByRole("tab", { name: "源码包", exact: true }).click();
  assert.equal(await page.locator(".feature-row").count(), 10);
  await page.getByRole("tab", { name: "功能包规划", exact: true }).click();
  assert.equal(
    await planner.locator(".package-picker input:checked").count(),
    10,
  );
  await nav("项目设置");
  await page.getByText("功能规划（10）", { exact: true }).click();
  assert.equal(
    await planner
      .getByRole("checkbox", { name: "规划 云端存档", exact: true })
      .isChecked(),
    true,
  );
  await nav("功能块");
  await page.getByRole("button", { name: "保存规划", exact: true }).click();
  const saved = await waitState(
    page,
    (s) => s.projects.find((p) => p.id === project.id).blueprintRevision === 4,
  );
  const blueprint = saved.projects.find((p) => p.id === project.id).blueprint;
  assert.equal(blueprint.plannedFeatures.length, 10);
  assert.ok(blueprint.plannedFeatures.includes("engine-cloud-save"));
  assert.ok(blueprint.plannedFeatures.includes("engine-iap"));
  await assert.rejects(
    api("feature.add", {
      projectId: project.id,
      featureId: "engine-cloud-save",
    }),
    /仅支持规划/,
  );
  assert.equal((await api("state")).tasks.length, 0);
  await planner
    .getByLabel("搜索功能包", { exact: true })
    .fill("没有这个能力包");
  assert.equal(await choices().count(), 0);
  await planner.getByText("没有匹配的功能包", { exact: true }).waitFor();
  await planner.getByLabel("搜索功能包", { exact: true }).fill("");
  await planner
    .getByLabel("功能包来源", { exact: true })
    .selectOption("planning");
  await app.evaluate(({ BrowserWindow }) =>
    BrowserWindow.getAllWindows()[0].setContentSize(960, 680),
  );
  assert.ok(await planner.evaluate((n) => n.scrollWidth <= n.clientWidth + 1));
  assert.ok(
    await page
      .getByRole("button", { name: "保存规划", exact: true })
      .evaluate((n) => n.getBoundingClientRect().bottom <= innerHeight),
  );
  await page.screenshot({
    path: path.join(output, "17-engine-packages-compact.png"),
  });
  await app.evaluate(({ BrowserWindow }) =>
    BrowserWindow.getAllWindows()[0].setContentSize(1120, 760),
  );
  return blueprint;
}
