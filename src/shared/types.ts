import type { ProjectBlueprint } from "./project-blueprint";

export type TaskStatus =
  | "queued"
  | "running"
  | "waitingChildren"
  | "awaitingInput"
  | "interrupted"
  | "failed"
  | "conflict"
  | "completed"
  | "rolledBack";
export type Capability =
  "code" | "review" | "image" | "speech" | "music" | "translation";
export type Snapshot = Record<string, string>;
export interface GameBrief {
  genres: string[];
  theme: string;
  style: string;
  scope: string;
}
export interface Project {
  npr?: {
    godot?: string;
    status: "installing" | "ready" | "failed";
    error?: string;
    runtime?: { godot: string; engineVersion: string };
  };
  delivery?: Delivery;
  id: string;
  name: string;
  path: string;
  createdAt: string;
  design?: GameBrief;
  blueprint?: ProjectBlueprint;
  blueprintRevision?: number;
}
export interface Reference {
  path: string;
  note: string;
  region?: { x: number; y: number; w: number; h: number };
}
export interface Change {
  path: string;
  before?: string;
  after?: string;
}
export interface Task {
  assetTask?: boolean;
  delivery?: Delivery;
  askRatio?: import("./autonomy").AskRatio | null;
  effectiveAskRatio?: import("./autonomy").AskRatio;
  projectContext?: import("./task-brief").TaskProjectContext;
  parentTaskId?: string;
  relation?: "child" | "followup";
  decompose?: boolean;
  autoAccept?: boolean;
  subtaskIds?: string[];
  dependsOn?: string[];
  plan?: import("./task-plan").TaskPlan;
  workspacePrepared?: boolean;
  planPaused?: boolean;
  approvalSource?: "automatic" | "user";
  direction?: import("./task-board").TaskDirection;
  id: string;
  projectId: string;
  title: string;
  prompt: string;
  design?: GameBrief;
  stopConditions: string;
  status: TaskStatus;
  createdAt: string;
  updatedAt: string;
  workspace: string;
  references: Reference[];
  baseline: Snapshot;
  changes: Change[];
  conflicts: string[];
  threadId?: string;
  turnId?: string;
  sessionHistory?: {
    threadId: string;
    endedAt: string;
    reason: "freshContext";
  }[];
  restartContext?: {
    previousReport: string;
    previousError: string;
    changedPaths: string[];
  };
  error?: string;
  report?: string;
  clarifications?: import("./clarifications").Clarification[];
  accepted?: boolean;
  retainedFiles?: string[];
  maxMinutes: number;
  capability: "code" | "review";
  feature?: {
    id: string;
    version: string;
    snapshot: Snapshot;
    previous?: FeatureAdoption;
  };
}
export interface Delivery {
  status: "verified" | "failed";
  scope: "export-snapshot";
  checkedAt: string;
  path?: string;
  message?: string;
  runtimeVerified: false;
}
export interface FeatureAdoption {
  id: string;
  version: string;
  snapshot: Snapshot;
  taskId: string;
}
export interface TaskEvent {
  time: string;
  kind: string;
  text: string;
}
export interface Endpoint {
  baseUrl: string;
  model: string;
  route: string;
  hasKey?: boolean;
}
export interface Settings {
  askRatio?: import("./autonomy").AskRatio;
  mode: "local" | "cloud";
  local: Record<Capability, Endpoint>;
  cloud: Endpoint;
  cloudModels: Record<Capability, string>;
  tools: { codex: string; godot: string; blender: string; node: string };
  mcp: { godot: boolean; blender: boolean };
  maxParallel: number;
}
export interface Asset {
  path: string;
  bytes: number;
  kind: "image" | "audio" | "model" | "scene" | "text" | "other";
  modifiedAt: string;
}
export interface ToolStatus {
  name: keyof Settings["tools"];
  path: string;
  available: boolean;
  version: string;
}
export interface Feature {
  id: string;
  version: string;
  name: string;
  description: string;
  category?: string;
  kind?: "module" | "reference";
}
export type ExecutorLocation = "local" | "cloud";
export interface TaskExecutor {
  readonly location: ExecutorLocation;
  start(task: Task): Promise<void>;
  interrupt(taskId: string): Promise<void>;
  steer(taskId: string, text: string): Promise<void>;
  shutdown(): Promise<void>;
}
export const capabilities: Capability[] = [
  "code",
  "review",
  "image",
  "speech",
  "music",
  "translation",
];
export function defaultSettings(): Settings {
  const local = Object.fromEntries(
    capabilities.map((c) => [
      c,
      {
        baseUrl: "",
        model: "",
        route:
          c === "image"
            ? "/images/generations"
            : c === "speech"
              ? "/audio/speech"
              : "",
      },
    ]),
  ) as Record<Capability, Endpoint>;
  return {
    mode: "local",
    local,
    cloud: { baseUrl: "", model: "", route: "" },
    cloudModels: {
      code: "",
      review: "",
      image: "",
      speech: "",
      music: "",
      translation: "",
    },
    tools: { codex: "", godot: "", blender: "", node: "" },
    mcp: { godot: false, blender: false },
    maxParallel: 3,
    askRatio: 100,
  };
}
