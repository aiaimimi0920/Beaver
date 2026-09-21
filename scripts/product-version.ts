import assert from "node:assert/strict";
import fs from "node:fs/promises";
import path from "node:path";
import { z } from "zod";

const component = "(?:0|[1-9][0-9]*)";
const packageVersionSchema = z.object({
  version: z
    .string()
    .regex(new RegExp(`^${component}\\.${component}\\.${component}$`))
    .refine((value) => value.split(".").every((part) => Number(part) <= 65535)),
  beaverBuild: z.number().int().min(1).max(65535),
});

export function productVersion(metadata: unknown, channel = "development") {
  assert.ok(
    channel === "development" || channel === "release",
    "BEAVER_CHANNEL must be development or release",
  );
  const { version: releaseVersion, beaverBuild: buildNumber } =
    packageVersionSchema.parse(metadata);
  return {
    version:
      channel === "development"
        ? `${releaseVersion}.${buildNumber}`
        : releaseVersion,
    releaseVersion,
    buildNumber,
    channel,
    windowsVersion: `${releaseVersion}.${channel === "development" ? buildNumber : 0}`,
  };
}

export type ProductVersion = ReturnType<typeof productVersion>;

export async function readProductVersion(
  root = process.cwd(),
  channel = process.env.BEAVER_CHANNEL,
) {
  const metadata: unknown = JSON.parse(
    await fs.readFile(path.join(root, "package.json"), "utf8"),
  );
  return productVersion(metadata, channel);
}

export function verifyProductVersion(metadata: unknown): ProductVersion {
  const manifest = z
    .object({
      version: z.string(),
      releaseVersion: z.string(),
      buildNumber: z.number(),
      channel: z.string(),
      windowsVersion: z.string(),
    })
    .parse(metadata);
  const expected = productVersion(
    { version: manifest.releaseVersion, beaverBuild: manifest.buildNumber },
    manifest.channel,
  );
  assert.deepEqual(manifest, expected, "Product version metadata disagrees");
  return expected;
}
