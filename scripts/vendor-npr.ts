import fs from "node:fs/promises";
import path from "node:path";
import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";

export const NPR_COMMIT = "a08b47a6cc229b6978afda26d74d13af690a67ba";
export const NPR_SOURCE_PATH = "addons/npr_character_frame";
const REPOSITORY = "https://github.com/aiaimimi0920/NPRCharacterFrame";
const SAMPLE_LAUNCHERS = new Set([
  "npr_character_preview",
  "npr_lab",
  "npr_multiview_lab",
  "npr_standalone",
]);

type SourceEntry = { path: string; git_blob_oid: string; git_mode: string };
type SourceManifest = {
  repositories: {
    repository_url: string;
    commit: string;
    files: SourceEntry[];
  }[];
};

export function includedNprFile(relative: string): boolean {
  if (relative.startsWith("samples/")) return false;
  if (relative.startsWith(".ci_script/tools/hosiery_surface_probe.gd"))
    return false;
  if (relative.startsWith(".ci_script/"))
    return (
      relative === ".ci_script/model/check_model.gd" ||
      relative.startsWith(".ci_script/tools/") ||
      relative.startsWith(".ci_script/ai_model/")
    );
  if (relative === "showcase/wardrobe.tscn") return false;
  if (relative.startsWith("showcase/")) {
    const stem = path.posix.basename(relative).split(".")[0];
    if (SAMPLE_LAUNCHERS.has(stem ?? "")) return false;
  }
  return !relative.split("/").some((part) => part.startsWith("."));
}

async function sourceEntries(
  source: string,
  manifest?: string,
): Promise<SourceEntry[]> {
  if (manifest) {
    const data = JSON.parse(
      await fs.readFile(manifest, "utf8"),
    ) as SourceManifest;
    const repo = data.repositories.find(
      (entry) => entry.repository_url === REPOSITORY,
    );
    if (repo?.commit !== NPR_COMMIT)
      throw new Error("Unexpected NPR archive commit");
    return repo.files;
  }
  const commit = execFileSync("git", ["rev-parse", "HEAD"], {
    cwd: source,
    encoding: "utf8",
  }).trim();
  if (commit !== NPR_COMMIT)
    throw new Error("NPR source must be the pinned upstream commit");
  const tree = execFileSync(
    "git",
    ["ls-tree", "-rz", NPR_COMMIT, "--", NPR_SOURCE_PATH],
    {
      cwd: source,
      encoding: "utf8",
    },
  );
  return tree
    .split("\0")
    .filter(Boolean)
    .map((line) => {
      const [metadata, name] = line.split("\t");
      const [mode, type, oid] = (metadata ?? "").split(" ");
      if (type !== "blob" || !name || !mode || !oid)
        throw new Error("Unexpected NPR Git entry");
      return { path: name, git_blob_oid: oid, git_mode: mode };
    });
}

// Only verified tracked bytes are copied. Neither an archive label nor a local
// snapshot commit is treated as the upstream revision; verify every Git blob.
export async function vendorNpr(
  source: string,
  target: string,
  manifest?: string,
) {
  const entries = await sourceEntries(source, manifest);
  const selected = entries
    .filter(
      (entry) =>
        entry.path.startsWith(NPR_SOURCE_PATH + "/") &&
        includedNprFile(entry.path.slice(NPR_SOURCE_PATH.length + 1)),
    )
    .sort((a, b) => a.path.localeCompare(b.path, "en"));
  const bytesByName = new Map<string, Buffer>();
  const files: Record<string, string> = {};
  const gitBlobs: Record<string, string> = {};
  for (const entry of selected) {
    if (!/^100(644|755)$/.test(entry.git_mode) || entry.path.includes(".."))
      throw new Error(
        "NPR package contains an unsafe path or unsupported Git mode",
      );
    const file = path.join(source, entry.path);
    if (!(await fs.lstat(file)).isFile())
      throw new Error("NPR package file is not regular");
    const bytes = await fs.readFile(file);
    const blob = createHash("sha1")
      .update(`blob ${bytes.length}\0`)
      .update(bytes)
      .digest("hex");
    if (blob !== entry.git_blob_oid)
      throw new Error(`NPR source differs from pinned blob: ${entry.path}`);
    const relative = entry.path.slice(NPR_SOURCE_PATH.length + 1);
    bytesByName.set(relative, bytes);
    files[relative] = createHash("sha256").update(bytes).digest("hex");
    gitBlobs[relative] = blob;
  }
  const contract = JSON.parse(
    bytesByName.get("asset_contract.json")?.toString() ?? "null",
  );
  const pluginVersion = bytesByName
    .get("plugin.cfg")
    ?.toString()
    .match(/^version="([^"]+)"/m)?.[1];
  if (
    contract?.contract !== "npr-character-frame" ||
    contract.version !== "1.1.0" ||
    pluginVersion !== "1.3.0"
  )
    throw new Error("Unexpected NPR contract or plugin version");
  for (const required of [
    ".ci_script/model/check_model.gd",
    "docs/model_authoring/README.md",
    ...["geometry", "textures", "feature_data"].map(
      (name) => `docs/model_authoring/prompts/${name}.md`,
    ),
  ]) {
    if (!bytesByName.has(required))
      throw new Error(`Missing required NPR file: ${required}`);
  }
  // Verification finishes before replacing the previous generated package.
  await fs.rm(target, { recursive: true, force: true });
  for (const [relative, bytes] of bytesByName) {
    const destination = path.join(target, NPR_SOURCE_PATH, relative);
    await fs.mkdir(path.dirname(destination), { recursive: true });
    await fs.writeFile(destination, bytes);
  }
  const provenance = {
    package: "npr-characters",
    contract: contract.contract,
    contractVersion: contract.version,
    pluginVersion,
    source: REPOSITORY,
    sourceCommit: NPR_COMMIT,
    sourcePath: NPR_SOURCE_PATH,
    exclusions: [
      "samples/**",
      "sample-only showcase launchers and wardrobe.tscn",
      ".ci_script/tools/hosiery_surface_probe.gd and UID (sample-only collector)",
      ".ci_script/framework/**",
      ".ci_script/build/**",
      ".ci_script/model/capture_views.gd and cases.json",
    ],
    verification:
      "Every included file verified against its pinned Git blob; SHA-256 covers exact installed bytes",
    files,
    gitBlobs,
  };
  await fs.writeFile(
    path.join(target, "provenance.json"),
    JSON.stringify(provenance, null, 2) + "\n",
  );
  return provenance;
}

async function main() {
  const result = await vendorNpr(
    path.resolve(process.argv[2] ?? "../NPRCharacterFrame"),
    path.resolve("resources/packages/npr-characters"),
    process.argv[3],
  );
  console.log(
    `Vendored ${Object.keys(result.files).length} verified NPR files; source unchanged.`,
  );
}

if (
  process.argv[1] &&
  /(?:^|[\\/])vendor-npr\.(?:ts|js)$/.test(process.argv[1])
) {
  void main().catch((error: unknown) => {
    console.error(error);
    process.exitCode = 1;
  });
}
