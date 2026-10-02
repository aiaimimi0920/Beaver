import fs from "node:fs/promises";
import path from "node:path";
import { listFiles, safePath } from "./files";
import { bundlePreset, type GodotBundle } from "./godot-bundle";

export async function prepareBundleWorkspace(
  project: string,
  workspace: string,
  bundle: GodotBundle,
  preset: string,
): Promise<void> {
  for (const relative of await listFiles(project)) {
    const target = await safePath(workspace, relative);
    await fs.mkdir(path.dirname(target), { recursive: true });
    await fs.copyFile(
      await safePath(project, relative),
      target,
      fs.constants.COPYFILE_EXCL,
    );
  }
  const file = path.join(workspace, "export_presets.cfg");
  await fs.writeFile(
    file,
    await bundlePreset(
      bundle,
      await fs.readFile(file, "utf8"),
      preset,
      workspace,
    ),
    "utf8",
  );
}

export async function copyBundleLibraries(
  bundle: GodotBundle,
  folder: string,
): Promise<void> {
  for (const item of await fs.readdir(bundle.directory, {
    withFileTypes: true,
  })) {
    if (!/\.dll$/i.test(item.name)) continue;
    if (!item.isFile() || item.isSymbolicLink())
      throw new Error("本地引擎依赖不是普通文件");
    await fs.copyFile(
      path.join(bundle.directory, item.name),
      path.join(folder, item.name),
      fs.constants.COPYFILE_EXCL,
    );
  }
}
