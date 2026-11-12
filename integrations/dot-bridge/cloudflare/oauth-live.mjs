import { readFile, writeFile } from "node:fs/promises";
import { proveOAuth } from "./oauth-proof.mjs";
const [origin, credentials, output] = process.argv.slice(2);
if (!output)
  throw new Error(
    "Usage: oauth-live.mjs ORIGIN OWNER_CREDENTIALS_JSON NEW_PROOF_FILE",
  );
const { owner } = JSON.parse(await readFile(credentials, "utf8"));
const proof = await proveOAuth(origin, owner);
await writeFile(output, JSON.stringify({ origin, ...proof }, null, 2), {
  encoding: "utf8",
  flag: "wx",
});
console.log(JSON.stringify(proof));
