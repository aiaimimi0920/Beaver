import { readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { buildBrandAssets } from "./lib/branding";

async function main() {
  const root = path.resolve("resources/branding");
  const assets = buildBrandAssets(
    await readFile(path.join(root, "beaver.svg")),
  );
  for (const [name, bytes] of assets)
    await writeFile(path.join(root, name), bytes);
  console.log(`Beaver branding: ${assets.size} assets from the approved SVG`);
}

void main().catch((error: unknown) => {
  console.error(error);
  process.exitCode = 1;
});
