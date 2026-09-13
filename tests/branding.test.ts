import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { Data } from "resedit";
import {
  buildBrandAssets,
  iconSizes,
  renderBrandIcon,
} from "../scripts/lib/branding";

const master = () => readFile("resources/branding/beaver.svg");

test("brand master is exactly the approved round 04 talk-to-play icon", async () => {
  const svg = await master();
  assert.deepEqual(
    svg,
    await readFile("docs/ui/icon-rounds/round-04/01-talk-to-play.svg"),
  );
  assert.notEqual(svg.subarray(0, 3).toString("hex"), "efbbbf");
});

test("brand raster keeps transparent cutouts and approved colors at every size", async () => {
  const svg = await master();
  for (const size of iconSizes) {
    const rendered = renderBrandIcon(svg, size);
    assert.equal(rendered.width, size);
    assert.equal(rendered.height, size);
    const pixels = rendered.pixels;
    const colors = new Set<string>();
    for (let i = 0; i < pixels.length; i += 4)
      colors.add(pixels.subarray(i, i + 4).toString("hex"));
    assert.ok(colors.has("22c55eff"), `${size}px green`);
    assert.ok(colors.has("d9ff38ff"), `${size}px yellow`);
    assert.equal(pixels[3], 0, `${size}px transparent corner`);
  }
  const pixels = renderBrandIcon(svg, 128).pixels;
  const pixel = (x: number, y: number) => [
    ...pixels.subarray((y * 128 + x) * 4, (y * 128 + x) * 4 + 4),
  ];
  assert.equal(pixel(40, 63)[3], 0, "d-pad is a cutout");
  assert.deepEqual(pixel(85, 51), [217, 255, 56, 255]);
  assert.deepEqual(pixel(98, 72), [217, 255, 56, 255]);
  assert.deepEqual(pixel(64, 40), [34, 197, 94, 255]);
});

test("brand ICO round-trips all PNG frames without changing their bytes", async () => {
  const svg = await master();
  const assets = buildBrandAssets(svg);
  assert.equal(assets.size, iconSizes.length + 1);
  assert.deepEqual(assets, buildBrandAssets(svg), "deterministic generation");
  const ico = assets.get("beaver.ico");
  assert.ok(ico);
  const parsed = Data.IconFile.from(ico);
  assert.equal(
    parsed.icons[0]?.data.width,
    256,
    "Tauri decodes the first frame for the taskbar icon",
  );
  assert.equal(parsed.icons.length, iconSizes.length);
  for (const [index, frame] of parsed.icons.entries()) {
    const size = iconSizes[index];
    assert.ok(frame.data.isRaw());
    assert.equal(frame.data.width, size);
    assert.equal(frame.data.height, size);
    assert.equal(frame.data.bitCount, 32);
    assert.deepEqual(
      Buffer.from(frame.data.bin),
      assets.get(`beaver-${size}.png`),
    );
  }
});

test("brand generation rejects a non-square master", () => {
  assert.throws(
    () =>
      buildBrandAssets(
        '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 128 64"/>',
      ),
    /must be square/,
  );
});
