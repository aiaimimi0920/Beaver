import { build } from "esbuild";
import { z } from "zod";
import { overviewSchema } from "../src/shared/project-overview";
import { directionSchema } from "../src/shared/task-board";
import { objectTaskIdentitySchema } from "../src/shared/object-framework";
import fs from "node:fs/promises";
import path from "node:path";
import { defaultSettings } from "../src/shared/types";
import { askUserTool } from "../src/shared/clarifications";
import {
  nativePlanTool as planTool,
  nativePlanningInstruction as planningInstruction,
} from "../src/shared/task-plan";
import { buildBrandAssets } from "./lib/branding";
import { readProductVersion } from "./product-version";
import { windowsToolHintsScript } from "../src/core/tool-discovery";
import {
  genres,
  themes,
  styles,
  scopes,
  featureSchema,
  gameBriefSchema,
} from "../src/shared/game-design";
import {
  genreChoices,
  themeChoices,
  audiences,
  gameSizes,
  phases,
  campaignChoices,
} from "../src/shared/blueprint-catalog";
import {
  defaultBlueprint,
  blueprintSchema,
  legacyGenreChoices,
} from "../src/shared/project-blueprint";

async function writeIfChanged(file: string, data: string | Uint8Array) {
  const content =
    typeof data === "string" ? Buffer.from(data, "utf8") : Buffer.from(data);
  try {
    if ((await fs.readFile(file)).equals(content)) return;
  } catch (error: unknown) {
    if (!(error instanceof Error && "code" in error && error.code === "ENOENT"))
      throw error;
  }
  await fs.mkdir(path.dirname(file), { recursive: true });
  await fs.writeFile(file, content);
}

async function encodeTree(directory: string): Promise<Record<string, string>> {
  const files: Record<string, string> = {};
  async function collect(current: string, prefix = "") {
    for (const entry of await fs.readdir(current, { withFileTypes: true })) {
      const relative = prefix + entry.name;
      if (entry.isDirectory())
        await collect(path.join(current, entry.name), relative + "/");
      else if (entry.isFile())
        files[relative] = (
          await fs.readFile(path.join(current, entry.name))
        ).toString("base64");
      else throw new Error("Embedded resource contains unsupported file type");
    }
  }
  await collect(directory);
  return files;
}

async function main() {
  const version = await readProductVersion();
  const brand = await fs.readFile("resources/branding/beaver.svg");
  for (const [name, data] of buildBrandAssets(brand))
    await writeIfChanged(path.join("resources/branding", name), data);
  await fs.mkdir("dist-native", { recursive: true });
  await writeIfChanged("dist-native/plan-tool.json", JSON.stringify(planTool));
  await writeIfChanged(
    "dist-native/planning-instruction.txt",
    planningInstruction,
  );
  await writeIfChanged(
    "dist-native/business-schemas.json",
    JSON.stringify({
      design: z.toJSONSchema(gameBriefSchema),
      blueprint: z.toJSONSchema(blueprintSchema),
      overview: z.toJSONSchema(overviewSchema),
      direction: z.toJSONSchema(directionSchema),
      objectFramework: {
        type: "object",
        ...z.toJSONSchema(objectTaskIdentitySchema),
      },
    }),
  );
  await writeIfChanged(
    "dist-native/tool-discovery-windows.ps1",
    windowsToolHintsScript,
  );
  const bundle = await build({
    entryPoints: ["src/ui/main.tsx"],
    outfile: "dist-native/ui.js",
    bundle: true,
    platform: "browser",
    format: "esm",
    minify: true,
    sourcemap: false,
    loader: { ".woff2": "file" },
    write: false,
  });
  for (const file of bundle.outputFiles)
    await writeIfChanged(file.path, file.contents);
  const html = await fs.readFile("src/ui/index.html", "utf8");
  await writeIfChanged(
    "dist-native/index.html",
    html.replace(
      /\s*<meta\s+http-equiv="Content-Security-Policy"[\s\S]*?\/>/,
      "",
    ),
  );
  await writeIfChanged("dist-native/beaver.svg", brand);
  await writeIfChanged(
    "dist-native/default-settings.json",
    JSON.stringify(defaultSettings()),
  );
  const features: unknown[] = [];
  const featureSources: Record<string, Record<string, string>> = {};
  await writeIfChanged(
    "dist-native/ask-user-tool.json",
    JSON.stringify(askUserTool),
  );
  for (const directory of await fs.readdir("resources/features")) {
    const manifest = featureSchema.parse(
      JSON.parse(
        await fs.readFile(
          path.join("resources/features", directory, "feature.json"),
          "utf8",
        ),
      ),
    );
    if (manifest.id !== directory)
      throw new Error("Feature directory and manifest disagree");
    features.push(manifest);
    featureSources[directory] = await encodeTree(
      path.join("resources/features", directory),
    );
  }
  await writeIfChanged("dist-native/features.json", JSON.stringify(features));
  await writeIfChanged(
    "dist-native/feature-sources.json",
    JSON.stringify(featureSources),
  );
  const templates: Record<string, Record<string, string>> = {};
  for (const name of ["blank", "nightbar"]) {
    templates[name] = await encodeTree(path.join("resources/templates", name));
  }
  await writeIfChanged("dist-native/templates.json", JSON.stringify(templates));
  await writeIfChanged(
    "dist-native/npr-package.json",
    JSON.stringify(await encodeTree("resources/packages/npr-characters")),
  );
  await writeIfChanged(
    "dist-native/blueprint-catalog.json",
    JSON.stringify({
      genres: [...genreChoices, ...legacyGenreChoices],
      briefGenres: genreChoices,
      themes: themeChoices,
      audiences,
      sizes: gameSizes,
      styles,
      phases,
      campaigns: campaignChoices,
      default: defaultBlueprint(),
      legacy: { genres, themes, styles, scopes },
    }),
  );
  await writeIfChanged(
    "dist-native/design-catalog.json",
    JSON.stringify({
      genres: genres.map((v) => v.id),
      themes: themes.map((v) => v.id),
      styles: styles.map((v) => v.id),
      scopes: scopes.map((v) => v.id),
    }),
  );
  await writeIfChanged("dist-native/build.json", JSON.stringify(version));
  console.log(
    "Native frontend built without Electron or production source maps",
  );
}
void main();
