export interface FrameworkCommand {
  executable: string;
  sha256: string;
  args: string[];
  timeoutSeconds: number;
  files: Record<string, string>;
}

export interface FrameworkPlugin {
  id: string;
  host: "godot" | "blender" | "codex";
  version: string;
  hostVersion: string;
  probe: FrameworkCommand;
  install?: FrameworkCommand | null;
  enable?: FrameworkCommand | null;
  reload?: FrameworkCommand | null;
}

export interface FrameworkConfiguration {
  revision: number;
  plugins: FrameworkPlugin[];
  rules: { id: string; version: number; checker: FrameworkCommand }[];
  semanticRequired: boolean;
}

export type FrameworkJob =
  | { kind: "callback"; request: unknown }
  | { kind: "plugin"; plugin: string; action: string }
  | { kind: "check"; candidateId: string }
  | { kind: "inputExport"; attemptId: string }
  | { kind: "dependencies"; command: FrameworkCommand; paths: string[] };

export interface FrameworkOperation {
  id: string;
  requestId: string;
  taskId: string;
  job: FrameworkJob;
  source: string;
  status: string;
  createdAt: string;
  endedAt?: string | null;
  result?: unknown;
  error?: string | null;
}

export interface FrameworkState {
  configuration: FrameworkConfiguration;
  operations: FrameworkOperation[];
  traces: unknown[];
  checks: unknown[];
  judgments: unknown[];
  observations: unknown[];
  recovery: unknown[];
  currentRecovery: unknown;
  coverage: Record<string, string>;
}

export const operationTerminal = (status: string) =>
  ["succeeded", "failed", "cancelled", "interrupted", "stale"].includes(status);

export type FrameworkAction = (request: unknown) => Promise<void>;
