import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import vm from "node:vm";

const html = readFileSync("src/direct/index.html", "utf8");
const renderer = readFileSync("src/direct/renderer.js", "utf8");

function fixture(call: (method: string, args: unknown) => Promise<unknown>) {
  const elements = new Map<string, any>();
  for (const match of html.matchAll(/id="([^"]+)"/g)) {
    elements.set(match[1]!, {
      value: "",
      textContent: "",
      disabled: false,
      children: [] as unknown[],
      replaceChildren() {
        this.children = [];
      },
      append(child: unknown) {
        this.children.push(child);
      },
    });
  }
  const buttons = [...html.matchAll(/<button\s+id="([^"]+)"/g)].map((match) =>
    elements.get(match[1]!),
  );
  elements.get("role").value = "user";
  vm.runInNewContext(
    readFileSync("src/direct/npr-upgrade.js", "utf8") + "\n" + renderer,
    {
      document: {
        createElement: () => ({ value: "", textContent: "" }),
        getElementById: (id: string) => {
          assert.ok(elements.has(id), `Missing DOM element ${id}`);
          return elements.get(id);
        },
        querySelectorAll: (selector: string) => {
          assert.equal(selector, "button");
          return buttons;
        },
      },
      window: { beaver: { call } },
    },
  );
  return elements;
}

test("interrupt sends only selected task ID and suppresses busy duplicate clicks", async () => {
  const calls: unknown[] = [];
  let finish!: (value: unknown) => void;
  const pending = new Promise((resolve) => (finish = resolve));
  const elements = fixture(async (method, args) => {
    calls.push([method, args]);
    if (method === "task.interrupt") return pending;
    assert.equal(method, "state");
    return {
      tasks: [
        { id: "selected-task", title: "Selected", status: "interrupted" },
        { id: "other-task", title: "Other", status: "queued" },
      ],
    };
  });
  elements.get("task").value = "selected-task";
  const first = elements.get("interrupt").onclick();
  assert.equal(elements.get("interrupt").disabled, true);
  elements.get("task").value = "other-task";
  await elements.get("interrupt").onclick();
  assert.deepEqual(JSON.parse(JSON.stringify(calls)), [
    ["task.interrupt", { id: "selected-task" }],
  ]);
  finish(null);
  await first;
  assert.equal(elements.get("interrupt").disabled, false);
  assert.match(elements.get("result").textContent, /interrupted/);
  assert.equal(calls.length, 2);
  assert.equal(
    elements.get("task").children[0].textContent,
    "Selected · interrupted",
  );
  assert.equal(elements.get("task").value, "other-task");
});

test("interrupt rejects empty selection without calling Core", async () => {
  let calls = 0;
  const elements = fixture(async () => {
    calls++;
  });
  await elements.get("interrupt").onclick();
  assert.equal(calls, 0);
  assert.match(elements.get("result").textContent, /先选择要中止的任务/);
  assert.equal(elements.get("interrupt").disabled, false);
});

test("interrupt markup has one button and no stray separator text", () => {
  assert.equal([...html.matchAll(/id="interrupt"/g)].length, 1);
  const section = html.match(/<button id="history">[\s\S]*?<\/section>/)?.[0];
  assert.ok(section);
  const text = section.replace(/<[^>]*>/g, "").replace(/\s/g, "");
  assert.equal(text, "读取任务历史中止选中任务撤销未执行补充步骤");
});

test("resume never silently submits the goal textbox as a new instruction", async () => {
  const calls: unknown[] = [];
  const elements = fixture(async (method, args) => {
    calls.push([method, args]);
    return null;
  });
  elements.get("task").value = "parent";
  elements.get("goal").value = "Old goal that must not become a supplement";
  await elements.get("continue").onclick();
  assert.deepEqual(JSON.parse(JSON.stringify(calls)), [
    ["task.continue", { id: "parent", text: "" }],
  ]);
});

test("withdrawal uses selected child and explicit reason through Core", async () => {
  const calls: unknown[] = [];
  const elements = fixture(async (method, args) => {
    calls.push([method, args]);
    if (method === "state")
      return {
        projects: [],
        tasks: [
          {
            id: "supplement",
            parentTaskId: "parent",
            title: "Supplement",
            status: "failed",
          },
        ],
      };
    assert.equal(method, "task.withdrawSupplement");
    return { parentId: "parent", reason: "Redundant" };
  });
  await elements.get("refresh").onclick();
  elements.get("task").value = "supplement";
  elements.get("withdraw-reason").value = "Redundant";
  await elements.get("withdraw").onclick();
  assert.deepEqual(JSON.parse(JSON.stringify(calls[1])), [
    "task.withdrawSupplement",
    { id: "supplement", parentId: "parent", reason: "Redundant" },
  ]);
  assert.equal(calls.length, 3);
});
