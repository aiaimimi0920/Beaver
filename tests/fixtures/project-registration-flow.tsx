import { createRoot } from "react-dom/client";
import { ProjectRegistrationDialog } from "../../src/ui/ProjectRegistrationDialog";
import { NotificationProvider } from "../../src/ui/Notifications";

const root = createRoot(document.getElementById("root")!);
const requests: { method: string; input: unknown }[] = [];
let closed = 0;
let failure = false;
let directory: string | null = null;
let release: (() => void) | undefined;
let hold = false;
let errors = 0;
window.beaver = {
  async call(method, input) {
    if (method === "project.storage.status")
      return {
        path: "C:/old",
        state: "offline",
        message: "项目目录不可用，请重新关联项目位置。",
      };
    requests.push({ method, input });
    if (method === "chooseDirectory") return directory;
    if (hold)
      await new Promise<void>((resolve) => {
        release = resolve;
      });
    if (failure) throw new Error("stale registration");
    return { draining: true };
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
  const element = [...document.querySelectorAll("button")].find(
    (node) => node.textContent === text,
  );
  if (!element) throw new Error(`Missing button: ${text}`);
  return element;
}
async function click(text: string) {
  button(text).click();
  await tick();
}
async function mount(key: string) {
  root.render(
    <NotificationProvider key={key}>
      <ProjectRegistrationDialog
        project={{
          id: "p1",
          name: "Fixture",
          path: "C:/old",
          createdAt: "now",
        }}
        close={() => {
          closed += 1;
        }}
        run={async (work) => {
          try {
            await work();
          } catch {
            errors += 1;
          }
        }}
      />
    </NotificationProvider>,
  );
  await tick();
}
async function verify() {
  await mount("reassociate");
  check(
    document.body.textContent?.includes("项目目录不可用"),
    "Offline diagnosis must be displayed",
  );
  check(button("重新关联").disabled, "Empty directory must be disabled");
  await click("选择目录");
  check(button("重新关联").disabled, "Cancelled picker must not submit");
  directory = "C:/moved";
  await click("选择目录");
  failure = true;
  await click("重新关联");
  check(
    closed === 0 && errors === 1,
    "Failure must retain the dialog for retry",
  );
  check(
    JSON.stringify(requests.at(-1)) ===
      JSON.stringify({
        method: "project.reassociate",
        input: { id: "p1", expectedPath: "C:/old", path: "C:/moved" },
      }),
    "Reassociation must carry the captured registration path",
  );
  failure = false;
  await click("重新关联");
  check(closed === 1, "Success must close");

  await mount("unregister");
  const before = requests.length;
  await click("从列表移除");
  await click("取消移除");
  check(requests.length === before, "Removal requires explicit confirmation");
  await click("从列表移除");
  hold = true;
  button("确认移除").click();
  button("确认移除").click();
  await tick();
  check(requests.length === before + 1, "Double click must submit once");
  check(button("关闭").disabled, "Pending mutation must disable close");
  document.dispatchEvent(
    new KeyboardEvent("keydown", { key: "Escape", bubbles: true }),
  );
  check(closed === 1, "Escape must not dismiss pending mutation");
  check(
    JSON.stringify(requests.at(-1)) ===
      JSON.stringify({
        method: "project.unregister",
        input: { id: "p1", expectedPath: "C:/old" },
      }),
    "Removal must carry original path CAS",
  );
  release?.();
  await tick();
  check(closed === 2, "Removal success must close");
  check(
    document.body.textContent?.includes("已开始的工作仍在收尾"),
    "Draining notice required",
  );
  return "PASS: picker cancellation, CAS, failure retry, confirmation, duplicate guard, pending close, draining";
}
Object.assign(window, { registrationResult: verify() });
