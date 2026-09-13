import fs from "node:fs/promises";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import assert from "node:assert/strict";
import {
  inventory,
  MAX_PAYLOAD_BYTES,
  verifyNativeRelease,
} from "./native-release";

type Package = {
  name: string;
  version: string;
  source?: string;
  manifest_path: string;
  license?: string;
  license_file?: string;
  repository?: string;
};
async function main() {
  const root = path.resolve(__dirname, "..");
  assert.ok(
    process.platform === "win32" && process.arch === "x64",
    "Windows x64 packaging only",
  );
  const releaseRoot = await fs.realpath(path.join(root, "release"));
  assert.equal(
    releaseRoot.toLowerCase(),
    path.join(await fs.realpath(root), "release").toLowerCase(),
    "Release root must not be redirected",
  );
  const { version } = JSON.parse(
    await fs.readFile(path.join(root, "package.json"), "utf8"),
  ) as { version: string };
  const name =
    process.argv[2] ??
    `Beaver-native-${version}-preview-${Date.now()}-win32-x64`;
  assert.match(
    name,
    /^[A-Za-z0-9][A-Za-z0-9._-]+$/,
    "Use a release directory name, not a path",
  );
  assert.ok(name !== "." && name !== "..");
  const output = path.join(releaseRoot, name);
  await fs.mkdir(output); // Refuse existing releases; never delete a failed candidate.
  const metadata = JSON.parse(
    execFileSync(
      "rtk",
      [
        "proxy",
        "cargo",
        "metadata",
        "--locked",
        "--offline",
        "--format-version",
        "1",
        "--filter-platform",
        "x86_64-pc-windows-msvc",
      ],
      { cwd: root, encoding: "utf8", maxBuffer: 32 * 1024 * 1024 },
    ),
  ) as { packages: Package[] };
  const notices: {
    name: string;
    version: string;
    license: string;
    source: string;
    files: string[];
  }[] = [];
  // Reuse only an integrity-verified release with identical package provenance.
  const licenseCache = process.argv[3]
    ? path.resolve(process.argv[3])
    : undefined;
  let cachedNotices: typeof notices = [];
  if (licenseCache) {
    await verifyNativeRelease(licenseCache);
    cachedNotices = JSON.parse(
      await fs.readFile(path.join(licenseCache, "THIRD-PARTY.json"), "utf8"),
    ).packages;
    assert.ok(Array.isArray(cachedNotices), "Invalid cached license inventory");
  }
  const remote = new Map<string, Promise<Buffer | null>>();
  async function download(url: string) {
    if (!remote.has(url))
      remote.set(
        url,
        (async () => {
          const response = await fetch(url, {
            signal: AbortSignal.timeout(30_000),
            redirect: "error",
          });
          if (response.status === 404) return null;
          assert.ok(
            response.ok,
            `License download failed: ${response.status} ${url}`,
          );
          const data = Buffer.from(await response.arrayBuffer());
          assert.ok(
            data.length > 0 && data.length < 1_000_000,
            "Invalid license size",
          );
          return data;
        })(),
      );
    return remote.get(url)!;
  }
  async function collect(pkg: Package, ecosystem: string) {
    const directory = path.dirname(pkg.manifest_path);
    const destination = `licenses/${ecosystem}/${pkg.name}-${pkg.version}`;
    const files: string[] = [];
    async function save(relative: string, data: Buffer) {
      const target = `${destination}/${relative}`;
      await fs.mkdir(path.dirname(path.join(output, target)), {
        recursive: true,
      });
      await fs.writeFile(path.join(output, target), data, { flag: "wx" });
      files.push(target);
    }
    for (const entry of await fs.readdir(directory, { withFileTypes: true })) {
      if (
        entry.isFile() &&
        /^(licen[sc]e|copying|notice)([._-]|$)/i.test(entry.name)
      )
        await save(
          entry.name,
          await fs.readFile(path.join(directory, entry.name)),
        );
    }
    if (!files.length && pkg.license_file) {
      const source = path.resolve(directory, pkg.license_file);
      assert.ok(
        source.startsWith(directory + path.sep),
        "License file outside package",
      );
      await save(path.basename(source), await fs.readFile(source));
    }
    let source = pkg.repository ?? pkg.source ?? "npm";
    if (!files.length && ecosystem === "rust") {
      const vcs = JSON.parse(
        await fs.readFile(path.join(directory, ".cargo_vcs_info.json"), "utf8"),
      ) as { git: { sha1: string } };
      assert.match(vcs.git.sha1, /^[a-f0-9]{40}$/);
      const repo = /^https:\/\/github.com\/([^/]+\/[^/]+?)\/?$/
        .exec(source)?.[1]
        ?.replace(/\.git$/, "");
      assert.ok(repo, `Cannot locate pinned upstream license for ${pkg.name}`);
      source = `https://github.com/${repo}/tree/${vcs.git.sha1}`;
      const cached = cachedNotices.find(
        (entry) =>
          entry.name === `${ecosystem}:${pkg.name}` &&
          entry.version === pkg.version &&
          entry.license === pkg.license &&
          entry.source === source,
      );
      if (cached && licenseCache) {
        for (const relative of cached.files) {
          assert.ok(
            relative.startsWith(`${destination}/`) && !relative.includes(".."),
            "Invalid cached license path",
          );
          await save(
            path.basename(relative),
            await fs.readFile(path.join(licenseCache, relative)),
          );
        }
      }
      for (const filename of files.length
        ? []
        : [
            "LICENSE",
            "LICENSE-MIT",
            "LICENSE-APACHE",
            "LICENSE.md",
            "LICENSE-MIT.txt",
            "LICENSE-APACHE.txt",
            "COPYING",
            "COPYRIGHT.md",
          ]) {
        const data = await download(
          `https://raw.githubusercontent.com/${repo}/${vcs.git.sha1}/${filename}`,
        );
        if (data) await save(filename, data);
      }
      if (
        !files.length &&
        pkg.name === "selectors" &&
        pkg.license === "MPL-2.0"
      ) {
        const license = await download(
          "https://www.mozilla.org/media/MPL/2.0/index.815ca599c9df.txt",
        );
        assert.ok(license);
        await save("LICENSE-MPL-2.0.txt", license);
        await save(
          "SOURCE.txt",
          Buffer.from(
            `Unmodified source: ${source}/selectors\nLicense: https://mozilla.org/MPL/2.0/\n`,
          ),
        );
      }
    }
    assert.ok(files.length, `No license text for ${pkg.name}`);
    assert.ok(pkg.license, `No declared license for ${pkg.name}`);
    notices.push({
      name: `${ecosystem}:${pkg.name}`,
      version: pkg.version,
      license: pkg.license,
      source,
      files: files.sort(),
    });
  }
  // Conservative superset includes build dependencies, not only linked runtime crates.
  for (const pkg of metadata.packages
    .filter((pkg) => pkg.source)
    .sort((a, b) =>
      `${a.name}-${a.version}`.localeCompare(`${b.name}-${b.version}`),
    ))
    await collect(pkg, "rust");
  for (const name of [
    "react",
    "react-dom",
    "scheduler",
    "smol-toml",
    "three",
    "zod",
  ]) {
    const manifest = path.join(root, "node_modules", name, "package.json");
    const pkg = JSON.parse(await fs.readFile(manifest, "utf8"));
    await collect(
      {
        name,
        version: pkg.version,
        license: pkg.license,
        manifest_path: manifest,
        source: `https://www.npmjs.com/package/${name}/v/${pkg.version}`,
      },
      "npm",
    );
  }
  await fs.copyFile(
    path.join(root, "target/release/Beaver.exe"),
    path.join(output, "Beaver.exe"),
    fs.constants.COPYFILE_EXCL,
  );
  const sysroot = execFileSync(
    "rtk",
    ["proxy", "rustc", "--print", "sysroot"],
    { encoding: "utf8" },
  ).trim();
  await fs.cp(
    path.join(sysroot, "share/doc/rust/licenses"),
    path.join(output, "licenses/rust-standard-library/licenses"),
    { recursive: true, errorOnExist: true, force: false },
  );
  await fs.copyFile(
    path.join(sysroot, "share/doc/rust/COPYRIGHT-library.html"),
    path.join(output, "licenses/rust-standard-library/COPYRIGHT-library.html"),
    fs.constants.COPYFILE_EXCL,
  );
  await fs.copyFile(
    path.join(root, "scripts/Start-Beaver.ps1"),
    path.join(output, "Start-Beaver.ps1"),
    fs.constants.COPYFILE_EXCL,
  );
  await fs.copyFile(
    path.join(root, "docs/NATIVE-RELEASE.txt"),
    path.join(output, "README.txt"),
    fs.constants.COPYFILE_EXCL,
  );
  await fs.writeFile(
    path.join(output, "THIRD-PARTY.json"),
    JSON.stringify(
      {
        scope:
          "Conservative Windows Cargo dependency and bundled frontend license inventory; includes build dependencies. Upstream license text is unmodified.",
        packages: notices,
      },
      null,
      2,
    ),
  );
  const hash = async (name: string) =>
    createHash("sha256")
      .update(await fs.readFile(path.join(root, name)))
      .digest("hex");
  await fs.writeFile(
    path.join(output, "RELEASE.json"),
    JSON.stringify(
      {
        format: "beaver-native-release-v1",
        entry: "Beaver.exe",
        platform: "win32-x64",
        channel: "migration-preview",
        createdAt: new Date().toISOString(),
        signed: false,
        runtimeVerified: false,
        migrationComplete: false,
        maxPayloadBytes: MAX_PAYLOAD_BYTES,
        externalRuntime:
          "Microsoft WebView2 Evergreen and Microsoft Visual C++ v14 x64; manually installed if missing",
        locks: {
          cargo: await hash("Cargo.lock"),
          npm: await hash("package-lock.json"),
        },
        files: await inventory(output),
      },
      null,
      2,
    ),
  );
  console.log(JSON.stringify(await verifyNativeRelease(output), null, 2));
}
main().catch((error: unknown) => {
  console.error(error);
  process.exitCode = 1;
});
