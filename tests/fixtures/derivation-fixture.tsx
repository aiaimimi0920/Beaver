import { createRoot } from "react-dom/client";
import { MigrationDialog } from "../../src/ui/MigrationDialog";
import "../../src/ui/style.css";

const root = createRoot(document.getElementById("root")!);
export const state = {
  requests: [] as { method: string; input: unknown }[],
  failure: "",
  hold: false,
  release: undefined as (() => void) | undefined,
  directory: null as string | null,
  closed: 0,
  opened: "",
  activated: false,
  runtimeReady: false,
};
const prepared = {
  request: {
    source: "C:/source",
    sourceProjectId: "original",
    targetProjectId: "derived",
    requestId: "stable-request",
  },
  entities: 4,
  calls: 1,
};
window.beaver = {
  async call(method, input) {
    state.requests.push({ method, input });
    if (method === "chooseDirectory") return state.directory;
    if (state.hold)
      await new Promise<void>((resolve) => {
        state.release = resolve;
      });
    if (method === state.failure) throw new Error(`fixture failure: ${method}`);
    if (
      [
        "migration.inspectDerivationSource",
        "migration.prepareDerivation",
        "migration.inspectDerivation",
      ].includes(method)
    )
      return prepared;
    if (
      ["migration.assembleDerivation", "migration.inspectAssembly"].includes(
        method,
      )
    )
      return {
        activated: state.activated,
        assembly: {
          projectId: "derived",
          binding: "C:/assembly/project",
          preparationSha256: "verified-digest",
          tasksInterrupted: 2,
          sessionPathsRewritten: 1,
        },
      };
    if (method === "migration.activateAssembly") {
      state.activated = true;
      return {};
    }
    if (method === "migration.registerAssembly")
      return {
        projectId: "derived",
        registrationCommitted: true,
        runtimeReady: state.runtimeReady,
        runtimeError: state.runtimeReady ? null : "fixture recovery failure",
      };
    throw new Error(`Unexpected ${method}`);
  },
  subscribe: () => () => {},
  onWindowState: () => () => {},
  windowControl: async () => ({ maximized: false }),
};
export const tick = () => new Promise((resolve) => setTimeout(resolve, 35));
export function check(value: unknown, message: string) {
  if (!value) throw new Error(message);
}
export function button(text: string) {
  const node = [...document.querySelectorAll("button")].find(
    (node) => node.textContent === text,
  );
  if (!node) throw new Error(`Missing ${text}`);
  return node;
}
export async function click(text: string) {
  button(text).click();
  await tick();
}
export async function input(label: string, value: string) {
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
export async function mount(key: string) {
  root.render(
    <MigrationDialog
      key={key}
      close={() => {
        state.closed++;
      }}
      openProject={async (id) => {
        state.opened = id;
      }}
    />,
  );
  await tick();
}
export function last(method: string, input: unknown) {
  check(
    JSON.stringify(state.requests.at(-1)) === JSON.stringify({ method, input }),
    `Wrong request for ${method}: ${JSON.stringify(state.requests.at(-1))}`,
  );
}
