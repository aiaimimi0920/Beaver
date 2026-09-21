export interface WorkDefinition {
  title: string;
  goal: string;
  acceptance: string;
}

export type AttemptStatus = "running" | "completed" | "failed" | "interrupted";
export type SubtaskStatus = AttemptStatus | "pending" | "cancelled" | "stale";

export interface AssetSubtask {
  id: string;
  stageId: string;
  definition: WorkDefinition;
  status: SubtaskStatus;
  lastAttemptId: string | null;
  note: string;
}

export interface WorkAttempt {
  id: string;
  subtaskId: string;
  stageId: string;
  definition: WorkDefinition;
  status: AttemptStatus;
  threadId: string;
  turnId: string;
  sessionId: string | null;
  checkpoint: string | null;
  endCheckpoint: string | null;
  assetRevision: number;
  inputCandidates: string[];
  inputFiles?: WorkInputFile[] | null;
  inputs: Record<string, unknown>;
  outputs: Record<string, unknown>;
  tools: Record<string, unknown>[];
  summary: string;
  recoveryNote: string;
  startedAt: string;
  endedAt: string | null;
}

export interface WorkInputFile {
  path: string;
  role: "source" | "dependency";
  sha256: string;
}

export interface AssetWork {
  subtasks: AssetSubtask[];
  attempts: WorkAttempt[];
}
