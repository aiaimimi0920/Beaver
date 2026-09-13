export type Category = "roaming" | "feature" | "ui" | "task";
export type Region = [number, number, number, number];
export type SourceReference = {
  path: string;
  node: string;
  symbol: string;
  source: "author" | "runtime" | "ai";
};
export type Step = { id: string; references: SourceReference[] } & (
  | { kind: "wait"; frames: number }
  | { kind: "action"; name: string; pressed: boolean }
  | { kind: "key"; code: number; pressed: boolean }
  | { kind: "click"; node: string }
  | {
      kind: "waitFor";
      node: string;
      property: string;
      equals: unknown;
      timeout: number;
    }
  | { kind: "capture" }
);
export interface Definition {
  key: string;
  name: string;
  category: Category;
  purpose: string;
  entry: string;
  taskIds: string[];
  config: {
    width: number;
    height: number;
    fps: number;
    seed: number;
    locale: string;
    saves: string[];
    threshold: number;
    masks: Region[];
  };
  steps: Step[];
  video: boolean;
  references: SourceReference[];
  retiredReason: string;
  roaming?: {
    actions: string[];
    rounds: number;
    minFrames: number;
    maxFrames: number;
    setup: Step[];
  } | null;
}
export interface Flow {
  id: string;
  projectId: string;
  revision: number;
  definition: Definition;
  updatedAt: string;
  reason: string;
}
export const categories: Record<Category, string> = {
  roaming: "漫游画面",
  feature: "功能画面",
  ui: "UI 画面",
  task: "任务画面",
};
export function defaultStep(
  kind: Step["kind"],
  id: string = crypto.randomUUID(),
): Step {
  const common = { id, references: [] };
  switch (kind) {
    case "wait":
      return { ...common, kind, frames: 30 };
    case "action":
      return { ...common, kind, name: "ui_accept", pressed: true };
    case "key":
      return { ...common, kind, code: 32, pressed: true };
    case "click":
      return { ...common, kind, node: "" };
    case "waitFor":
      return {
        ...common,
        kind,
        node: "",
        property: "visible",
        equals: true,
        timeout: 300,
      };
    case "capture":
      return { ...common, kind };
  }
}
export function newDefinition(taskId = ""): Definition {
  return {
    key: `flow-${crypto.randomUUID().slice(0, 8)}`,
    name: "新画面流程",
    category: taskId ? "task" : "feature",
    purpose: "",
    entry: "",
    taskIds: taskId ? [taskId] : [],
    config: {
      width: 960,
      height: 540,
      fps: 30,
      seed: 1,
      locale: "en",
      saves: [],
      threshold: 0.98,
      masks: [],
    },
    steps: [defaultStep("wait", "ready"), defaultStep("capture", "result")],
    video: false,
    references: [],
    retiredReason: "",
  };
}
