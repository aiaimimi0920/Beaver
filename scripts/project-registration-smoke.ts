import { build } from "esbuild";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { spawnSync } from "node:child_process";

async function main() {
  const output = path.resolve("output/project-registration-ui");
  await mkdir(output, { recursive: true });
  await build({
    entryPoints: ["tests/fixtures/project-registration-flow.tsx"],
    bundle: true,
    outfile: path.join(output, "fixture.js"),
    platform: "browser",
    jsx: "automatic",
  });
  await writeFile(
    path.join(output, "index.html"),
    '<div id="root"></div><script src="fixture.js"></script>',
  );
  await writeFile(
    path.join(output, "runner.cjs"),
    `
const { app, BrowserWindow } = require("electron");
app.setPath("userData", ${JSON.stringify(path.join(output, "profile"))});
app.whenReady().then(async () => {
  const window = new BrowserWindow({ show: false });
  try {
    await window.loadFile(${JSON.stringify(path.join(output, "index.html"))});
    console.log(await window.webContents.executeJavaScript("window.registrationResult"));
    app.exit(0);
  } catch (error) { console.error(error); app.exit(1); }
});
`,
  );
  const env = { ...process.env };
  delete env.ELECTRON_RUN_AS_NODE;
  const result = spawnSync(
    path.resolve("node_modules/electron/dist/electron.exe"),
    [path.join(output, "runner.cjs")],
    {
      env,
      encoding: "utf8",
      timeout: 30000,
    },
  );
  console.log(result.stdout);
  if (result.status !== 0) {
    console.error(result.stderr, result.error);
    process.exitCode = 1;
  }
}
void main().catch((error: unknown) => {
  console.error(error);
  process.exitCode = 1;
});
