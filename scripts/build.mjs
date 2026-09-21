import { build } from "esbuild";
import { mkdir, copyFile, writeFile } from "node:fs/promises";
import { readProductVersion } from "./product-version.ts";
const product = await readProductVersion();
await mkdir("dist", { recursive: true });
await build({
  entryPoints: ["src/main/main.ts"],
  outfile: "dist/main.cjs",
  bundle: true,
  platform: "node",
  format: "cjs",
  external: ["electron"],
  sourcemap: true,
});
await build({
  entryPoints: ["src/main/preload.ts"],
  outfile: "dist/preload.cjs",
  bundle: true,
  platform: "node",
  format: "cjs",
  external: ["electron"],
});
await build({
  entryPoints: ["src/mcp/server.ts"],
  outfile: "dist/mcp.cjs",
  bundle: true,
  platform: "node",
  format: "cjs",
});
await build({
  entryPoints: ["src/ui/main.tsx"],
  outfile: "dist/ui.js",
  bundle: true,
  platform: "browser",
  format: "esm",
  minify: true,
  sourcemap: true,
  loader: { ".woff2": "file" },
});
await copyFile("src/ui/index.html", "dist/index.html");
await copyFile("resources/branding/beaver.svg", "dist/beaver.svg");
await writeFile(
  "dist/build.json",
  JSON.stringify({ ...product, builtAt: new Date().toISOString() }) + "\n",
);
console.log("Beaver build complete");
