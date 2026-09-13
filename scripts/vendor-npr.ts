import fs from "node:fs/promises";
import path from "node:path";
import { createHash } from "node:crypto";

// Explicit, copy-only package refresh. Never read the source project's assets/cache.
async function main() {
  const source = path.resolve(
    process.argv[2] ?? "../RoleNPR/addons/npr_characters",
  );
  const target = path.resolve(
    "resources/packages/npr-characters/addons/npr_characters",
  );
  const contract = JSON.parse(
    await fs.readFile(path.join(source, "asset_contract.json"), "utf8"),
  ) as { contract: string; version: string };
  if (contract.contract !== "npr-characters")
    throw new Error("Unexpected NPR source");
  const files: Record<string, string> = {};
  async function copy(directory: string, relative = "") {
    for (const entry of await fs.readdir(directory, { withFileTypes: true })) {
      const name = relative + entry.name;
      if (entry.isDirectory())
        await copy(path.join(directory, entry.name), name + "/");
      else if (entry.isFile()) {
        const bytes = await fs.readFile(path.join(directory, entry.name));
        const destination = path.join(target, name);
        await fs.mkdir(path.dirname(destination), { recursive: true });
        await fs.writeFile(destination, bytes);
        files[name] = createHash("sha256").update(bytes).digest("hex");
      } else
        throw new Error("NPR package contains a link or unsupported entry");
    }
  }
  await copy(source);
  await fs.writeFile(
    "resources/packages/npr-characters/provenance.json",
    JSON.stringify(
      {
        package: contract.contract,
        version: contract.version,
        source: "RoleNPR/addons/npr_characters",
        files,
      },
      null,
      2,
    ) + "\n",
    "utf8",
  );
  console.log(
    `Vendored ${Object.keys(files).length} NPR files; source unchanged.`,
  );
}
void main();
