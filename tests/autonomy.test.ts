import test from "node:test";
import assert from "node:assert/strict";
import {
  automaticChoice,
  askRatioSchema,
  shouldAutomate,
} from "../src/shared/autonomy";

test("importance boundaries select only the configured decisions", () => {
  for (const ratio of [10, 30, 70]) {
    assert.equal(shouldAutomate(ratio, 100 - ratio), true);
    assert.equal(shouldAutomate(ratio, 101 - ratio), false);
  }
  assert.equal(shouldAutomate(0, 100), true);
  assert.equal(shouldAutomate(100, 1), false);
  assert.equal(askRatioSchema.safeParse(20).success, false);
});

test("automation uses a real recommended option and leaves legacy manual questions valid", () => {
  const q = {
    importance: 60,
    recommended: "B",
    reason: "Fits scope",
    options: [{ label: "A" }, { label: "B" }],
  };
  assert.equal(automaticChoice(30, q), "B");
  assert.equal(automaticChoice(70, q), undefined);
  assert.equal(automaticChoice(100, {}), undefined);
  assert.throws(() => automaticChoice(0, { ...q, recommended: "Invented" }));
  assert.throws(() => automaticChoice(0, { ...q, reason: " " }));
});
