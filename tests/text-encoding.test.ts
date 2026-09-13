import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { decodeResourceText } from "../src/core/text-encoding";
import { normalizeSources } from "../src/core/source-encoding";
import { recoverableFailure } from "../src/shared/recovery";
test("BOM-marked Unicode logs decode losslessly; unknown bytes are rejected", () => {
  const text = "日志：完成\n";
  assert.equal(
    decodeResourceText(Buffer.from("\ufeff" + text, "utf16le")),
    text,
  );
  const be = Buffer.from("\ufeff" + text, "utf16le");
  be.swap16();
  assert.equal(decodeResourceText(be), text);
  assert.equal(decodeResourceText(Buffer.from("\ufeff" + text)), text);
  assert.throws(() => decodeResourceText(Uint8Array.from([255, 254, 65])));
  assert.throws(() => decodeResourceText(Uint8Array.from([255, 129])));
  assert.equal(recoverableFailure("stream disconnected"), true);
  assert.equal(
    recoverableFailure("401 unauthorized: connection closed"),
    false,
  );
  assert.equal(recoverableFailure("request timed out"), false);
});
test("normalize changed sources and preserve binary or unrecognized data", async () => {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), "beaver-encoding-"));
  try {
    await fs.writeFile(path.join(root, "main.tscn"), "\ufeff[gd_scene]\n");
    await fs.writeFile(
      path.join(root, "sound.wav"),
      Buffer.from([255, 254, 0]),
    );
    assert.deepEqual(await normalizeSources(root, {}), ["main.tscn"]);
    assert.equal(
      await fs.readFile(path.join(root, "main.tscn"), "utf8"),
      "[gd_scene]\n",
    );
    assert.deepEqual(
      await fs.readFile(path.join(root, "sound.wav")),
      Buffer.from([255, 254, 0]),
    );
    await fs.writeFile(path.join(root, "bad.gd"), Buffer.from([255]));
    await assert.rejects(normalizeSources(root, {}));
    assert.deepEqual(
      await fs.readFile(path.join(root, "bad.gd")),
      Buffer.from([255]),
    );
  } finally {
    await fs.rm(root, { recursive: true, force: true });
  }
});
