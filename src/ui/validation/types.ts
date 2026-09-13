import type { Flow, Region, SourceReference } from "./flow";

export interface Evidence {
  id: string;
  file: string;
  sha256: string;
  kind: string;
  point: string;
  start: number;
  end: number;
  references: SourceReference[];
  state: unknown;
}
export interface CodeReport {
  gutVersion: string;
  directories: string[];
  passed: number;
  failed: number;
  skipped: number;
  exitCode: number | null;
  reportSha256: string;
  outputSha256: string;
  cases: {
    name: string;
    file: string;
    status: string;
    assertions: number;
    message: string;
  }[];
}
export interface ValidationRun {
  id: string;
  projectId: string;
  taskId: string | null;
  releaseId: string | null;
  kind: string;
  managed: boolean;
  createdAt: string;
  finishedAt: string | null;
  snapshotId: string;
  flow: Flow | null;
  baselineId: string | null;
  runnerVersion: string;
  engineVersion: string;
  status: string;
  phase: string;
  completedSteps: number;
  verdict: string;
  error: string | null;
  log: string;
  evidence: Evidence[];
  evidenceCount?: number;
  code: CodeReport | null;
  judgments: unknown[];
  confirmations: { source: string; evidenceIds: string[]; at: string }[];
  sourcePaths?: string[];
  integrityError?: string;
  baselineError?: string;
  baseline?: {
    record: {
      id: string;
      confirmedAt: string;
      source: string;
      snapshotId: string;
    };
    run: ValidationRun;
    integrityError: string | null;
  };
}
export interface ValidationSettings {
  revision: number;
  visualRequired: boolean;
  ffmpeg: string;
}
export interface ReleaseCheck {
  id: string;
  projectId: string;
  snapshotId: string;
  scopeId: string;
  preset: string;
  visualRequired: boolean;
  policyVersion: string;
  runIds: string[];
  createdAt: string;
  ready: boolean;
  integrityError?: string;
  items?: {
    runId: string;
    kind: string;
    flowId: string | null;
    status: string;
    phase: string;
    verdict: string;
    passed: boolean;
    error?: string | null;
    integrityError?: string;
  }[];
  missing: string[];
  excluded: unknown[];
}
export interface Feedback {
  id: string;
  taskId: string;
  runId: string;
  text: string;
  status: string;
  rerunId?: string;
  error?: string;
  reason?: string | null;
  taskStatus?: string;
  findings?: unknown;
}
export interface ValidationData {
  flows: Flow[];
  runs: ValidationRun[];
  settings: ValidationSettings;
  coverage: {
    taskId: string;
    status: string;
    reason?: string;
    runIds?: string[];
  }[];
  feedback: Feedback[];
  releases: ReleaseCheck[];
  repairDecisions: {
    runId: string;
    status: string;
    reason: string;
    taskId?: string;
  }[];
}
export interface Selection {
  evidenceId?: string;
  range?: [number, number];
  region?: Region;
}
export type Mutate = <T = unknown>(
  method: string,
  input?: Record<string, unknown>,
) => Promise<T>;
export const running = (run: ValidationRun) =>
  ["queued", "preparing", "running"].includes(run.status);
export const labels: Record<string, string> = {
  queued: "排队中",
  preparing: "准备中",
  importing: "导入资源",
  testing: "运行 GUT",
  capturing: "运行与采集",
  encoding: "编码视频",
  comparing: "对比画面",
  running: "执行中",
  completed: "已完成",
  failed: "执行失败",
  cancelled: "已停止",
  interrupted: "运行中断",
  pending: "待判断",
  passed: "代码通过",
  autoPassed: "自动对比通过",
  userPassed: "用户已认可",
  needsReview: "需要查看",
  noBaseline: "需要首次认可",
  missingBaseline: "缺少认可基准",
  invalid: "流程失效",
  unavailable: "尚不可用",
  stale: "结果已失效",
  missing: "缺少可运行流程",
  retired: "已明确退役",
};
export const label = (value: string) => labels[value] ?? value;
export const verdictLabel = (run: Pick<ValidationRun, "kind" | "verdict">) =>
  run.kind === "code" && run.verdict === "autoPassed"
    ? "代码验收通过"
    : label(run.verdict);
export function tone(run: ValidationRun): string {
  if (run.integrityError || run.status === "failed") return "bad";
  if (running(run)) return "busy";
  return run.status === "completed" &&
    ["autoPassed", "userPassed"].includes(run.verdict)
    ? "good"
    : "attention";
}
export const shortId = (id: string) => id.slice(0, 10);
export const time = (value: string) => new Date(value).toLocaleString();
