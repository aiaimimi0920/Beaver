import { readFile, writeFile, rename, mkdir, chmod } from "node:fs/promises";
import { dirname } from "node:path";

export async function openStore(path) {
  let data = { tasks: {}, subscriptions: {} };
  try {
    data = JSON.parse(await readFile(path, "utf8"));
  } catch (error) {
    if (error.code !== "ENOENT") throw error;
  }
  return {
    data,
    async save() {
      await mkdir(dirname(path), { recursive: true, mode: 0o700 });
      await writeFile(path + ".tmp", JSON.stringify(data), { mode: 0o600 });
      await chmod(path + ".tmp", 0o600);
      await rename(path + ".tmp", path);
    },
  };
}
