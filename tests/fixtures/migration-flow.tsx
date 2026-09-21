import { createRoot } from "react-dom/client";
import { MigrationDialog } from "../../src/ui/MigrationDialog";

const root = createRoot(document.getElementById("root")!);
const requests: { method: string; input: unknown }[] = [];
let directory: string | null = null;
let failure = "";
let hold = false;
let release: (() => void) | undefined;
let closed = 0;
window.beaver = {
  async call(method, input) {
    requests.push({ method, input });
    if (method === "chooseDirectory") return directory;
    if (hold)
      await new Promise<void>((resolve) => {
        release = resolve;
      });
    if (failure === method) throw new Error(`fixture failure: ${method}`);
    if (method === "migration.inspect")
      return {
        projects: { p1: {} },
        unresolved: { entities: 2, events: 1, calls: 0 },
      };
    if (method === "migration.prepareProjects")
      return { restoredData: "C:/new/data", activationBlockers: [] };
    if (method === "migration.activate")
      return {
        ready_to_activate: true,
        data_directory: "C:/new/data",
        default_data_directory_changed: false,
      };
    throw new Error(`Unexpected ${method}`);
  },
  subscribe: () => () => {},
  onWindowState: () => () => {},
  windowControl: async () => ({ maximized: false }),
};
const tick = () => new Promise((resolve) => setTimeout(resolve, 40));
function check(value: unknown, message: string) {
  if (!value) throw new Error(message);
}
function button(text: string) {
  const node = [...document.querySelectorAll("button")].find(
    (node) => node.textContent === text,
  );
  if (!node) throw new Error(`Missing ${text}`);
  return node;
}
async function click(text: string) {
  button(text).click();
  await tick();
}
async function input(label: string, value: string) {
  const node = document.querySelector<HTMLInputElement | HTMLSelectElement>(
    `[aria-label="${label}"]`,
  )!;
  const prototype =
    node instanceof HTMLSelectElement
      ? HTMLSelectElement.prototype
      : HTMLInputElement.prototype;
  Object.getOwnPropertyDescriptor(prototype, "value")!.set!.call(node, value);
  node.dispatchEvent(
    new Event(node instanceof HTMLSelectElement ? "change" : "input", {
      bubbles: true,
    }),
  );
  await tick();
}
async function mount(key: string) {
  root.render(
    <MigrationDialog
      key={key}
      close={() => {
        closed += 1;
      }}
    />,
  );
  await tick();
}
function last(method: string, expected: unknown) {
  check(
    JSON.stringify(requests.at(-1)) ===
      JSON.stringify({ method, input: expected }),
    `Wrong request for ${method}`,
  );
}
async function verify() {
  await mount("new");
  check(
    button("检查归档").disabled && button("准备副本").disabled,
    "Initial stages gated",
  );
  await click("选择归档");
  check(
    button("检查归档").disabled,
    "Cancelled picker must keep archive empty",
  );
  directory = "C:/archive";
  await click("选择归档");
  await click("检查归档");
  check(
    document.body.textContent?.includes("未归属记录 3 条"),
    "Show retained history warning",
  );
  await input("副本目录", "C:/failed");
  await input("归档目录", "C:/archive2");
  check(button("准备副本").disabled, "Archive change invalidates inspection");
  await click("检查归档");
  failure = "migration.prepareProjects";
  await click("准备副本");
  check(
    button("准备副本").disabled && closed === 0,
    "Failed target must require new directory",
  );
  check(
    document.body.textContent?.includes("部分副本保留在 C:/failed"),
    "Partial copy location retained",
  );
  failure = "";
  await input("副本目录", "C:/new");
  hold = true;
  const before = requests.length;
  button("准备副本").click();
  button("准备副本").click();
  await tick();
  check(requests.length === before + 1, "Duplicate preparation suppressed");
  last("migration.prepareProjects", {
    backup: "C:/archive2",
    destination: "C:/new",
  });
  document.dispatchEvent(
    new KeyboardEvent("keydown", { key: "Escape", bubbles: true }),
  );
  document.querySelector<HTMLButtonElement>('[aria-label="关闭弹窗"]')!.click();
  check(
    closed === 0 && button("关闭").disabled,
    "Pending operation cannot dismiss",
  );
  hold = false;
  release?.();
  await tick();
  check(
    (document.querySelector('[aria-label="归档目录"]') as HTMLInputElement)
      .disabled,
    "Prepared source frozen",
  );
  const prepared = requests.length;
  await click("启用副本");
  await click("取消启用");
  check(requests.length === prepared, "Confirmation must not activate");
  await input("工具路径 JSON 文件", "C:/tools.json");
  await click("启用副本");
  failure = "migration.activate";
  await click("确认启用副本");
  last("migration.activate", {
    backup: "C:/archive2",
    prepared: "C:/new",
    toolPaths: "C:/tools.json",
  });
  check(
    closed === 0 && document.body.textContent?.includes("fixture failure"),
    "Activation error visible and retryable",
  );
  failure = "";
  await click("确认启用副本");
  check(
    document.body.textContent?.includes("副本已启用，当前环境未切换"),
    "Success must explain no environment switch",
  );
  check(
    document.body.textContent?.includes("BEAVER_DATA_DIR"),
    "Provide manual startup location",
  );

  await mount("resume");
  await input("归档目录", "C:/archive");
  await click("检查归档");
  await input("副本操作", "resume");
  await input("副本目录", "C:/prepared");
  const resumeBefore = requests.length;
  await click("启用副本");
  await click("确认启用副本");
  check(
    requests.length === resumeBefore + 1,
    "Resume must not restore over existing copy",
  );
  last("migration.activate", { backup: "C:/archive", prepared: "C:/prepared" });
  return "PASS: inspection invalidation, picker cancellation, new target recovery, pending guards, confirmation, activation retry, explicit switch guidance, resume";
}
Object.assign(window, { migrationResult: verify() });
