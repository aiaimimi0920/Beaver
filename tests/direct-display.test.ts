import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import vm from "node:vm";
import test from "node:test";

const source = readFileSync(
  new URL("../src/direct/renderer.js", import.meta.url),
  "utf8",
);
const helper = source.slice(
  source.indexOf("function displayCopy("),
  source.indexOf("const print ="),
);
const project = vm.runInNewContext(`${helper}; displayCopy`);

test("display projection bounds payloads without modifying authoritative values", () => {
  const input = {
    status: "succeeded",
    fingerprint: { code: "s".repeat(100000) },
    result: {
      content: "x".repeat(100000),
      values: Array.from({ length: 5000 }, (_, i) => i),
    },
  };
  const before = JSON.stringify(input);
  const output = project(input);
  assert.equal(output.status, "succeeded");
  assert.match(output.fingerprint, /retained/);
  assert.ok(output.result.content.length < 2100);
  assert.ok(output.result.values.length <= 61);
  assert.ok(JSON.stringify(output).length < 5000);
  assert.equal(JSON.stringify(input), before);
});

test("nested context display terminates and preserves scalar errors", () => {
  let nested: unknown = { error: "failed", success: false, count: 0 };
  for (let i = 0; i < 100; i++) nested = { child: nested };
  assert.ok(JSON.stringify(project(nested)).length < 500);
  assert.deepEqual(
    JSON.parse(
      JSON.stringify(
        project({ error: "failed", success: false, count: 0, data: null }),
      ),
    ),
    { error: "failed", success: false, count: 0, data: null },
  );
  assert.match(source, /JSON\.stringify\(displayCopy\(context\), null, 2\)/);
});

test("wide objects and oversized keys stay bounded", () => {
  const input = Object.fromEntries(
    Array.from({ length: 10000 }, (_, i) => [String(i), i]),
  );
  assert.ok(Object.keys(project(input)).length <= 60);
  const longKey = "k".repeat(100000);
  assert.ok(JSON.stringify(project({ [longKey]: "value" })).length < 300);
  assert.equal(input["9999"], 9999);
});

test("long diagnostics retain their final error", () => {
  const output = project({ stderr: "x".repeat(100000) + "FINAL ERROR" });
  assert.ok(output.stderr.length < 2100);
  assert.ok(output.stderr.endsWith("FINAL ERROR"));
});
