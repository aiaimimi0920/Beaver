import fs from "node:fs/promises";
import path from "node:path";
import { createRequire } from "node:module";
import assert from "node:assert/strict";
import { spawn, execFile } from "node:child_process";
import { promisify } from "node:util";
import { verifyBlueprint } from "./blueprint-smoke.mjs";

// Uses an explicitly supplied external Playwright installation, not a fake browser bridge.
if (!process.env.BEAVER_PLAYWRIGHT)
  throw new Error(
    "Set BEAVER_PLAYWRIGHT to the installed playwright package directory",
  );
const { _electron } = createRequire(import.meta.url)(
  process.env.BEAVER_PLAYWRIGHT,
);
const root = process.cwd();
const output = path.join(root, "output/validation", `desktop-${Date.now()}`);
await fs.mkdir(output, { recursive: true });
// The import test must never read or copy the developer's personal credentials.
const fixtureHome = path.join(output, "fixture-codex");
const fixtureKey = "beaver-smoke-fixture-not-a-real-key";
await fs.mkdir(fixtureHome);
await fs.writeFile(
  path.join(fixtureHome, "config.toml"),
  'model = "beaver-fixture-model"\nmodel_provider = "fixture"\n[model_providers.fixture]\nbase_url = "http://127.0.0.1:1/v1"\nwire_api = "responses"\n',
);
await fs.writeFile(
  path.join(fixtureHome, "auth.json"),
  JSON.stringify({ OPENAI_API_KEY: fixtureKey }),
);
const env = {
  ...process.env,
  BEAVER_DATA_DIR: path.join(output, "data"),
  BEAVER_TEST_HIDE: "1",
  CODEX_HOME: fixtureHome,
};
delete env.ELECTRON_RUN_AS_NODE;
delete env.CODEX_PROFILE;
delete env.OPENAI_API_KEY;
if (process.env.BEAVER_EXPECT_DESKTOP_TOOLS === "1") {
  delete env.GODOT_PATH;
  delete env.BLENDER_PATH;
}
const executablePath =
  process.env.BEAVER_EXECUTABLE ||
  path.join(root, "node_modules/electron/dist/electron.exe");
const args = process.env.BEAVER_EXECUTABLE ? [] : [root];
async function waitState(page, predicate) {
  // Poll awaited IPC in Node; waitForFunction treats a Promise as a truthy predicate.
  const deadline = Date.now() + 30000;
  while (Date.now() < deadline) {
    const state = await page.evaluate(() => window.beaver.call("state"));
    if (predicate(state)) return state;
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  throw new Error("Timed out waiting for persisted application state");
}
async function readRailIcons(page) {
  return page
    .locator(".rail-toggle > .icon, .rail-icon > .icon")
    .evaluateAll((icons) =>
      icons.map((icon) => {
        const button = icon.closest("button");
        const { x, y, width, height } = icon.getBoundingClientRect();
        return {
          name: button.classList.contains("rail-toggle")
            ? "toggle"
            : button.getAttribute("aria-label"),
          x,
          y,
          width,
          height,
        };
      }),
    );
}
let app;
let game;
const errors = [];
try {
  app = await _electron.launch({ executablePath, args, env, timeout: 30000 });
  const page = await app.firstWindow();
  page.on("pageerror", (e) => errors.push(e.message));
  await page.locator(".welcome").waitFor();
  const brand = await page.locator("img.brand-mark").evaluate(async (mark) => {
    await mark.decode();
    const bounds = mark.getBoundingClientRect();
    return {
      src: mark.getAttribute("src"),
      width: bounds.width,
      height: bounds.height,
      loaded: mark.naturalWidth > 0,
      favicon: document.querySelector('link[rel="icon"]').getAttribute("href"),
    };
  });
  assert.deepEqual(brand, {
    src: "./beaver.svg",
    width: 28,
    height: 28,
    loaded: true,
    favicon: "./beaver.svg",
  });
  let executableIcon;
  if (process.env.BEAVER_EXECUTABLE) {
    const nativeWindow = await app.evaluate(({ BrowserWindow }) =>
      BrowserWindow.getAllWindows()[0]
        .getNativeWindowHandle()
        .readBigUInt64LE()
        .toString(),
    );
    // Direct native extraction avoids the shell's associated-file icon cache.
    const { stdout } = await promisify(execFile)(
      "powershell.exe",
      [
        "-NoLogo",
        "-NoProfile",
        "-NonInteractive",
        "-ExecutionPolicy",
        "Bypass",
        "-File",
        path.join(root, "scripts/native-branding-smoke.ps1"),
        "-ExecutablePath",
        await app.evaluate(({ app }) => app.getPath("exe")),
        "-OutputPath",
        output,
        "-WindowHandle",
        nativeWindow,
      ],
      { windowsHide: true, timeout: 30000 },
    );
    executableIcon = JSON.parse(stdout.trim());
    assert.equal(executableIcon.icons.length, 6);
    await fs.writeFile(
      path.join(output, "00-windows-exe-icon.json"),
      JSON.stringify(executableIcon, null, 2),
    );
  }
  const api = (method, input) =>
    page.evaluate(([m, i]) => window.beaver.call(m, i), [method, input]);
  const detected = await api("tools.detect", {
    tools: { codex: "", godot: "", blender: "", node: "" },
  });
  if (process.env.BEAVER_EXPECT_DESKTOP_TOOLS === "1") {
    for (const name of ["godot", "blender"]) {
      const tool = detected.find((t) => t.name === name);
      assert.ok(
        tool?.available,
        `${name} must be discovered without a configured path or environment override`,
      );
      assert.ok((await fs.stat(tool.path)).isFile());
    }
  }
  const setup = page.getByRole("dialog", { name: "未找到必要工具" });
  if (await setup.isVisible())
    await setup.getByRole("button", { name: "稍后准备" }).click();
  await app.evaluate(({ BrowserWindow }) =>
    BrowserWindow.getAllWindows()[0].showInactive(),
  );
  const initialBounds = await app.evaluate(({ BrowserWindow }) =>
    BrowserWindow.getAllWindows()[0].getBounds(),
  );
  const initialContent = await page.evaluate(() => ({
    width: innerWidth,
    height: innerHeight,
  }));
  assert.deepEqual(initialContent, { width: 1120, height: 760 });
  assert.deepEqual(
    await page
      .locator(".window-controls button")
      .evaluateAll((nodes) =>
        nodes.map((node) => node.getAttribute("aria-label")),
      ),
    ["刷新", "最小化", "最大化", "关闭窗口"],
  );
  assert.equal(
    await page.evaluate(async () => {
      try {
        await window.beaver.windowControl("exit");
        return false;
      } catch {
        return true;
      }
    }),
    true,
    "Window IPC must reject unknown commands",
  );
  const preload = path.join(
    await app.evaluate(({ app }) => app.getAppPath()),
    "dist",
    "preload.cjs",
  );
  assert.equal(
    await app.evaluate(async ({ BrowserWindow }, preload) => {
      const other = new BrowserWindow({
        show: false,
        webPreferences: {
          preload,
          contextIsolation: true,
          sandbox: true,
        },
      });
      try {
        await other.loadURL(
          "data:text/html,<html><body>IPC isolation probe</body></html>",
        );
        return await other.webContents.executeJavaScript(
          "window.beaver.windowControl('minimize').then(() => false, () => true)",
        );
      } finally {
        other.destroy();
      }
    }, preload),
    true,
    "Window IPC must reject a different webContents sender",
  );
  await page.getByRole("button", { name: "最大化", exact: true }).click();
  await page.getByRole("button", { name: "还原窗口", exact: true }).waitFor();
  assert.equal(
    await app.evaluate(({ BrowserWindow }) =>
      BrowserWindow.getAllWindows()[0].isMaximized(),
    ),
    true,
  );
  await page.getByRole("button", { name: "还原窗口", exact: true }).click();
  await page.getByRole("button", { name: "最大化", exact: true }).waitFor();
  await page.getByRole("button", { name: "最小化", exact: true }).click();
  assert.equal(
    await app.evaluate(({ BrowserWindow }) =>
      BrowserWindow.getAllWindows()[0].isMinimized(),
    ),
    true,
  );
  await app.evaluate(({ BrowserWindow }) => {
    const window = BrowserWindow.getAllWindows()[0];
    window.restore();
    window.showInactive();
  });
  await page.screenshot({ path: path.join(output, "01-empty.png") });
  await page.getByRole("button", { name: "新建项目", exact: true }).click();
  await page.getByLabel("项目名称", { exact: true }).waitFor();
  await page.waitForFunction(() =>
    document.querySelector('[role="dialog"]')?.contains(document.activeElement),
  );
  await page.keyboard.press("Escape");
  await page.getByRole("dialog").waitFor({ state: "hidden" });
  assert.ok(
    await page
      .getByRole("button", { name: "新建项目", exact: true })
      .evaluate((n) => document.activeElement === n),
  );
  await page.getByRole("button", { name: "新建项目", exact: true }).click();
  await page.getByLabel("项目名称", { exact: true }).fill("原创夜航验收");
  await page.getByLabel("项目存放目录").fill(output);
  await page.getByRole("tab", { name: "2. 创作方向", exact: true }).click();
  assert.equal(
    await page.getByLabel("主类别", { exact: true }).locator("option").count(),
    64,
  );
  await page.getByLabel("副类别", { exact: true }).selectOption("management");
  assert.ok(
    await page.getByRole("dialog").evaluate((n) => {
      const r = n.getBoundingClientRect();
      const style = getComputedStyle(n);
      return (
        Math.abs(r.width - 740) < 1 &&
        r.height <= innerHeight - 100 + 1 &&
        style.overflow === "hidden"
      );
    }),
    "Creation dialog must retain its compact size and scroll only its body",
  );
  await page.screenshot({ path: path.join(output, "01-create-direction.png") });
  await page.getByRole("tab", { name: "3. 功能规划", exact: true }).click();
  assert.equal(await page.locator(".package-choice").count(), 78);
  await page.getByLabel("功能包来源", { exact: true }).selectOption("planning");
  assert.equal(await page.locator(".package-choice").count(), 68);
  await page
    .getByRole("checkbox", { name: "规划 手柄输入", exact: true })
    .check();
  await page.screenshot({
    path: path.join(output, "18-create-engine-packages.png"),
  });
  await page
    .getByRole("checkbox", { name: "规划 手柄输入", exact: true })
    .uncheck();
  await page.getByLabel("功能包来源", { exact: true }).selectOption("");
  await page.getByRole("button", { name: "加入推荐", exact: true }).click();
  assert.equal(
    await page
      .getByRole("dialog")
      .getByRole("checkbox", { checked: true })
      .count(),
    8,
  );
  await page.getByRole("button", { name: "清空", exact: true }).click();
  assert.equal(
    await page
      .getByRole("dialog")
      .getByRole("checkbox", { checked: true })
      .count(),
    0,
  );
  await page.getByRole("button", { name: "加入推荐", exact: true }).click();
  await page.screenshot({ path: path.join(output, "01-create-catalog.png") });
  await app.evaluate(({ BrowserWindow }) =>
    BrowserWindow.getAllWindows()[0].setContentSize(960, 680),
  );
  const createButton = page
    .getByRole("dialog")
    .getByRole("button", { name: "创建项目", exact: true });
  await createButton.scrollIntoViewIfNeeded();
  assert.ok(
    await createButton.evaluate((button) => {
      const rect = button.getBoundingClientRect();
      return (
        rect.top >= 50 &&
        rect.bottom <= innerHeight &&
        document.documentElement.scrollWidth <= innerWidth
      );
    }),
    "Minimum window must retain an accessible creation action without horizontal overflow",
  );
  await page.screenshot({ path: path.join(output, "01-create-compact.png") });
  await app.evaluate(({ BrowserWindow }) =>
    BrowserWindow.getAllWindows()[0].setContentSize(1120, 760),
  );
  await page
    .getByRole("dialog")
    .getByRole("button", { name: "创建项目", exact: true })
    .click();
  await page.getByRole("dialog").waitFor({ state: "hidden" });
  await page.getByRole("region", { name: "游戏总览", exact: true }).waitFor();
  const state = await api("state");
  assert.equal(state.projects.length, 1);
  const project = state.projects[0];
  assert.equal(
    project.design,
    undefined,
    "UI prototype does not replace executable task context",
  );
  const blueprintProof = await verifyBlueprint({
    page,
    app,
    output,
    project,
    api,
    waitState,
  });
  const expectedDesign = {
    genres: ["narrative", "management"],
    theme: "cyberpunk",
    style: "pixel",
    scope: "prototype",
  };
  // Keep regression coverage for the previous executable direction file separately.
  await fs.writeFile(
    path.join(project.path, "beaver.project.json"),
    JSON.stringify({ schemaVersion: 1, design: expectedDesign }),
  );
  assert.ok(
    (
      await fs.readFile(path.join(project.path, "project.godot"), "utf8")
    ).includes("config_version=5"),
  );
  const imported = await api("project.import", { path: project.path });
  assert.equal(imported.id, project.id);
  await fs.writeFile(
    path.join(project.path, "reference.svg"),
    '<svg xmlns="http://www.w3.org/2000/svg" width="600" height="360"><rect width="600" height="360" fill="#0e1218"/><circle cx="300" cy="160" r="100" fill="#d9ff38"/><text x="40" y="325" fill="white" font-size="24">Beaver local preview fixture</text></svg>',
  );
  await fs.writeFile(
    path.join(project.path, "triangle.obj"),
    "o PreviewFixture\nv -1 0 0\nv 1 0 0\nv 0 2 0\nf 1 2 3\n",
  );
  const wav = Buffer.alloc(44 + 8000);
  wav.write("RIFF", 0);
  wav.writeUInt32LE(wav.length - 8, 4);
  wav.write("WAVEfmt ", 8);
  wav.writeUInt32LE(16, 16);
  wav.writeUInt16LE(1, 20);
  wav.writeUInt16LE(1, 22);
  wav.writeUInt32LE(8000, 24);
  wav.writeUInt32LE(16000, 28);
  wav.writeUInt16LE(2, 32);
  wav.writeUInt16LE(16, 34);
  wav.write("data", 36);
  wav.writeUInt32LE(8000, 40);
  for (let i = 0; i < 4000; i++)
    wav.writeInt16LE(
      Math.round(Math.sin((i * Math.PI * 440) / 4000) * 1000),
      44 + i * 2,
    );
  await fs.writeFile(path.join(project.path, "audition.wav"), wav);
  await fs.writeFile(
    path.join(project.path, "missing-preview.txt"),
    "Temporary preview fixture",
  );
  const metrics = await page.evaluate(() => ({
    headings: [...document.querySelectorAll(".main h1")].map(
      (n) => n.textContent,
    ),
    decorativeLabels: document.querySelectorAll(
      ".section-label,.rail-section,.principles",
    ).length,
    navHeight: document
      .querySelector(".rail nav button")
      .getBoundingClientRect().height,
    composerHeight: document.querySelector(".composer").getBoundingClientRect()
      .height,
    railWidth: document.querySelector(".rail").getBoundingClientRect().width,
    titlebarHeight: document.querySelector(".topbar").getBoundingClientRect()
      .height,
    headerHeight: document.querySelector(".page-header").getBoundingClientRect()
      .height,
    footerBottom:
      innerHeight -
      document.querySelector(".rail-footer").getBoundingClientRect().bottom,
    dragRegion: getComputedStyle(
      document.querySelector(".titlebar-context"),
    ).getPropertyValue("-webkit-app-region"),
    buttonRegion: getComputedStyle(
      document.querySelector(".window-control"),
    ).getPropertyValue("-webkit-app-region"),
  }));
  assert.deepEqual(metrics.headings, ["任务"]);
  assert.equal(metrics.decorativeLabels, 0);
  assert.equal(metrics.navHeight, 36);
  assert.equal(metrics.railWidth, 186);
  assert.equal(metrics.titlebarHeight, 50);
  assert.equal(metrics.headerHeight, 86);
  assert.ok(Math.abs(metrics.footerBottom - 9) < 0.01);
  assert.equal(metrics.dragRegion, "drag");
  assert.equal(metrics.buttonRegion, "no-drag");
  assert.ok(metrics.composerHeight <= 160);
  await page.screenshot({ path: path.join(output, "02-tasks.png") });
  await page.getByLabel("任务目标").fill("折叠与刷新不能丢失任务草稿");
  await page.getByRole("button", { name: "执行选项", exact: true }).click();
  await page.getByLabel("停止条件", { exact: true }).fill("遇到付费时停止");
  await page.getByRole("button", { name: "执行选项", exact: true }).click();
  const railAnchors = { expanded: await readRailIcons(page) };
  assert.equal(railAnchors.expanded.length, 7);
  await page.getByRole("button", { name: "收起侧栏" }).click();
  assert.equal(
    await page
      .locator(".rail")
      .evaluate((n) => n.getBoundingClientRect().width),
    52,
  );
  railAnchors.collapsed = await readRailIcons(page);
  await fs.writeFile(
    path.join(output, "rail-anchors.json"),
    JSON.stringify(railAnchors, null, 2),
  );
  assert.deepEqual(
    railAnchors.collapsed,
    railAnchors.expanded,
    "Sidebar toggle, navigation and settings icons must not move when collapsed",
  );
  await page.getByRole("button", { name: "刷新", exact: true }).click();
  assert.equal(
    await page.getByLabel("任务目标").inputValue(),
    "折叠与刷新不能丢失任务草稿",
  );
  assert.equal(
    await page.getByLabel("停止条件", { exact: true }).inputValue(),
    "遇到付费时停止",
  );
  await page.screenshot({ path: path.join(output, "02-tasks-collapsed.png") });
  await page
    .locator(".rail-footer")
    .getByRole("button", { name: "设置", exact: true })
    .click();
  await page.getByRole("heading", { name: "设置", exact: true }).waitFor();
  assert.deepEqual(await readRailIcons(page), railAnchors.collapsed);
  await page.getByRole("button", { name: "返回工作区" }).click();
  await page.getByRole("heading", { name: "任务", exact: true }).waitFor();
  await page.getByRole("button", { name: "展开侧栏" }).click();
  railAnchors.restored = await readRailIcons(page);
  assert.deepEqual(railAnchors.restored, railAnchors.expanded);
  for (const icon of railAnchors.expanded)
    // A 1px border can snap to a fractional CSS pixel at Windows 150% DPI.
    assert.ok(
      Math.abs(icon.x + icon.width / 2 - 26) < 0.5,
      `${icon.name} icon column`,
    );
  const bounds = await app.evaluate(({ BrowserWindow }) => {
    const window = BrowserWindow.getAllWindows()[0];
    const original = window.getBounds();
    window.setBounds({ width: 960, height: 680 });
    return original;
  });
  await page.waitForTimeout(250);
  assert.ok(
    await page.evaluate(() => {
      const n = document.querySelector(".page-content");
      return n.scrollWidth <= n.clientWidth + 1;
    }),
    "Compact window must not scroll horizontally",
  );
  await page.screenshot({ path: path.join(output, "02-tasks-compact.png") });
  railAnchors.compactExpanded = await readRailIcons(page);
  await page.getByRole("button", { name: "收起侧栏" }).click();
  railAnchors.compactCollapsed = await readRailIcons(page);
  assert.deepEqual(railAnchors.compactCollapsed, railAnchors.compactExpanded);
  await page.screenshot({
    path: path.join(output, "02-tasks-compact-collapsed.png"),
  });
  await page.getByRole("button", { name: "展开侧栏" }).click();
  assert.deepEqual(await readRailIcons(page), railAnchors.compactExpanded);
  await fs.writeFile(
    path.join(output, "rail-anchors.json"),
    JSON.stringify(railAnchors, null, 2),
  );
  await app.evaluate(
    ({ BrowserWindow }, original) =>
      BrowserWindow.getAllWindows()[0].setBounds(original),
    bounds,
  );
  await page.locator("nav").getByRole("button", { name: /素材/ }).click();
  await page.getByText("main.gd", { exact: true }).waitFor();
  await page
    .getByRole("button", { name: "预览 reference.svg", exact: true })
    .focus();
  await page.keyboard.press("Enter");
  await page.waitForFunction(
    () => document.querySelector('img[alt="素材预览"]')?.naturalWidth === 600,
  );
  await page.getByLabel("素材反馈").fill("统一这里的风格，并保留圆形构图");
  const frame = await page.locator(".image-frame").boundingBox();
  assert.ok(frame);
  await page.mouse.move(
    frame.x + frame.width * 0.2,
    frame.y + frame.height * 0.2,
  );
  await page.mouse.down();
  await page.mouse.move(
    frame.x + frame.width * 0.7,
    frame.y + frame.height * 0.7,
    { steps: 5 },
  );
  await page.mouse.up();
  await page.locator(".region").waitFor();
  await page.screenshot({ path: path.join(output, "03-assets.png") });
  await app.evaluate(({ BrowserWindow }) =>
    BrowserWindow.getAllWindows()[0].setBounds({ width: 960, height: 680 }),
  );
  await page.waitForTimeout(250);
  assert.ok(
    await page.evaluate(() => {
      const n = document.querySelector(".page-content");
      return n.scrollWidth <= n.clientWidth + 1;
    }),
    "Assets must fit the actual minimum window size",
  );
  await page.screenshot({ path: path.join(output, "03-assets-compact.png") });
  await app.evaluate(
    ({ BrowserWindow }, original) =>
      BrowserWindow.getAllWindows()[0].setBounds(original),
    bounds,
  );
  await page.getByRole("button", { name: /加入任务/ }).click();
  await page.getByText("reference.svg · 已框选", { exact: true }).waitFor();
  await page.locator("nav").getByRole("button", { name: /素材/ }).click();
  await page
    .locator("article.asset-card")
    .filter({ hasText: "triangle.obj" })
    .click();
  await page.locator(".model-view canvas").waitFor();
  await page.locator('.model-view[data-loaded="true"]').waitFor();
  await page.screenshot({ path: path.join(output, "03-model.png") });
  await page
    .locator("article.asset-card")
    .filter({ hasText: "audition.wav" })
    .click();
  await page.waitForFunction(
    () => document.querySelector("audio")?.readyState >= 2,
  );
  await fs.unlink(path.join(project.path, "missing-preview.txt"));
  await page
    .getByRole("button", { name: "预览 missing-preview.txt", exact: true })
    .click();
  await page.locator(".inspector").getByRole("alert").waitFor();
  await page
    .locator("nav")
    .getByRole("button", { name: /功能块/ })
    .click();
  await page.getByRole("heading", { name: "功能块", exact: true }).waitFor();
  await page.getByRole("tab", { name: "源码包", exact: true }).click();
  assert.equal(await page.locator(".feature-row").count(), 10);
  await page
    .getByLabel("功能块分类", { exact: true })
    .selectOption("progression");
  assert.equal(await page.locator(".feature-row").count(), 1);
  await page.getByLabel("功能块分类", { exact: true }).selectOption("");
  await page.getByLabel("搜索功能块", { exact: true }).fill("配方");
  assert.equal(await page.locator(".feature-row").count(), 1);
  await page.getByLabel("搜索功能块", { exact: true }).fill("");
  await page.getByRole("checkbox", { name: "适合此项目" }).check();
  assert.equal(await page.locator(".feature-row").count(), 8);
  await page.getByRole("checkbox", { name: "适合此项目" }).uncheck();
  await page.screenshot({ path: path.join(output, "04-features.png") });
  const featureDesign = { ...expectedDesign, theme: "fantasy" };
  await fs.writeFile(
    path.join(project.path, "beaver.project.json"),
    JSON.stringify({ schemaVersion: 1, design: featureDesign }),
  );
  await page
    .locator(".feature-row")
    .filter({ hasText: "物品与背包" })
    .getByRole("button", { name: "接入 / 更新", exact: true })
    .click();
  const featureState = await waitState(page, (state) =>
    state.tasks.some(
      (t) => t.feature?.id === "inventory" && t.status === "failed",
    ),
  );
  const featureTask = featureState.tasks.find(
    (t) => t.feature?.id === "inventory",
  );
  assert.deepEqual(
    featureTask.design,
    featureDesign,
    "New task must use captured project metadata, not the cached project catalog",
  );
  assert.match(
    await fs.readFile(
      path.join(
        featureTask.workspace,
        ".beaver-context/feature/new/inventory.gd",
      ),
      "utf8",
    ),
    /class_name BeaverInventory/,
  );
  assert.equal(
    JSON.parse(
      await fs.readFile(
        path.join(
          featureTask.workspace,
          ".beaver-context/feature/upstream-changes.json",
        ),
        "utf8",
      ),
    ).to,
    "1.0.0",
  );
  await fs.writeFile(
    path.join(project.path, "beaver.project.json"),
    JSON.stringify({ schemaVersion: 1, design: expectedDesign }),
  );
  await page
    .locator("nav")
    .getByRole("button", { name: /功能块/ })
    .click();
  await page.getByRole("heading", { name: "功能块", exact: true }).waitFor();
  await page
    .locator("nav")
    .getByRole("button", { name: "设置", exact: true })
    .click();
  await page.waitForFunction(
    () => !document.querySelector(".tools-toolbar button")?.disabled,
  );
  assert.deepEqual(await page.locator(".main h1").allTextContents(), ["设置"]);
  assert.equal(await page.locator(".page-header").count(), 0);
  assert.equal(await page.locator(".settings-group[open]").count(), 0);
  assert.equal(
    await page
      .locator(".settings-group summary")
      .first()
      .evaluate((n) => n.getBoundingClientRect().height),
    66,
  );
  await page.screenshot({
    path: path.join(output, "05-settings-overview.png"),
  });
  await page.getByRole("button", { name: "返回工作区" }).click();
  await page.getByRole("heading", { name: "功能块", exact: true }).waitFor();
  await page
    .locator(".rail-footer")
    .getByRole("button", { name: "设置", exact: true })
    .click();
  await page.locator("summary").filter({ hasText: "本机工具" }).click();
  await page.getByRole("button", { name: "重新检测" }).click();
  await page.getByRole("button", { name: "重新检测" }).waitFor();
  await page.getByLabel("codex 路径").waitFor();
  await page.waitForFunction(
    () =>
      document.querySelector('input[aria-label="codex 路径"]').value.length > 0,
    undefined,
    { timeout: 30000 },
  );
  if (process.env.BEAVER_EXPECT_DESKTOP_TOOLS === "1") {
    const godot = detected.find((t) => t.name === "godot");
    assert.equal(await page.getByLabel("godot 路径").inputValue(), godot.path);
    assert.equal(
      await page.getByLabel("blender 路径").inputValue(),
      detected.find((t) => t.name === "blender").path,
    );
    await page
      .getByLabel("godot 路径")
      .fill(path.join(output, "missing-godot.exe"));
    await page.getByRole("button", { name: "重新检测" }).click();
    await page.getByRole("button", { name: "重新检测" }).waitFor();
    assert.match(
      await page
        .locator(".tool-row")
        .filter({ has: page.getByLabel("godot 路径") })
        .innerText(),
      /路径不存在/,
    );
    await page.getByLabel("godot 路径").fill(godot.path);
    await page.getByRole("button", { name: "重新检测" }).click();
    await page.getByRole("button", { name: "重新检测" }).waitFor();
  }
  const codeProvider = page
    .locator(".provider")
    .filter({ hasText: "Codex 编程" });
  await codeProvider.locator("summary").click();
  await codeProvider.getByLabel("模型标识").fill("beaver-ui-draft");
  const imageProvider = page
    .locator(".provider")
    .filter({ hasText: "图像生成 / 编辑" });
  await imageProvider.locator("summary").click();
  assert.equal(await codeProvider.getAttribute("open"), null);
  await codeProvider.locator("summary").click();
  assert.equal(
    await codeProvider.getByLabel("模型标识").inputValue(),
    "beaver-ui-draft",
  );
  await codeProvider.getByLabel("模型标识").fill("");
  await page.screenshot({ path: path.join(output, "05-settings-api.png") });
  const beforeToast = await page.locator(".page-content").boundingBox();
  await codeProvider
    .getByLabel("API Base URL")
    .fill("http://example.invalid/v1");
  await page.getByRole("button", { name: "保存设置" }).click();
  const errorToast = page.locator(".app-toast--error");
  await errorToast.getByRole("alert").waitFor();
  assert.match(await errorToast.innerText(), /非本机 API 必须使用 HTTPS/);
  await page.getByRole("button", { name: "保存设置" }).focus();
  await page.keyboard.press("Enter");
  await errorToast.getByText("重复 2 次", { exact: true }).waitFor();
  assert.equal(await errorToast.count(), 1);
  assert.deepEqual(
    await page.locator(".page-content").boundingBox(),
    beforeToast,
  );
  assert.equal(await page.locator(".banner").count(), 0);
  await codeProvider.getByLabel("API Base URL").fill("");
  await page.getByRole("button", { name: "保存设置" }).focus();
  await page.keyboard.press("Enter");
  const successToast = page.locator(".app-toast--success");
  await successToast.getByRole("status").waitFor();
  await successToast.getByRole("button", { name: "关闭通知" }).focus();
  await page.waitForTimeout(3500);
  assert.ok(
    await successToast.isVisible(),
    "Focused notification must pause auto-dismiss",
  );
  const toastMetrics = await page
    .locator(".app-toast-stack")
    .evaluate((stack) => {
      const box = stack.getBoundingClientRect();
      const items = [...stack.querySelectorAll(".app-toast")].map((item) =>
        item.getBoundingClientRect(),
      );
      return {
        top: box.top,
        right: innerWidth - box.right,
        cssRight: getComputedStyle(stack).right,
        width: box.width,
        count: items.length,
        separated: items[1].top >= items[0].bottom + 7,
        windowControlsClear:
          box.top >
          document.querySelector(".topbar").getBoundingClientRect().bottom,
      };
    });
  assert.ok(Math.abs(toastMetrics.top - 64) < 0.01);
  assert.equal(toastMetrics.cssRight, "18px");
  assert.ok(
    Math.abs(toastMetrics.right - 18) < 1,
    JSON.stringify(toastMetrics),
  );
  assert.equal(toastMetrics.width, 420);
  assert.equal(toastMetrics.count, 2);
  assert.ok(toastMetrics.separated && toastMetrics.windowControlsClear);
  await page.screenshot({ path: path.join(output, "05-notifications.png") });
  await page.locator(".settings-toolbar h1").click();
  await successToast.waitFor({ state: "hidden", timeout: 6000 });
  assert.ok(
    await errorToast.isVisible(),
    "Errors remain available until explicitly dismissed",
  );
  await errorToast.getByRole("button", { name: "关闭通知" }).click();
  await errorToast.waitFor({ state: "hidden" });
  await page.locator("summary").filter({ hasText: "本机工具" }).click();
  await page.getByRole("button", { name: "保存设置" }).click();
  await page.getByText("设置已保存", { exact: true }).waitFor();
  await page.screenshot({ path: path.join(output, "05-settings.png") });
  await app.evaluate(({ BrowserWindow }) =>
    BrowserWindow.getAllWindows()[0].setBounds({ width: 960, height: 680 }),
  );
  await page.getByLabel("blender 路径").scrollIntoViewIfNeeded();
  assert.ok(
    await page.evaluate(() => {
      const n = document.querySelector(".page-content");
      const footer = document
        .querySelector(".rail-footer")
        .getBoundingClientRect();
      const save = document
        .querySelector(".settings-toolbar > button")
        .getBoundingClientRect();
      return (
        n.scrollWidth <= n.clientWidth + 1 &&
        footer.bottom <= innerHeight &&
        save.top >= 50 &&
        save.bottom <= innerHeight
      );
    }),
    "Narrow settings must retain save and bottom-left navigation without horizontal overflow",
  );
  await page.screenshot({ path: path.join(output, "05-settings-compact.png") });
  await app.evaluate(({ BrowserWindow }) =>
    BrowserWindow.getAllWindows()[0].setContentSize(1120, 760),
  );
  const noProvider = await api("task.create", {
    projectId: project.id,
    prompt: "未配置服务时必须如实失败，不应改动游戏",
  });
  assert.deepEqual(noProvider.design, expectedDesign);
  await waitState(page, (state) =>
    state.tasks.some((t) => t.id === noProvider.id && t.status === "failed"),
  );
  const beforeClose = await api("state");
  let ownedPlayChecked = false;
  if (
    process.env.GODOT_PATH ||
    process.env.BEAVER_EXPECT_DESKTOP_TOOLS === "1"
  ) {
    await api("game.play", { id: project.id });
    const deadline = Date.now() + 15000;
    while (!ownedPlayChecked && Date.now() < deadline) {
      ownedPlayChecked = (await api("screenshots")).some((s) =>
        s.name.includes("夜航调饮室"),
      );
      if (!ownedPlayChecked) await new Promise((r) => setTimeout(r, 250));
    }
    assert.ok(ownedPlayChecked, "Play action must show a visible game window");
  }
  if (process.env.BEAVER_EXPORTED_GAME) {
    game = spawn(process.env.BEAVER_EXPORTED_GAME, [], {
      windowsHide: false,
      stdio: "pipe",
    });
    let gameLog = "";
    game.stdout.on("data", (b) => {
      gameLog += b.toString();
    });
    game.stderr.on("data", (b) => {
      gameLog += b.toString();
    });
    await new Promise((resolve, reject) => {
      game.once("spawn", resolve);
      game.once("error", reject);
    });
    let sources = [];
    const deadline = Date.now() + 15000;
    while (
      !sources.some((s) => s.name.includes("夜航调饮室")) &&
      Date.now() < deadline
    ) {
      sources = await api("screenshots");
      await new Promise((r) => setTimeout(r, 300));
    }
    const source =
      sources.find((s) => s.name === "夜航调饮室") ??
      sources.find((s) => s.name.includes("夜航调饮室"));
    await fs.writeFile(path.join(output, "standalone-window.log"), gameLog);
    assert.ok(
      source,
      `Standalone game window must be discoverable; exit=${game.exitCode}; ${gameLog.slice(-2000)}`,
    );
    const captured = await api("screenshot.capture", {
      id: project.id,
      source: source.id,
    });
    await fs.copyFile(
      path.join(project.path, captured),
      path.join(output, "07-standalone-game.png"),
    );
    assert.ok((await fs.stat(path.join(project.path, captured))).size > 1000);
    game.kill();
    game = undefined;
  }
  await page.getByRole("button", { name: "关闭窗口", exact: true }).click();
  assert.equal(
    await app.evaluate(({ BrowserWindow }) =>
      BrowserWindow.getAllWindows()[0].isVisible(),
    ),
    false,
  );
  assert.equal((await api("state")).tasks.length, beforeClose.tasks.length);
  await app.evaluate(({ BrowserWindow }) =>
    BrowserWindow.getAllWindows()[0].showInactive(),
  );
  await page.locator("nav").getByRole("button", { name: /任务/ }).click();
  await page.screenshot({ path: path.join(output, "06-failure-honest.png") });
  const pid = app.process().pid;
  const actualExecutable = await app.evaluate(({ app }) => app.getPath("exe"));
  assert.equal(
    path.resolve(actualExecutable).toLowerCase(),
    path.resolve(executablePath).toLowerCase(),
  );
  await app.close();
  app = await _electron.launch({ executablePath, args, env, timeout: 30000 });
  const restored = await app.firstWindow();
  await restored
    .getByRole("heading", { name: "夜航：失物招领", exact: true })
    .waitFor();
  assert.equal(
    await restored.getByRole("switch", { name: "编辑游戏设定" }).isChecked(),
    false,
  );
  const result = await restored.evaluate(() => window.beaver.call("state"));
  assert.equal(result.projects[0].id, project.id);
  assert.equal(result.projects[0].name, "夜航：失物招领");
  assert.equal(result.tasks[0].id, noProvider.id);
  assert.deepEqual(result.projects[0].design, expectedDesign);
  assert.deepEqual(result.projects[0].blueprint, blueprintProof.blueprint);
  await restored
    .locator(".rail-footer")
    .getByRole("button", { name: "设置", exact: true })
    .click();
  await restored
    .getByRole("button", { name: "从本机 Codex 补齐", exact: true })
    .click();
  const importedSettings = await waitState(
    restored,
    (state) => state.settings.local.code.hasKey,
  );
  for (const slot of ["code", "review", "translation"]) {
    assert.equal(
      importedSettings.settings.local[slot].baseUrl,
      "http://127.0.0.1:1/v1",
    );
    assert.equal(
      importedSettings.settings.local[slot].model,
      "beaver-fixture-model",
    );
    assert.equal(importedSettings.settings.local[slot].hasKey, true);
  }
  for (const slot of ["image", "speech", "music"])
    assert.equal(importedSettings.settings.local[slot].hasKey, false);
  assert.ok(!JSON.stringify(importedSettings).includes(fixtureKey));
  assert.ok(
    !(await restored.locator("body").textContent()).includes(fixtureKey),
  );
  await restored.locator("summary").filter({ hasText: "Codex 编程" }).click();
  await restored.screenshot({
    path: path.join(output, "08-local-api-import.png"),
  });
  assert.deepEqual(errors, []);
  await fs.writeFile(
    path.join(output, "desktop-proof.json"),
    JSON.stringify(
      {
        executablePath,
        actualExecutable,
        pid,
        project: project.path,
        detected,
        metrics,
        toastMetrics,
        initialBounds,
        initialContent,
        brand,
        executableIcon,
        railAnchors,
        blueprintProof,
        checks: [
          "real Electron launch",
          "approved brand SVG loaded at 28px with matching favicon",
          ...(executableIcon
            ? [
                "Windows native EXE, tray ICO and live window icons contain approved green and yellow",
              ]
            : []),
          "template creation via UI",
          "64 genres, planned feature selection does not run AI on creation",
          ...blueprintProof.checks,
          "legacy executable project direction remains separate from UI planning",
          "10 feature blocks with category, search and project recommendation filters",
          "feature integration action queues real source, upstream diff and fresh captured project design",
          "explicit local Codex import fills text providers without exposing keys or configuring media",
          "existing import deduplicated",
          "assets",
          "features",
          "tool detection",
          "compact UI without explanatory headings",
          "unified top-right notifications, no layout shift, no overlap with window controls",
          "notification deduplication, stacking, focus pauses expiry, errors require dismissal",
          "Loom 1120x760 window, 186/52 rail, 50 titlebar, 36 navigation and bottom-left settings",
          "custom maximize, restore, minimize and close-to-tray controls",
          "window IPC command allowlist and sender isolation",
          "rail collapse and refresh preserve task drafts and execution options",
          "sidebar toggle, navigation and settings icon positions stay fixed across collapse, expand and minimum window",
          "settings back returns to previous workspace",
          "accordion keeps unsaved fields",
          "dialog focus and Escape restoration",
          "keyboard asset preview and visible text read error",
          "task and asset layouts at 960x680 minimum window",
          "settings at minimum window retain visible save and bottom-left settings",
          ...(process.env.BEAVER_EXPECT_DESKTOP_TOOLS === "1"
            ? [
                "real Godot and Blender autodetected without overrides",
                "unsaved tool path is actually validated",
              ]
            : []),
          "settings persistence",
          "missing AI fails honestly",
          "window close hides without exit",
          "full exit and restart retains tasks",
        ],
        errors,
      },
      null,
      2,
    ),
  );
  console.log(JSON.stringify({ success: true, output, executablePath }));
} finally {
  game?.kill();
  if (app) await app.close();
}
