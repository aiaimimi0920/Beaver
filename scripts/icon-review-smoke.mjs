import { createRequire } from "node:module";
import fs from "node:fs/promises";
import path from "node:path";
import { pathToFileURL } from "node:url";
import assert from "node:assert/strict";

if (!process.env.BEAVER_PLAYWRIGHT)
  throw new Error(
    "Set BEAVER_PLAYWRIGHT to the external playwright package directory",
  );
const { chromium } = createRequire(import.meta.url)(
  process.env.BEAVER_PLAYWRIGHT,
);
const folder = path.resolve(process.argv[2] || "docs/ui/icon-rounds/round-01");
const expectedCandidates = Number(process.argv[3] || 3);
assert.ok(
  Number.isInteger(expectedCandidates) &&
    expectedCandidates > 0 &&
    expectedCandidates <= 26,
);
const lastChoice = String.fromCharCode(64 + expectedCandidates);
const output = path.resolve(
  "output/playwright",
  `icons-${path.basename(folder)}-${Date.now()}`,
);
await fs.mkdir(output, { recursive: true });
const browser = await chromium.launch({
  headless: true,
  channel: process.env.BEAVER_BROWSER_CHANNEL || undefined,
});
const errors = [];
try {
  const page = await browser.newPage({
    viewport: { width: 1440, height: 1120 },
    deviceScaleFactor: 1,
  });
  const externalRequests = [];
  await page.route(/^https?:/, (route) => {
    externalRequests.push(route.request().url());
    return route.abort();
  });
  page.on("pageerror", (error) => errors.push(error.message));
  await page.goto(pathToFileURL(path.join(folder, "index.html")).href);
  await page.waitForFunction(() =>
    [...document.images].every(
      (image) => image.complete && image.naturalWidth > 0,
    ),
  );
  assert.equal(await page.locator(".candidate").count(), expectedCandidates);
  const marks = await page
    .locator(".candidate .hero img")
    .evaluateAll((images) => images.map((image) => image.getAttribute("src")));
  assert.equal(new Set(marks).size, expectedCandidates);
  for (const size of [16, 24, 32, 48, 64])
    assert.equal(
      await page.locator(`.candidate img[alt="${size}px"]`).count(),
      expectedCandidates,
    );
  assert.ok(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  );
  await page.screenshot({
    path: path.join(output, "01-dark.png"),
    fullPage: true,
  });
  await page.getByRole("button", { name: "浅色预览", exact: true }).click();
  await page.getByRole("button", { name: "深色预览", exact: true }).waitFor();
  await page.screenshot({
    path: path.join(output, "02-light.png"),
    fullPage: true,
  });
  await page.getByRole("button", { name: "深色预览", exact: true }).click();
  await page.getByRole("button", { name: "选择 A", exact: true }).focus();
  await page.keyboard.press("Enter");
  assert.equal(
    await page.locator(".candidate.selected").getAttribute("data-choice"),
    "A",
  );
  await page
    .getByLabel("修改意见", { exact: true })
    .fill("保留门齿，尾巴更明确");
  // Exercise the offline file:// fallback without clipboard permission assumptions.
  await page.evaluate(() =>
    Object.defineProperty(navigator, "clipboard", {
      value: undefined,
      configurable: true,
    }),
  );
  await page.getByRole("button", { name: "复制选型意见" }).click();
  assert.match(
    await page.getByLabel("可手动复制的选型意见").inputValue(),
    /选择 A/,
  );
  assert.match(
    await page.getByLabel("可手动复制的选型意见").inputValue(),
    /尾巴更明确/,
  );
  await page
    .getByRole("button", { name: `选择 ${lastChoice}`, exact: true })
    .click();
  assert.equal(await page.locator(".candidate.selected").count(), 1);
  assert.equal(
    await page.locator(".candidate.selected").getAttribute("data-choice"),
    lastChoice,
  );
  if (await page.locator("#clear").count()) {
    await page.getByRole("button", { name: "全部重想", exact: true }).click();
    assert.equal(await page.locator(".candidate.selected").count(), 0);
    assert.equal(
      await page.locator('[data-pick][aria-pressed="true"]').count(),
      0,
    );
    await page.getByRole("button", { name: "复制选型意见" }).click();
    assert.match(
      await page.getByLabel("可手动复制的选型意见").inputValue(),
      /六组均需调整/,
    );
    await page
      .getByRole("button", { name: `选择 ${lastChoice}`, exact: true })
      .click();
    assert.equal(await page.locator("#copy-fallback").isVisible(), false);
  }
  await page.setViewportSize({ width: 560, height: 900 });
  assert.ok(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  );
  await page.screenshot({
    path: path.join(output, "03-narrow.png"),
    fullPage: true,
  });
  for (const width of [360, 800, 1200, 1920]) {
    await page.setViewportSize({ width, height: 900 });
    assert.ok(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= innerWidth,
      ),
      `Horizontal overflow at ${width}px`,
    );
    const oversizedSamples = await page
      .locator(".samples")
      .evaluateAll((rows) =>
        rows.some((row) => row.scrollWidth > row.clientWidth + 1),
      );
    assert.equal(oversizedSamples, false, `Sample overflow at ${width}px`);
  }
  assert.deepEqual(externalRequests, []);
  assert.deepEqual(errors, []);
  await fs.writeFile(
    path.join(output, "proof.json"),
    JSON.stringify(
      {
        folder,
        output,
        checks: [
          `${expectedCandidates} distinct candidates`,
          "all local SVGs loaded",
          "dark and light",
          "16/24/32/48/64 size previews",
          "keyboard selection",
          "offline feedback copy fallback",
          "no external network requests",
          "360/560/800/1200/1440/1920px layout",
        ],
        errors,
      },
      null,
      2,
    ),
  );
  console.log(JSON.stringify({ success: true, output }));
} finally {
  await browser.close();
}
