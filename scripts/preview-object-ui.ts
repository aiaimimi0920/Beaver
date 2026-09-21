import { context } from "esbuild";
import { copyFile, mkdir } from "node:fs/promises";
import path from "node:path";

async function main() {
  const root = process.cwd();
  const outdir = path.join(root, "output", "object-ui-preview");
  await mkdir(outdir, { recursive: true });
  await copyFile(
    path.join(root, "src/ui/index.html"),
    path.join(outdir, "index.html"),
  );
  await copyFile(
    path.join(root, "resources/branding/beaver.svg"),
    path.join(outdir, "beaver.svg"),
  );
  const build = await context({
    entryPoints: [path.join(root, "src/ui/main.tsx")],
    outfile: path.join(outdir, "ui.js"),
    bundle: true,
    platform: "browser",
    format: "esm",
    sourcemap: true,
    define: { "process.env.NODE_ENV": '"development"' },
  });
  await build.watch();
  const server = await build.serve({
    servedir: outdir,
    host: "127.0.0.1",
    port: 4175,
  });
  console.log(
    "Beaver UI preview: http://127.0.0.1:" + server.port + "/?preview=objects",
  );
  console.log(
    "Mock data only. No backend or creative tools are started. Ctrl+C to stop.",
  );
  const stop = async () => {
    await build.dispose();
    process.exit(0);
  };
  process.once("SIGINT", stop);
  process.once("SIGTERM", stop);
}

main().catch((error: unknown) => {
  console.error(error);
  process.exit(1);
});
