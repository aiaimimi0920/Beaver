import fs from "node:fs/promises";
import path from "node:path";

export async function managedCodex(
  prefix: string,
  architecture = process.arch,
): Promise<string | undefined> {
  const triple =
    architecture === "arm64"
      ? "aarch64-pc-windows-msvc"
      : "x86_64-pc-windows-msvc";
  const packageName = `codex-win32-${architecture}`;
  const roots = [
    path.join(prefix, "node_modules/@openai", packageName, "vendor", triple),
    path.join(
      prefix,
      "node_modules/@openai/codex/node_modules/@openai",
      packageName,
      "vendor",
      triple,
    ),
    path.join(prefix, "node_modules/@openai/codex/vendor", triple),
  ];
  for (const root of roots)
    for (const folder of ["bin", "codex"]) {
      const file = path.join(root, folder, "codex.exe");
      if ((await fs.stat(file).catch(() => undefined))?.isFile()) return file;
    }
  return undefined;
}
