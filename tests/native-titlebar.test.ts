import assert from "node:assert/strict";
import test from "node:test";
import { nativeTitlebarCommand } from "../src/ui/native-titlebar";

test("native titlebar drags only primary clicks in non-interactive regions", () => {
  assert.equal(nativeTitlebarCommand(0, 1, true, false), "startDragging");
  assert.equal(nativeTitlebarCommand(0, 2, true, false), "toggleMaximize");
  for (const detail of [1, 2]) {
    assert.equal(nativeTitlebarCommand(1, detail, true, false), null);
    assert.equal(nativeTitlebarCommand(2, detail, true, false), null);
    assert.equal(nativeTitlebarCommand(0, detail, false, false), null);
    assert.equal(nativeTitlebarCommand(0, detail, true, true), null);
  }
});
